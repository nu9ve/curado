use crate::{analysis::WaveformBuilder, annotations::CueState};
use std::{
    fs::File,
    panic::{catch_unwind, AssertUnwindSafe},
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
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
/// A damaged file can make a decoder spin without producing audio. Real tracks of 90 minutes
/// decode in seconds, so this only ever stops one that is not going anywhere.
pub const MAX_DECODE_TIME: Duration = Duration::from_secs(180);
const TIMED_OUT: &str = "El archivo tardó demasiado en decodificarse y parece dañado.";
const DAMAGED: &str = "El archivo está dañado y no se pudo leer.";
/// Damaged frames are skipped, as a player would. This many in a row means it is not audio.
const MAX_BAD_PACKETS: u32 = 500;
const SAMPLE_RATES: std::ops::RangeInclusive<u32> = 8000..=384_000;
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
    /// Integrated LUFS, or nothing for silence.
    pub loudness: Option<f32>,
    /// Seconds of every beat, for a track whose tempo moves. Sorted, at least two.
    pub beats: Option<Arc<[f64]>>,
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
    decode_within(path, id, cached, max_bytes, cancelled, MAX_DECODE_TIME)
}
/// A file is outside data: a decoder bug it triggers becomes an error for that file instead of
/// taking the application down, or a lock held by whoever asked.
pub fn decode_within(
    path: &Path,
    id: u64,
    cached: Option<crate::music::MusicAnalysis>,
    max_bytes: usize,
    cancelled: &dyn Fn() -> bool,
    limit: Duration,
) -> Result<Clip, String> {
    catch_unwind(AssertUnwindSafe(|| {
        decode_inner(path, id, cached, max_bytes, cancelled, limit)
    }))
    .unwrap_or_else(|_| Err(DAMAGED.into()))
}
fn decode_inner(
    path: &Path,
    id: u64,
    cached: Option<crate::music::MusicAnalysis>,
    max_bytes: usize,
    cancelled: &dyn Fn() -> bool,
    limit: Duration,
) -> Result<Clip, String> {
    let started = Instant::now();
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
    let mut bad_packets = 0;
    loop {
        if cancelled() {
            return Err(CANCELLED.into());
        }
        if started.elapsed() > limit {
            return Err(TIMED_OUT.into());
        }
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(format!("Error de lectura: {e}")),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(Error::DecodeError(_)) if bad_packets < MAX_BAD_PACKETS => {
                bad_packets += 1;
                continue;
            }
            Err(e) => return Err(format!("Error al decodificar: {e}")),
        };
        bad_packets = 0;
        let rate = decoded.spec().rate;
        if !SAMPLE_RATES.contains(&rate) {
            return Err(format!("Frecuencia de muestreo no válida: {rate} Hz"));
        }
        // A rate that changes partway is a damaged frame: its samples would play at a wrong speed.
        if analysis.is_some() && rate != sample_rate {
            continue;
        }
        sample_rate = rate;
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
    let mut music = cached
        .filter(|m| m.valid())
        .unwrap_or_else(|| crate::music::analyze(&samples, sample_rate));
    // Exports have a grid but no first sound, which comes from the audio.
    if music.imported && music.first_sound == 0. {
        music.first_sound = crate::beat::first_sound(&samples, sample_rate);
    }
    let loudness = crate::loudness::integrated(&samples, sample_rate);
    Ok(Clip {
        id,
        cues: Arc::new(CueState::default()),
        loudness,
        beats: None,
        samples,
        sample_rate,
        waveform,
        title,
        artist,
        music,
    })
}

#[cfg(test)]
mod fuzz {
    //! Damaged files on purpose. Mutations come from a fixed seed, so a failure repeats.
    use super::*;
    use std::time::Instant;

