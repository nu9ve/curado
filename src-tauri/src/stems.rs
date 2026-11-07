//! Separating tracks into stems, in a Python process kept apart from the app. Optional: nothing
//! is bundled, and nothing else depends on it. Work happens in a temporary folder that always
//! goes away, and a result reaches the library whole or not at all. Stems that nobody chose to
//! keep are a cache: they can be made again, so they make room when the space runs out.
use crate::{
    identity::{self, Identity},
    library::Database,
};
use curado_audio::decode::Clip;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

const WORKER: &str = include_str!("../native/separate.py");
const MANIFEST: &str = "manifest.json";
const JOB_PREFIX: &str = ".job-";
pub const DEFAULT_LIMIT_GB: u64 = 20;
const SETTING_PYTHON: &str = "stems-python";
const SETTING_FOLDER: &str = "stems-folder";
const SETTING_LIMIT: &str = "stems-limit-gb";
pub const MODELS: [(&str, usize); 3] = [("htdemucs", 4), ("htdemucs_ft", 4), ("htdemucs_6s", 6)];
const WAV_HEADER: u64 = 44;
/// A float WAV of a second, the most a job holds of each stem while it runs.
const FLOAT_BYTES_PER_SECOND: f64 = 44100. * 8.;

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub track: u64,
    pub title: String,
    /// preparing, loading, separating, writing, storing, done, cancelled or error.
    pub stage: String,
    pub message: Option<String>,
    /// Over the limit with only stems you chose to keep left.
    pub over_limit: bool,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Runtime {
    pub python: String,
    pub engine: String,
    pub version: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub runtime: Option<Runtime>,
    pub folder: String,
    pub used_bytes: u64,
    pub limit_gb: u64,
    pub free_bytes: Option<u64>,
    pub job: Option<Progress>,
}
#[derive(Default)]
pub struct Stems {
    runtime: Mutex<Option<Runtime>>,
    child: Arc<Mutex<Option<Child>>>,
    cancelled: Arc<AtomicBool>,
    pub job: Mutex<Option<Progress>>,
}
#[derive(Deserialize)]
struct Probe {
    packages: std::collections::HashMap<String, String>,
    #[serde(rename = "appleSilicon")]
    apple_silicon: bool,
}
#[derive(Deserialize)]
struct WorkerEvent {
    stage: String,
    message: Option<String>,
}
#[derive(Deserialize)]
struct WorkerResult {
    engine: String,
    rate: u32,
    stems: Vec<WorkerStem>,
}
#[derive(Deserialize)]
struct WorkerStem {
    name: String,
    file: String,
}
#[derive(Deserialize, Serialize)]
pub struct Manifest {
    version: u32,
    parent_fingerprint: Option<String>,
    title: String,
    artist: String,
    model: String,
    engine: String,
    /// What every stem of this set was scaled by to fit without clipping.
    gain: f64,
    stems: Vec<ManifestStem>,
}
#[derive(Deserialize, Serialize)]
struct ManifestStem {
    name: String,
    file: String,
    duration: f64,
    rate: u32,
    sha256: String,
}
pub fn model_stems(model: &str) -> Option<usize> {
    MODELS
        .iter()
        .find(|(name, _)| *name == model)
        .map(|(_, stems)| *stems)
}
pub fn folder(db: &Database, data_dir: &Path) -> PathBuf {
    db.setting::<String>(SETTING_FOLDER)
        .map(PathBuf::from)
        .unwrap_or_else(|| data_dir.join("stems"))
}
pub fn limit_gb(db: &Database) -> u64 {
    db.setting(SETTING_LIMIT).unwrap_or(DEFAULT_LIMIT_GB)
}
pub fn set_limit_gb(db: &Database, gigabytes: u64) -> Result<(), String> {
    if !(1..=4096).contains(&gigabytes) {
        return Err("El límite debe estar entre 1 y 4096 GB".into());
    }
    db.save(SETTING_LIMIT, &gigabytes)
}
pub fn set_folder(db: &Database, path: &Path) -> Result<(), String> {
    db.save(SETTING_FOLDER, &path.to_string_lossy())
}
pub fn set_python(db: &Database, path: &Path) -> Result<(), String> {
    db.save(SETTING_PYTHON, &path.to_string_lossy())
}
pub fn used_bytes(db: &Database) -> u64 {
    db.connection
        .query_row(
            "SELECT COALESCE(SUM(file_size),0) FROM tracks WHERE owned=1",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0)
}
fn worker_path(cache: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(cache).map_err(|e| e.to_string())?;
    let path = cache.join("separate.py");
    if fs::read_to_string(&path).ok().as_deref() != Some(WORKER) {
        fs::write(&path, WORKER).map_err(|e| e.to_string())?;
    }
    Ok(path)
}
/// Where an environment may already be: one chosen by hand, the one the setup script makes, and
/// the one SoundShop already has, rather than a second copy of the same half gigabyte.
fn candidates(db: &Database, data_dir: &Path) -> Vec<PathBuf> {
    let mut list: Vec<PathBuf> = db
        .setting::<String>(SETTING_PYTHON)
        .map(PathBuf::from)
        .into_iter()
        .collect();
    list.push(data_dir.join(VENV_PYTHON));
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        list.push(
            Path::new(&home)
                .join("code/ai/soundshop/.venv-separation")
                .join(VENV_PYTHON),
        );
    }
    list
}
/// Where a virtual environment keeps its interpreter on this system.
#[cfg(not(windows))]
const VENV_PYTHON: &str = "bin/python";
#[cfg(windows)]
const VENV_PYTHON: &str = r"Scripts\python.exe";
fn probe(python: &Path, worker: &Path) -> Option<Runtime> {
    let output = Command::new(python)
        .arg(worker)
        .arg("--probe")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let found: Probe = serde_json::from_slice(&output.stdout).ok()?;
    let (engine, version) = match (
        found.packages.get("demucs-mlx"),
        found.packages.get("demucs"),
    ) {
        (Some(version), _) if found.apple_silicon => ("demucs-mlx", version),
        (_, Some(version)) => ("demucs", version),
        _ => return None,
    };
    Some(Runtime {
        python: python.to_string_lossy().into(),
        engine: engine.into(),
        version: version.clone(),
    })
}
/// Models beside an environment that already has them, or a cache of curado's own.
fn model_cache(python: &Path, cache: &Path) -> PathBuf {
    let beside = python
        .ancestors()
        .nth(3)
        .map(|root| root.join(".separation-models"));
    beside
        .filter(|path| path.is_dir())
        .unwrap_or_else(|| cache.join("separation"))
}
impl Stems {
    pub fn runtime(&self, db: &Mutex<Database>, data_dir: &Path, cache: &Path) -> Option<Runtime> {
        if let Some(known) = self.runtime.lock().unwrap().clone() {
            return Some(known);
        }
        let list = candidates(&db.lock().unwrap(), data_dir);
        let worker = worker_path(cache).ok()?;
        let found = list
            .iter()
            .filter(|path| path.exists())
            .find_map(|path| probe(path, &worker));
        self.runtime.lock().unwrap().clone_from(&found);
        found
    }
    pub fn forget_runtime(&self) {
        *self.runtime.lock().unwrap() = None;
    }
    pub fn busy(&self) -> bool {
        self.job
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|job| !finished(&job.stage))
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        if let Some(child) = self.child.lock().unwrap().as_mut() {
            let _ = child.kill();
        }
    }
    pub fn cancelled_flag(&self) -> Arc<AtomicBool> {
        self.cancelled.clone()
    }
}
pub fn finished(stage: &str) -> bool {
    matches!(stage, "done" | "cancelled" | "error")
}
/// Runs the worker until it ends. Ends with it on cancel, and if curado goes away, its input
/// closes and it leaves too.
#[allow(clippy::too_many_arguments)]
fn run_worker(
    runtime: &Runtime,
    worker: &Path,
    model: &str,
    input: &Path,
    output: &Path,
    cache: &Path,
    slot: &Mutex<Option<Child>>,
    cancelled: &AtomicBool,
    mut stage: impl FnMut(&str),
) -> Result<(), String> {
    let mut child = Command::new(&runtime.python)
        .arg(worker)
        .args(["--engine", &runtime.engine, "--model", model])
        .arg("--input")
        .arg(input)
        .arg("--output")
        .arg(output)
        .arg("--cache")
        .arg(cache)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("No se pudo iniciar Python: {e}"))?;
    let stdout = child.stdout.take().ok_or("Sin salida del proceso")?;
    *slot.lock().unwrap() = Some(child);
    let mut failure = None;
    let mut complete = false;
    for line in BufReader::new(stdout).lines() {
        let Ok(line) = line else { break };
        let Ok(event) = serde_json::from_str::<WorkerEvent>(&line) else {
            continue;
        };
        match event.stage.as_str() {
            "error" => failure = event.message.or(Some("Falló la separación".into())),
            "complete" => complete = true,
            other => stage(other),
        }
    }
    let status = slot
        .lock()
        .unwrap()
        .take()
        .and_then(|mut child| child.wait().ok());
    if cancelled.load(Ordering::SeqCst) {
        return Err(CANCELLED.into());
    }
    if let Some(failure) = failure {
        return Err(failure);
    }
    if !complete || !status.is_some_and(|status| status.success()) {
        return Err("El proceso de separación terminó antes de tiempo".into());
    }
    Ok(())
}
pub const CANCELLED: &str = "cancelled";
pub fn write_float_wav(path: &Path, samples: &[[f32; 2]], rate: u32) -> Result<(), String> {
    let mut file = std::io::BufWriter::new(File::create(path).map_err(|e| e.to_string())?);
    let size = (samples.len() * 8) as u32;
    let mut header = Vec::with_capacity(WAV_HEADER as usize);
    header.extend(b"RIFF");
    header.extend((36 + size).to_le_bytes());
    header.extend(b"WAVEfmt ");
    header.extend(16_u32.to_le_bytes());
    header.extend(3_u16.to_le_bytes());
    header.extend(2_u16.to_le_bytes());
    header.extend(rate.to_le_bytes());
    header.extend((rate * 8).to_le_bytes());
    header.extend(8_u16.to_le_bytes());
    header.extend(32_u16.to_le_bytes());
    header.extend(b"data");
    header.extend(size.to_le_bytes());
    file.write_all(&header).map_err(|e| e.to_string())?;
    for frame in samples {
        file.write_all(&frame[0].to_le_bytes())
            .map_err(|e| e.to_string())?;
        file.write_all(&frame[1].to_le_bytes())
            .map_err(|e| e.to_string())?;
    }
    file.flush().map_err(|e| e.to_string())
}
/// The worker writes exactly one kind of file; anything else did not come from it.
fn float_wav_frames(path: &Path) -> Result<(u16, u64), String> {
    let invalid = || String::from("El proceso devolvió un archivo inesperado");
    let mut file = File::open(path).map_err(|_| invalid())?;
    let mut header = [0; WAV_HEADER as usize];
    file.read_exact(&mut header).map_err(|_| invalid())?;
    let word = |at: usize| u16::from_le_bytes([header[at], header[at + 1]]);
    let long = |at: usize| {
        u32::from_le_bytes([header[at], header[at + 1], header[at + 2], header[at + 3]])
    };
    if &header[..4] != b"RIFF" || &header[8..16] != b"WAVEfmt " || &header[36..40] != b"data" {
        return Err(invalid());
    }
    let channels = word(22);
    if word(20) != 3 || word(34) != 32 || !(1..=2).contains(&channels) {
        return Err(invalid());
    }
    let size = long(40) as u64;
    if file.metadata().map_err(|_| invalid())?.len() != WAV_HEADER + size {
        return Err(invalid());
    }
    Ok((channels, size / (4 * channels as u64)))
}
fn float_peak(path: &Path) -> Result<f32, String> {
    let mut file = BufReader::with_capacity(1 << 20, File::open(path).map_err(|e| e.to_string())?);
    file.seek(SeekFrom::Start(WAV_HEADER))
        .map_err(|e| e.to_string())?;
    let mut peak = 0_f32;
    let mut bytes = [0; 4];
    while file.read_exact(&mut bytes).is_ok() {
        let value = f32::from_le_bytes(bytes);
        if !value.is_finite() {
            return Err("El proceso devolvió datos inválidos".into());
        }
        peak = peak.max(value.abs());
    }
    Ok(peak)
}
fn scale_float(path: &Path, gain: f32) -> Result<(), String> {
    let mut data = fs::read(path).map_err(|e| e.to_string())?;
    for sample in data[WAV_HEADER as usize..].as_chunks_mut::<4>().0 {
        *sample = (f32::from_le_bytes(*sample) * gain).to_le_bytes();
    }
    fs::write(path, data).map_err(|e| e.to_string())
}
/// Lossless and about a third of the size. Where the system cannot, the WAV stays as it is.
fn compress(input: &Path, directory: &Path, name: &str) -> Result<PathBuf, String> {
    let converter = Path::new("/usr/bin/afconvert");
    if converter.exists() {
        let out = directory.join(format!("{name}.flac"));
        let status = Command::new(converter)
            .arg(input)
            .args(["-f", "flac", "-d", "flac"])
            .arg(&out)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| e.to_string())?;
        if status.success() && out.exists() {
            let _ = fs::remove_file(input);
            return Ok(out);
        }
        let _ = fs::remove_file(&out);
    }
    Ok(input.to_path_buf())
}
pub struct Request<'a> {
    pub track: u64,
    pub model: &'a str,
    pub title: &'a str,
    pub artist: &'a str,
    pub parent_fingerprint: Option<String>,
    /// Names this source's folder, so the same content is never separated into two places.
    pub key: String,
    pub duration: f64,
}
/// The files of a finished job, moved into place, with what the library needs to list them.
pub struct Stored {
    pub name: String,
    pub path: PathBuf,
    pub duration: f64,
    pub rate: u32,
    pub waveform: Vec<u8>,
    pub identity: Identity,
}
fn room_needed(duration: f64, stems: usize) -> u64 {
    (duration * FLOAT_BYTES_PER_SECOND * (stems + 1) as f64 * 1.2) as u64 + (1 << 30)
}
/// Room for the temporary files of a job, which are larger than what stays.
pub fn check_room(store: &Path, duration: f64, stems: usize) -> Result<(), String> {
    let mut near = store;
    while !near.exists() {
        match near.parent() {
            Some(parent) => near = parent,
            None => return Ok(()),
        }
    }
    let needed = room_needed(duration, stems);
    match identity::free_bytes(near) {
        Some(free) if free < needed => Err(format!(
            "No hay espacio para separar: hacen falta unos {:.1} GB libres y hay {:.1} GB",
            needed as f64 / 1e9,
            free as f64 / 1e9
        )),
        _ => Ok(()),
    }
}
/// Checks what the worker made, and only if all of it is right, moves it to its place.
pub fn store_results(
    job: &Path,
    store: &Path,
    request: &Request,
    decode: &dyn Fn(&Path) -> Result<Clip, String>,
    engine_stage: &mut dyn FnMut(&str),
) -> Result<Vec<Stored>, String> {
    let result: WorkerResult = serde_json::from_slice(
        &fs::read(job.join("result.json")).map_err(|_| "El proceso no entregó resultado")?,
    )
    .map_err(|_| "El resultado del proceso no es válido")?;
    let expected = model_stems(request.model).ok_or("Modelo desconocido")?;
    if result.stems.len() != expected {
        return Err("El proceso entregó otro número de stems".into());
    }
    let mut peak = 0_f32;
    for stem in &result.stems {
        if !["drums", "bass", "other", "vocals", "guitar", "piano"].contains(&stem.name.as_str())
            || stem.file != format!("{}.wav", stem.name)
        {
            return Err("El proceso entregó un stem inesperado".into());
        }
        let path = job.join(&stem.file);
        let (_, frames) = float_wav_frames(&path)?;
        // Models keep the length; a stem that does not has lost something.
        let seconds = frames as f64 / result.rate as f64;
        if (seconds - request.duration).abs() > 0.1 {
            return Err("Un stem no dura lo mismo que la pista".into());
        }
        peak = peak.max(float_peak(&path)?);
    }
    // FLAC holds ±1. The same gain on all, so they still add up to the same mix.
    let gain = if peak > 1. { 1. / peak } else { 1. };
    engine_stage("storing");
    let mut stored = Vec::new();
    let mut listed = Vec::new();
    for stem in &result.stems {
        let raw = job.join(&stem.file);
        if gain < 1. {
            scale_float(&raw, gain)?;
        }
        let packed = compress(&raw, job, &stem.name)?;
        let clip = decode(&packed)?;
        let file = packed
            .file_name()
            .ok_or("Stem sin nombre")?
            .to_string_lossy()
            .to_string();
        listed.push(ManifestStem {
            name: stem.name.clone(),
            file: file.clone(),
            duration: clip.duration(),
            rate: clip.sample_rate,
            sha256: identity::sha256(&packed)?,
        });
        stored.push(Stored {
            name: stem.name.clone(),
            path: packed,
            duration: clip.duration(),
            rate: clip.sample_rate,
            waveform: curado_audio::analysis::encode(&clip.waveform),
            identity: Identity::default(),
        });
    }
    let target = store.join(&request.key).join(request.model);
    fs::create_dir_all(&target).map_err(|e| e.to_string())?;
    let manifest = Manifest {
        version: 1,
        parent_fingerprint: request.parent_fingerprint.clone(),
        title: request.title.into(),
        artist: request.artist.into(),
        model: request.model.into(),
        engine: result.engine,
        gain: gain as f64,
        stems: listed,
    };
    // Whole or not at all: a failure halfway takes back what had already moved.
    let mut moved: Vec<PathBuf> = Vec::new();
    let placed = (|| -> Result<(), String> {
        for item in &mut stored {
            let name = item.path.file_name().ok_or("Stem sin nombre")?.to_owned();
            let destination = target.join(name);
            fs::rename(&item.path, &destination).map_err(|e| e.to_string())?;
            moved.push(destination.clone());
            item.identity = identity::identify(&destination)?;
            item.path = destination;
        }
        let text = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
        fs::write(target.join(MANIFEST), text).map_err(|e| e.to_string())
    })();
    if let Err(error) = placed {
        for path in moved {
            let _ = fs::remove_file(path);
        }
        let _ = fs::remove_dir(&target);
        let _ = fs::remove_dir(store.join(&request.key));
        return Err(error);
    }
    Ok(stored)
}
/// All or nothing: a separation appears in the library complete.
pub fn commit(db: &Database, request: &Request, stored: &[Stored]) -> Result<Vec<u64>, String> {
    let tx = db
        .connection
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;
    let mut ids = Vec::new();
    for item in stored {
        ids.push(db.save_stem(
            Some(request.track),
            &format!("{} · {}", request.title, item.name),
            request.artist,
            request.model,
            &item.name,
            &item.path,
            item.duration,
            item.rate,
            Some(&item.waveform),
            &item.identity,
        )?);
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(ids)
}
/// Takes a stem's file away, only if it is one curado wrote: its manifest must list it.
pub fn remove_owned_file(path: &Path) -> Result<(), String> {
    let Some(directory) = path.parent() else {
        return Ok(());
    };
    let manifest_path = directory.join(MANIFEST);
    let Ok(text) = fs::read(&manifest_path) else {
        return Ok(());
    };
    let mut manifest: Manifest = serde_json::from_slice(&text).map_err(|e| e.to_string())?;
    let name = path.file_name().map(|n| n.to_string_lossy().to_string());
    if !manifest
        .stems
        .iter()
        .any(|stem| Some(&stem.file) == name.as_ref())
    {
        return Ok(());
    }
    match fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.to_string()),
    }
    manifest
        .stems
        .retain(|stem| Some(&stem.file) != name.as_ref());
    if manifest.stems.is_empty() {
        let _ = fs::remove_file(&manifest_path);
        let _ = fs::remove_dir(directory);
        if let Some(source) = directory.parent() {
            let _ = fs::remove_dir(source);
        }
    } else {
        fs::write(
            &manifest_path,
            serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}
/// Oldest unused first, among what nobody chose to keep. A favorite, a tag, a playlist, a bank,
/// a saved loop or a deck holding it all count as choosing. Says if the limit could not be met.
pub fn make_room(db: &Database, limit_bytes: u64, keep: &HashSet<u64>) -> Result<bool, String> {
    let mut used = used_bytes(db);
    if used <= limit_bytes {
        return Ok(false);
    }
    let banked: HashSet<u64> = db
        .setting::<Vec<Vec<Option<u64>>>>("sample-banks")
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .flatten()
        .collect();
    let candidates: Vec<(u64, PathBuf, u64)> = {
        let mut stmt = db
            .connection
            .prepare(
                "SELECT id,path,COALESCE(file_size,0) FROM tracks t WHERE owned=1 AND favorite=0
                AND NOT EXISTS(SELECT 1 FROM tags WHERE track=t.id)
                AND NOT EXISTS(SELECT 1 FROM playlist_tracks WHERE track=t.id)
                AND NOT EXISTS(SELECT 1 FROM tracks c WHERE c.parent=t.id AND c.kind='loop')
                ORDER BY used_at,id",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok((r.get(0)?, PathBuf::from(r.get::<_, String>(1)?), r.get(2)?))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        rows
    };
    for (id, path, size) in candidates {
        if used <= limit_bytes {
            break;
        }
        if keep.contains(&id) || banked.contains(&id) {
            continue;
        }
        remove_owned_file(&path)?;
        db.connection
            .execute("DELETE FROM tracks WHERE id=?", [id])
            .map_err(|e| e.to_string())?;
        used = used.saturating_sub(size);
    }
    Ok(used > limit_bytes)
}
/// After a crash: temporary folders go, and stems the library lost track of come back, since
/// their manifests say what they are. Never deletes a stem it does not understand.
pub fn tidy(db: &Mutex<Database>, store: &Path) {
    let Ok(listing) = fs::read_dir(store) else {
        return;
    };
    for source in listing.flatten() {
        let path = source.path();
        if source.file_name().to_string_lossy().starts_with(JOB_PREFIX) {
            let _ = fs::remove_dir_all(path);
            continue;
        }
        let Ok(models) = fs::read_dir(&path) else {
            continue;
        };
        for directory in models.flatten() {
            let directory = directory.path();
            let Some(manifest) = fs::read(directory.join(MANIFEST))
                .ok()
                .and_then(|text| serde_json::from_slice::<Manifest>(&text).ok())
            else {
                continue;
            };
            let db = db.lock().unwrap();
            let parent: Option<u64> = manifest.parent_fingerprint.as_ref().and_then(|print| {
                db.connection
                    .query_row(
                        "SELECT id FROM tracks WHERE fingerprint=? AND kind<>'stem' ORDER BY kind='loop',id LIMIT 1",
                        [print],
                        |r| r.get(0),
                    )
                    .ok()
            });
            for stem in &manifest.stems {
                let file = directory.join(&stem.file);
                if !file.exists() {
                    continue;
                }
                let known: bool = db
                    .connection
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM tracks WHERE path=?)",
                        params![file.to_string_lossy()],
                        |r| r.get(0),
                    )
                    .unwrap_or(true);
                if known {
                    continue;
                }
                let Ok(found) = identity::identify(&file) else {
                    continue;
                };
                let _ = db.save_stem(
                    parent,
                    &format!("{} · {}", manifest.title, stem.name),
                    &manifest.artist,
                    &manifest.model,
                    &stem.name,
                    &file,
                    stem.duration,
                    stem.rate,
                    None,
                    &found,
                );
            }
        }
    }
}
pub fn job_folder(store: &Path, track: u64) -> PathBuf {
    store.join(format!("{JOB_PREFIX}{track}"))
}
/// Everything a job does outside the library. Whatever happens, its temporary folder goes.
#[allow(clippy::too_many_arguments)]
pub fn separate(
    stems: &Stems,
    runtime: &Runtime,
    cache: &Path,
    store: &Path,
    request: &Request,
    prepare: &dyn Fn(&Path) -> Result<(), String>,
    decode: &dyn Fn(&Path) -> Result<Clip, String>,
    report: &mut dyn FnMut(&str),
) -> Result<Vec<Stored>, String> {
    let job = job_folder(store, request.track);
    let _ = fs::remove_dir_all(&job);
    fs::create_dir_all(&job).map_err(|e| e.to_string())?;
    let result = (|| {
        let input = job.join("input.wav");
        prepare(&input)?;
        if stems.cancelled.load(Ordering::SeqCst) {
            return Err(CANCELLED.to_string());
        }
        let worker = worker_path(cache)?;
        run_worker(
            runtime,
            &worker,
            request.model,
            &input,
            &job,
            &model_cache(Path::new(&runtime.python), cache),
            &stems.child,
            &stems.cancelled,
            &mut *report,
        )?;
        let _ = fs::remove_file(&input);
        store_results(&job, store, request, decode, report)
    })();
    let _ = fs::remove_dir_all(&job);
    result
}
/// Waits for a job to notice a flag, for the app closing.
pub fn wait_until_idle(stems: &Stems, max: Duration) {
    let started = Instant::now();
    while stems.busy() && started.elapsed() < max {
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    fn folder(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("curado-stems-{name}"));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path.canonicalize().unwrap()
    }
    fn sine(seconds: f64, amplitude: f32) -> Vec<[f32; 2]> {
        (0..(seconds * 44100.) as usize)
            .map(|i| {
                let x = (i as f32 * 0.05).sin() * amplitude;
                [x, x * 0.5]
            })
            .collect()
    }
    fn decode(path: &Path) -> Result<Clip, String> {
        curado_audio::decode::decode_bounded(path, 0, Some(Default::default()), 1 << 30, &|| false)
    }
    fn request<'a>(seconds: f64) -> Request<'a> {
        Request {
            track: 1,
            model: "htdemucs",
            title: "A",
            artist: "B",
            parent_fingerprint: Some("print".into()),
            key: "print".into(),
            duration: seconds,
        }
    }
    fn fake_result(job: &Path, seconds: f64, amplitude: f32, names: &[&str]) {
        let mut stems = Vec::new();
        for name in names {
            write_float_wav(
                &job.join(format!("{name}.wav")),
                &sine(seconds, amplitude),
                44100,
            )
            .unwrap();
            stems.push(serde_json::json!({"name": name, "file": format!("{name}.wav")}));
        }
        fs::write(
            job.join("result.json"),
            serde_json::json!({"version":1,"engine":"demucs-mlx","model":"htdemucs","rate":44100,"stems":stems}).to_string(),
        )
        .unwrap();
    }
    const FOUR: [&str; 4] = ["drums", "bass", "other", "vocals"];
    fn database() -> Database {
        let db = Database::open(Path::new(":memory:")).unwrap();
        db.connection.execute("INSERT INTO tracks(id,path,title,artist,duration,sample_rate,peaks,fingerprint) VALUES(1,'/m/a.wav','A','B',2,44100,'[]','print')", []).unwrap();
        db
    }
    #[test]
    fn results_are_moved_whole_compressed_and_listed_once() {
        let (job, store) = (folder("job"), folder("store"));
        fake_result(&job, 2., 0.4, &FOUR);
        let request = request(2.);
        let stored = store_results(&job, &store, &request, &decode, &mut |_| {}).unwrap();
        assert_eq!(stored.len(), 4);
        let target = store.join("print/htdemucs");
        for item in &stored {
            assert!(item.path.starts_with(&target));
            assert!(item.path.exists());
            assert!(item.identity.size > 0);
        }
        assert!(target.join(MANIFEST).exists());
        let leftovers: Vec<_> = fs::read_dir(&job)
            .unwrap()
            .flatten()
            .map(|e| e.file_name())
            .collect();
        assert_eq!(
            leftovers.len(),
            1,
            "only result.json waits for the job to end: {leftovers:?}"
        );
        if Path::new("/usr/bin/afconvert").exists() {
            assert_eq!(stored[0].path.extension().unwrap(), "flac");
            // A sine compresses well: this is about not keeping what the WAV cost.
            assert!(stored[0].identity.size < 44100 * 8 * 2);
        }
        let db = database();
        let ids = commit(&db, &request, &stored).unwrap();
        assert_eq!(ids.len(), 4);
        // Again: the same rows, not new ones.
        let again = commit(&db, &request, &stored).unwrap();
        assert_eq!(ids, again);
        let library = db.library().unwrap();
        assert_eq!(
            library.tracks.iter().filter(|t| t.kind == "stem").count(),
            4
        );
        assert!(library
            .tracks
            .iter()
            .filter(|t| t.kind == "stem")
            .all(|t| t.parent == Some(1)));
        assert!(db.owned_file(ids[0]).is_some());
    }
    #[test]
    fn loud_stems_are_turned_down_together_never_clipped() {
        let (job, store) = (folder("loud-job"), folder("loud-store"));
        fake_result(&job, 1., 0.5, &FOUR[..3]);
        write_float_wav(&job.join("vocals.wav"), &sine(1., 2.), 44100).unwrap();
        let mut result: serde_json::Value =
            serde_json::from_slice(&fs::read(job.join("result.json")).unwrap()).unwrap();
        result["stems"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"name":"vocals","file":"vocals.wav"}));
        fs::write(job.join("result.json"), result.to_string()).unwrap();
        let stored = store_results(&job, &store, &request(1.), &decode, &mut |_| {}).unwrap();
        let manifest: Manifest =
            serde_json::from_slice(&fs::read(store.join("print/htdemucs").join(MANIFEST)).unwrap())
                .unwrap();
        assert!((manifest.gain - 0.5).abs() < 0.01, "{}", manifest.gain);
        let peak = |index: usize| {
            decode(&stored[index].path)
                .unwrap()
                .samples
                .iter()
                .fold(0_f32, |p, f| p.max(f[0].abs()))
        };
        let vocals = stored.iter().position(|s| s.name == "vocals").unwrap();
        assert!((peak(vocals) - 1.).abs() < 0.02, "{}", peak(vocals));
        let other = stored.iter().position(|s| s.name == "bass").unwrap();
        assert!(
            (peak(other) - 0.25).abs() < 0.02,
            "the rest moved by the same amount"
        );
    }
    type Case = (&'static str, Box<dyn Fn(&Path)>);
    #[test]
    fn a_wrong_result_leaves_nothing_in_the_store() {
        let cases: [Case; 4] = [
            (
                "short",
                Box::new(|job: &Path| fake_result(job, 1., 0.4, &FOUR)),
            ),
            (
                "few",
                Box::new(|job: &Path| fake_result(job, 2., 0.4, &FOUR[..2])),
            ),
            (
                "name",
                Box::new(|job: &Path| {
                    fake_result(job, 2., 0.4, &["drums", "bass", "other", "../evil"])
                }),
            ),
            (
                "bad",
                Box::new(|job: &Path| {
                    fake_result(job, 2., 0.4, &FOUR);
                    fs::write(job.join("bass.wav"), b"not audio").unwrap();
                }),
            ),
        ];
        for (name, make) in cases {
            let (job, store) = (
                folder(&format!("{name}-job")),
                folder(&format!("{name}-store")),
            );
            // A separator in a name fails at writing the fake too; what counts is the store.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| make(&job)));
            assert!(
                store_results(&job, &store, &request(2.), &decode, &mut |_| {}).is_err(),
                "{name}"
            );
            assert_eq!(fs::read_dir(&store).unwrap().count(), 0, "{name}");
        }
    }
    #[test]
    fn a_failure_while_moving_takes_back_what_had_moved() {
        if !Path::new("/usr/bin/afconvert").exists() {
            return;
        }
        let (job, store) = (folder("half-job"), folder("half-store"));
        fake_result(&job, 2., 0.4, &FOUR);
        // Something in the way of the last stem.
        let target = store.join("print/htdemucs");
        fs::create_dir_all(target.join("vocals.flac")).unwrap();
        assert!(store_results(&job, &store, &request(2.), &decode, &mut |_| {}).is_err());
        let left: Vec<_> = fs::read_dir(&target)
            .unwrap()
            .flatten()
            .map(|e| e.file_name())
            .collect();
        assert_eq!(left, ["vocals.flac"], "only what was already there");
    }
    #[test]
    fn nan_is_refused() {
        let (job, store) = (folder("nan-job"), folder("nan-store"));
        fake_result(&job, 2., 0.4, &FOUR);
        let mut samples = sine(2., 0.4);
        samples[500][0] = f32::NAN;
        write_float_wav(&job.join("drums.wav"), &samples, 44100).unwrap();
        assert!(store_results(&job, &store, &request(2.), &decode, &mut |_| {}).is_err());
        assert_eq!(fs::read_dir(&store).unwrap().count(), 0);
    }
    /// A stand-in for Python, to test the process around the separation. It is a `#!/bin/sh`
    /// script, so it and the tests that run it exist on Unix only.
    #[cfg(unix)]
    fn fake_python(name: &str, body: &str) -> PathBuf {
        let path = folder(name).join("python");
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }
    #[cfg(unix)]
    fn runtime(python: &Path) -> Runtime {
        Runtime {
            python: python.to_string_lossy().into(),
            engine: "demucs-mlx".into(),
            version: "1".into(),
        }
    }
    #[test]
    #[cfg(unix)]
    fn the_temporary_folder_goes_whatever_happens() {
        for (name, body, expected) in [
            (
                "fail",
                r#"echo '{"stage":"error","message":"boom"}'; exit 1"#,
                "boom",
            ),
            ("dies", "exit 3", "antes de tiempo"),
            ("noise", "echo not json; exit 0", "antes de tiempo"),
        ] {
            let (store, cache) = (folder(&format!("t-{name}")), folder(&format!("c-{name}")));
            let python = fake_python(&format!("py-{name}"), body);
            let stems = Stems::default();
            let error = separate(
                &stems,
                &runtime(&python),
                &cache,
                &store,
                &request(1.),
                &|input| write_float_wav(input, &sine(1., 0.2), 44100),
                &decode,
                &mut |_| {},
            )
            .err()
            .unwrap();
            assert!(error.contains(expected), "{name}: {error}");
            assert_eq!(fs::read_dir(&store).unwrap().count(), 0, "{name}");
        }
    }
    #[test]
    #[cfg(unix)]
    fn cancelling_ends_the_process_and_cleans_up() {
        let (store, cache) = (folder("cancel"), folder("cancel-cache"));
        let python = fake_python("cancel-py", r#"echo '{"stage":"loading"}'; exec sleep 30"#);
        let stems = Arc::new(Stems::default());
        let other = stems.clone();
        let canceller = std::thread::spawn(move || {
            while other.child.lock().unwrap().is_none() {
                std::thread::sleep(Duration::from_millis(5));
            }
            other.cancel();
        });
        let started = Instant::now();
        let mut seen = Vec::new();
        let error = separate(
            &stems,
            &runtime(&python),
            &cache,
            &store,
            &request(1.),
            &|input| write_float_wav(input, &sine(1., 0.2), 44100),
            &decode,
            &mut |stage| seen.push(stage.to_string()),
        )
        .err()
        .unwrap();
        canceller.join().unwrap();
        assert_eq!(error, CANCELLED);
        assert!(started.elapsed() < Duration::from_secs(10));
        assert_eq!(fs::read_dir(&store).unwrap().count(), 0);
        assert!(stems.child.lock().unwrap().is_none());
    }
    fn stored_set(
        store: &Path,
        db: &Database,
        track: u64,
        key: &str,
        stem_names: &[&str],
    ) -> Vec<u64> {
        let job = folder(&format!("set-{key}"));
        fake_result(&job, 1., 0.3, stem_names);
        let mut request = request(1.);
        request.track = track;
        request.key = key.into();
        request.model = "htdemucs_6s";
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(job.join("result.json")).unwrap()).unwrap();
        let mut extra = vec![];
        for name in ["guitar", "piano"] {
            write_float_wav(&job.join(format!("{name}.wav")), &sine(1., 0.3), 44100).unwrap();
            extra.push(serde_json::json!({"name": name, "file": format!("{name}.wav")}));
        }
        value["stems"].as_array_mut().unwrap().extend(extra);
        fs::write(job.join("result.json"), value.to_string()).unwrap();
        let stored = store_results(&job, store, &request, &decode, &mut |_| {}).unwrap();
        commit(db, &request, &stored).unwrap()
    }
    #[test]
    fn making_room_takes_only_what_nobody_chose_and_the_oldest_first() {
        let store = folder("room");
        let db = database();
        let ids = stored_set(&store, &db, 1, "print", &FOUR);
        let [favorite, tagged, listed, banked, looped, free] = ids[..] else {
            panic!()
        };
        let oldest = free;
        db.connection
            .execute("UPDATE tracks SET favorite=1 WHERE id=?", [favorite])
            .unwrap();
        db.edit(crate::library::LibraryEdit::AddTag {
            track: tagged,
            tag: "x".into(),
        })
        .unwrap();
        db.edit(crate::library::LibraryEdit::CreatePlaylist { name: "p".into() })
            .unwrap();
        db.edit(crate::library::LibraryEdit::AddToPlaylist {
            playlist: 1,
            track: listed,
        })
        .unwrap();
        db.edit(crate::library::LibraryEdit::SaveBank {
            index: 0,
            tracks: vec![Some(banked), None, None, None, None, None, None, None],
        })
        .unwrap();
        db.save_loop(looped, 0., 0.5).unwrap();
        let path = db.owned_file(oldest).unwrap();
        // Kept by the job that just made it.
        assert!(make_room(&db, 0, &HashSet::from([oldest])).unwrap());
        assert!(path.exists());
        // Everything else was chosen, so the limit cannot be met: it says so and deletes nothing more.
        assert!(make_room(&db, 0, &HashSet::new()).unwrap());
        assert!(!path.exists());
        assert!(db.owned_file(oldest).is_none());
        for id in [favorite, tagged, listed, banked, looped] {
            assert!(db.owned_file(id).unwrap().exists(), "{id}");
        }
        // Nothing to do under the limit.
        assert!(!make_room(&db, u64::MAX, &HashSet::new()).unwrap());
        let manifest: Manifest = serde_json::from_slice(
            &fs::read(store.join("print/htdemucs_6s").join(MANIFEST)).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest.stems.len(), 5, "the manifest forgets what went");
    }
    #[test]
    fn a_file_curado_did_not_write_is_never_removed() {
        let root = folder("yours");
        let yours = root.join("mine.flac");
        fs::write(&yours, b"music").unwrap();
        remove_owned_file(&yours).unwrap();
        assert!(yours.exists(), "no manifest");
        fs::write(root.join(MANIFEST), serde_json::to_vec(&serde_json::json!({
            "version":1,"parent_fingerprint":null,"title":"t","artist":"a","model":"m","engine":"e","gain":1.0,
            "stems":[{"name":"x","file":"other.flac","duration":1.0,"rate":44100,"sha256":""}]
        })).unwrap()).unwrap();
        remove_owned_file(&yours).unwrap();
        assert!(yours.exists(), "not listed");
    }
    #[test]
    fn after_a_crash_temporary_files_go_and_lost_stems_come_back_once() {
        let store = folder("tidy");
        let db = database();
        let job = folder("tidy-job");
        fake_result(&job, 1., 0.3, &FOUR);
        let stored = store_results(&job, &store, &request(1.), &decode, &mut |_| {}).unwrap();
        // The library never heard of them, as when a crash came before the commit.
        fs::create_dir_all(job_folder(&store, 9)).unwrap();
        fs::write(job_folder(&store, 9).join("input.wav"), b"x").unwrap();
        let db = Mutex::new(db);
        tidy(&db, &store);
        tidy(&db, &store);
        assert!(!job_folder(&store, 9).exists());
        let library = db.lock().unwrap().library().unwrap();
        let stems: Vec<_> = library.tracks.iter().filter(|t| t.kind == "stem").collect();
        assert_eq!(stems.len(), stored.len());
        assert!(stems.iter().all(|t| t.parent == Some(1)));
    }
    #[test]
    fn asking_for_room_uses_real_numbers() {
        let store = folder("check");
        // Five minutes in four stems, with a margin: temporary files cost about 640 MB, plus a gigabyte.
        let five = room_needed(300., 4);
        assert!((1_700_000_000..1_800_000_000).contains(&five), "{five}");
        let error = check_room(&store, 1e12, 4).unwrap_err();
        assert!(error.contains("espacio"), "{error}");
    }
    #[test]
    fn limits_are_checked() {
        let db = database();
        assert_eq!(limit_gb(&db), DEFAULT_LIMIT_GB);
        set_limit_gb(&db, 5).unwrap();
        assert_eq!(limit_gb(&db), 5);
        assert!(set_limit_gb(&db, 0).is_err());
        assert!(set_limit_gb(&db, 5000).is_err());
    }
    /// `cargo test -p curado --lib real_stems -- --ignored --nocapture` with an installed engine.
    #[test]
    #[ignore = "needs an installed Demucs and its model"]
    fn real_stems() {
        let python = PathBuf::from(std::env::var("CURADO_TEST_PYTHON").unwrap());
        let (store, cache) = (folder("real"), folder("real-cache"));
        let worker = worker_path(&cache).unwrap();
        let found = probe(&python, &worker).expect("an engine");
        println!("{found:?}");
        let seconds = 8.;
        let stems = Stems::default();
        let started = Instant::now();
        let mut seen = Vec::new();
        let stored = separate(
            &stems,
            &found,
            &cache,
            &store,
            &request(seconds),
            &|input| write_float_wav(input, &sine(seconds, 0.3), 44100),
            &decode,
            &mut |stage| seen.push(stage.to_string()),
        )
        .unwrap();
        println!("{seen:?} in {:?}", started.elapsed());
        for item in &stored {
            println!(
                "{} {} bytes {:.2} s",
                item.name, item.identity.size, item.duration
            );
        }
        assert_eq!(stored.len(), 4);
        assert_eq!(
            fs::read_dir(&store).unwrap().flatten().count(),
            1,
            "only the source's folder stays"
        );
    }
}
