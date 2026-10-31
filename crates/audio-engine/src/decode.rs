use crate::{analysis::WaveformBuilder, annotations::CueState};
use std::{fs::File, path::Path, sync::Arc};
use symphonia::core::{
    audio::SampleBuffer, codecs::DecoderOptions, errors::Error, formats::FormatOptions,
    io::MediaSourceStream, meta::MetadataOptions, probe::Hint,
};

/// Decoded stereo PCM per track: about 90 minutes at 48 kHz, never an unbounded file read.
pub const MAX_CLIP_BYTES: usize = 2 * 1024 * 1024 * 1024;
/// Decoding stopped because the audio would not fit in `max_bytes`.
pub const TOO_LARGE: &str = "El audio decodificado no cabe en la memoria disponible para audio.";
/// Decoding stopped because `cancelled` said so.
pub const CANCELLED: &str = "Carga cancelada";
#[derive(Debug)]
pub struct Clip {
    pub id: u64,
    pub cues: Arc<CueState>,
    pub samples: Vec<[f32; 2]>,
    pub sample_rate: u32,
    /// `[peak, low, mid, high]` per bin at `analysis::WAVEFORM_RATE`.
    pub waveform: Vec<[u8; 4]>,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub music: crate::music::MusicAnalysis,
}
impl Clip {
    pub fn duration(&self) -> f64 {
        self.samples.len() as f64 / self.sample_rate as f64
    }
}
pub fn decode(path: &Path, id: u64) -> Result<Clip, String> {
    decode_with_music(path, id, None)
}
pub fn decode_with_music(
    path: &Path,
    id: u64,
    cached: Option<crate::music::MusicAnalysis>,
) -> Result<Clip, String> {
    decode_with_limit(path, id, cached, MAX_CLIP_BYTES)
}
/// Encoder delay is deliberately kept: measured against stored grids, rekordbox's timeline for
/// MP3 includes it, so trimming would move every grid and cue about 25 ms away from theirs.
pub fn decode_with_limit(
    path: &Path,
    id: u64,
    cached: Option<crate::music::MusicAnalysis>,
    max_bytes: usize,
) -> Result<Clip, String> {
    decode_bounded(path, id, cached, max_bytes, &|| false)
}
/// `cancelled` is checked per packet, so a newer load never waits for a long one to finish.
pub fn decode_bounded(
    path: &Path,
    id: u64,
    cached: Option<crate::music::MusicAnalysis>,
    max_bytes: usize,
    cancelled: &dyn Fn() -> bool,
) -> Result<Clip, String> {
    let file = File::open(path).map_err(|e| format!("No se pudo abrir el archivo: {e}"))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let mut probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("Formato no compatible: {e}"))?;
    let mut title = None;
    let mut artist = None;
    let mut read_tags = |rev: &symphonia::core::meta::MetadataRevision| {
        for tag in rev.tags() {
            use symphonia::core::meta::StandardTagKey;
            match tag.std_key {
                Some(StandardTagKey::TrackTitle) => title = Some(tag.value.to_string()),
                Some(StandardTagKey::Artist) => artist = Some(tag.value.to_string()),
                _ => {}
            }
        }
    };
    if let Some(rev) = probed.metadata.get().as_ref().and_then(|m| m.current()) {
        read_tags(rev);
    }
    let mut format = probed.format;
    if let Some(rev) = format.metadata().current() {
        read_tags(rev);
    }
    let track = format
        .default_track()
        .ok_or("El archivo no contiene audio")?;
    let track_id = track.id;
    let params = track.codec_params.clone();
    let mut decoder = symphonia::default::get_codecs()
        .make(&params, &DecoderOptions::default())
        .map_err(|e| format!("Códec no compatible: {e}"))?;
    let frame_size = std::mem::size_of::<[f32; 2]>();
    let max_frames = max_bytes / frame_size;
    let mut samples: Vec<[f32; 2]> = Vec::new();
    // An allocation failure here is an error message; pushing into a Vec would abort the app.
    let memory_error = |_| String::from(TOO_LARGE);
    if let Some(expected) = params.n_frames {
        // Doubling would need twice the memory of a long track just to copy it.
        samples
            .try_reserve_exact((expected as usize).min(max_frames))
            .map_err(memory_error)?;
    }
    let mut sample_rate = params.sample_rate.unwrap_or(44100);
    let mut analysis: Option<WaveformBuilder> = None;
    loop {
        if cancelled() {
            return Err(CANCELLED.into());
        }
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(format!("Error de lectura: {e}")),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = decoder
            .decode(&packet)
            .map_err(|e| format!("Error al decodificar: {e}"))?;
        sample_rate = decoded.spec().rate;
        let analysis = analysis.get_or_insert_with(|| WaveformBuilder::new(sample_rate));
        let channels = decoded.spec().channels.count();
        if channels == 0 || channels > 2 {
            return Err("Esta versión admite archivos mono o estéreo".into());
        }
        let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
        buffer.copy_interleaved_ref(decoded);
        let frames = buffer.samples().len() / channels;
        if samples.len() + frames > max_frames {
            return Err(TOO_LARGE.into());
        }
        if samples.capacity() - samples.len() < frames {
            let room = frames
                .max(samples.len() / 8)
                .min(max_frames - samples.len());
            samples.try_reserve_exact(room).map_err(memory_error)?;
        }
        for frame in buffer.samples().chunks_exact(channels) {
            let clean = |x: f32| {
                if x.is_finite() {
                    x.clamp(-4.0, 4.0)
                } else {
                    0.0
                }
            };
            let frame = [clean(frame[0]), clean(frame[channels - 1])];
            analysis.push(frame);
            samples.push(frame);
        }
    }
    if samples.is_empty() {
        return Err("El archivo no contiene muestras decodificables".into());
    }
    samples.shrink_to_fit();
    if cancelled() {
        return Err(CANCELLED.into());
    }
    let waveform = analysis.map(WaveformBuilder::finish).unwrap_or_default();
    let music = cached
        .filter(|m| m.valid())
        .unwrap_or_else(|| crate::music::analyze(&samples, sample_rate));
    Ok(Clip {
        id,
        cues: Arc::new(CueState::default()),
        samples,
        sample_rate,
        waveform,
        title,
        artist,
        music,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decodes_mono_and_stereo_wav_and_rejects_garbage() {
        for channels in [1, 2] {
            let path = std::env::temp_dir().join(format!(
                "curado-decode-{}-{channels}.wav",
                std::process::id()
            ));
            let spec = hound::WavSpec {
                channels,
                sample_rate: 44100,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };
            let mut writer = hound::WavWriter::create(&path, spec).unwrap();
            for _ in 0..4410 {
                for _ in 0..channels {
                    writer.write_sample(8192_i16).unwrap();
                }
            }
            writer.finalize().unwrap();
            let clip = decode(&path, 5).unwrap();
            assert_eq!(clip.samples.len(), 4410);
            assert!((clip.duration() - 0.1).abs() < 0.00001);
            assert!((clip.samples[10][0] - 0.25).abs() < 0.001);
            assert_eq!(clip.samples[10][0], clip.samples[10][1]);
            assert_eq!(clip.waveform.len(), 5);
            std::fs::remove_file(path).unwrap();
        }
        let path = std::env::temp_dir().join(format!("curado-invalid-{}.wav", std::process::id()));
        std::fs::write(&path, b"not an audio file").unwrap();
        assert!(decode(&path, 1).is_err());
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn long_files_respect_the_limit_and_can_be_cancelled() {
        let path = std::env::temp_dir().join(format!("curado-limit-{}.wav", std::process::id()));
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 8000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();
        for _ in 0..8000 * 10 * 2 {
            writer.write_sample(1000_i16).unwrap();
        }
        writer.finalize().unwrap();
        let frame = std::mem::size_of::<[f32; 2]>();
        let fits = decode_with_limit(&path, 1, None, 8000 * 10 * frame).unwrap();
        assert_eq!(fits.samples.len(), 80000);
        assert_eq!(
            fits.samples.capacity(),
            80000,
            "holds exactly what it needs"
        );
        assert_eq!(
            decode_with_limit(&path, 1, None, 8000 * 5 * frame).unwrap_err(),
            TOO_LARGE
        );
        let calls = std::cell::Cell::new(0);
        let cancelled = decode_bounded(&path, 1, None, MAX_CLIP_BYTES, &|| {
            calls.set(calls.get() + 1);
            calls.get() > 3
        });
        assert_eq!(cancelled.unwrap_err(), CANCELLED);
        assert_eq!(calls.get(), 4, "stops at the next packet");
        std::fs::remove_file(path).unwrap();
    }
}