    pub struct Rng(pub u64);
    impl Rng {
        pub fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        pub fn below(&mut self, limit: usize) -> usize {
            (self.next() % limit.max(1) as u64) as usize
        }
    }
    pub fn mutate(rng: &mut Rng, data: &mut Vec<u8>) {
        match rng.below(6) {
            0 => {
                for _ in 0..1 + rng.below(32) {
                    let at = rng.below(data.len());
                    data[at] ^= 1 << rng.below(8);
                }
            }
            1 => {
                let at = rng.below(data.len());
                let end = (at + rng.below(4096)).min(data.len());
                data[at..end].fill(rng.next() as u8);
            }
            2 => data.truncate(rng.below(data.len()).max(1)),
            3 => {
                // Headers are where lengths and rates live.
                for _ in 0..1 + rng.below(8) {
                    let at = rng.below(data.len().min(256));
                    data[at] = rng.next() as u8;
                }
            }
            4 => {
                let at = rng.below(data.len());
                let extra: Vec<u8> = (0..rng.below(2048)).map(|_| rng.next() as u8).collect();
                data.splice(at..at, extra);
            }
            _ => {
                let from = rng.below(data.len());
                let to = rng.below(data.len());
                let len = rng.below(1024).min(data.len() - from.max(to));
                data.copy_within(from..from + len, to);
            }
        }
    }
    fn wav(frames: usize, channels: u16, bits: u16) -> Vec<u8> {
        let bytes = frames * channels as usize * bits as usize / 8;
        let mut out = Vec::with_capacity(44 + bytes);
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + bytes as u32).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16_u32.to_le_bytes());
        out.extend_from_slice(&1_u16.to_le_bytes());
        out.extend_from_slice(&channels.to_le_bytes());
        out.extend_from_slice(&44100_u32.to_le_bytes());
        out.extend_from_slice(&(44100 * channels as u32 * bits as u32 / 8).to_le_bytes());
        out.extend_from_slice(&(channels * bits / 8).to_le_bytes());
        out.extend_from_slice(&bits.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(bytes as u32).to_le_bytes());
        for i in 0..frames * channels as usize {
            let v = ((i as f32 * 0.03).sin() * 20000.) as i16;
            out.extend_from_slice(&v.to_le_bytes()[..(bits as usize / 8).min(2)]);
            out.extend(std::iter::repeat_n(
                0,
                (bits as usize / 8).saturating_sub(2),
            ));
        }
        out
    }
    /// Whatever comes back must be safe to hand to the engine.
    fn check(clip: &Clip) {
        assert!(SAMPLE_RATES.contains(&clip.sample_rate));
        assert!(!clip.samples.is_empty());
        assert!(clip
            .samples
            .iter()
            .flatten()
            .all(|v| v.is_finite() && v.abs() <= 4.));
        assert!(clip.music.bpm.is_none_or(|bpm| bpm.is_finite() && bpm > 0.));
    }
    fn run(seeds: &[Vec<u8>], extension: &str, rounds: usize, seed: u64) -> (usize, usize, f64) {
        let path = std::env::temp_dir().join(format!(
            "curado-fuzz-{}-{seed}.{extension}",
            std::process::id()
        ));
        let mut rng = Rng(seed);
        let (mut decoded, mut refused, mut slowest) = (0, 0, 0_f64);
        for round in 0..rounds {
            let mut data = seeds[round % seeds.len()].clone();
            for _ in 0..1 + rng.below(3) {
                mutate(&mut rng, &mut data);
            }
            std::fs::write(&path, &data).unwrap();
            let started = Instant::now();
            match decode_within(
                &path,
                1,
                None,
                256 * 1024 * 1024,
                &|| false,
                Duration::from_secs(20),
            ) {
                Ok(clip) => {
                    check(&clip);
                    decoded += 1;
                }
                Err(_) => refused += 1,
            }
            slowest = slowest.max(started.elapsed().as_secs_f64());
        }
        let _ = std::fs::remove_file(&path);
        (decoded, refused, slowest)
    }

    #[test]
    fn a_damaged_wav_is_decoded_or_refused_and_never_more() {
        let seeds = [wav(4000, 2, 16), wav(4000, 1, 16), wav(2000, 2, 24)];
        let (decoded, refused, _) = run(&seeds, "wav", 400, 0x1234_5678_9ABC_DEF1);
        assert_eq!(decoded + refused, 400);
        assert!(
            decoded > 0 && refused > 0,
            "{decoded} decoded, {refused} refused"
        );
    }

    #[test]
    fn a_decoder_panic_stays_inside_the_load() {
        let result: Result<Clip, String> =
            catch_unwind(AssertUnwindSafe(|| panic!("from a decoder")))
                .unwrap_or_else(|_| Err(DAMAGED.into()));
        assert_eq!(result.err().as_deref(), Some(DAMAGED));
    }

    #[test]
    fn a_load_stops_at_its_time_limit() {
        let path = std::env::temp_dir().join(format!("curado-limit-{}.wav", std::process::id()));
        std::fs::write(&path, wav(4000, 2, 16)).unwrap();
        let result = decode_within(&path, 1, None, 1 << 28, &|| false, Duration::ZERO);
        assert_eq!(result.err().as_deref(), Some(TIMED_OUT));
        let _ = std::fs::remove_file(&path);
    }

    /// `CURADO_FUZZ_DIR=/path CURADO_FUZZ_ROUNDS=2000 cargo test --release -p curado-audio
    /// real_files -- --ignored --nocapture`. Reads the first part of real files there and damages
    /// copies in the temporary folder; the originals are never written.
    #[test]
    #[ignore = "needs a folder of real audio"]
    fn real_files() {
        let root = std::path::PathBuf::from(std::env::var("CURADO_FUZZ_DIR").unwrap());
        let rounds: usize =
            std::env::var("CURADO_FUZZ_ROUNDS").map_or(1000, |value| value.parse().unwrap());
        let mut by_extension: std::collections::BTreeMap<String, Vec<Vec<u8>>> = Default::default();
        let mut pending = vec![root];
        while let Some(folder) = pending.pop() {
            let Ok(listing) = std::fs::read_dir(folder) else {
                continue;
            };
            for entry in listing.flatten() {
                let path = entry.path();
                let name = entry.file_name();
                if name.to_string_lossy().starts_with('.') {
                    continue;
                }
                if path.is_dir() {
                    pending.push(path);
                    continue;
                }
                let Some(extension) = path.extension().and_then(|e| e.to_str()) else {
                    continue;
                };
                let extension = extension.to_ascii_lowercase();
                let extension_name = extension.clone();
                if std::env::var("CURADO_FUZZ_ONLY").is_ok_and(|only| !only.contains(&extension)) {
                    continue;
                }
                if !["mp3", "wav", "flac", "aif", "aiff", "m4a", "aac", "ogg"]
                    .contains(&extension.as_str())
                {
                    continue;
                }
                let seeds = by_extension.entry(extension).or_default();
                if seeds.len() >= 6 {
                    continue;
                }
                use std::io::Read;
                let mut data = Vec::new();
                // These keep their index at the end, so a cut copy would not even open.
                let whole = matches!(extension_name.as_str(), "m4a" | "aac" | "aif" | "aiff");
                if whole && path.metadata().map_or(true, |file| file.len() > 24_000_000) {
                    continue;
                }
                let limit = if whole { 24_000_000 } else { 400_000 };
                if std::fs::File::open(&path)
                    .and_then(|file| file.take(limit).read_to_end(&mut data))
                    .is_ok()
                    && data.len() > 1024
                {
                    seeds.push(data);
                }
            }
        }
        by_extension.retain(|_, seeds| !seeds.is_empty());
        for (extension, seeds) in &by_extension {
            let started = Instant::now();
            let whole = matches!(extension.as_str(), "m4a" | "aac" | "aif" | "aiff");
            let rounds = if whole { rounds.min(60) } else { rounds };
            let (decoded, refused, slowest) = run(seeds, extension, rounds, 0xC0FF_EE00_1234_5677);
            println!(
                "{extension}: {} seeds, {rounds} mutations in {:.0} s: {decoded} decoded, \
                 {refused} refused, slowest {slowest:.2} s, no crash",
                seeds.len(),
                started.elapsed().as_secs_f64()
            );
        }
    }
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
