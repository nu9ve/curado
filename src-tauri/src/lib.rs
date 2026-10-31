mod annotations;
mod demo;
mod devices;
mod library;
mod memory;
mod midi;
mod playback;
mod preferences;
mod rekordbox;
mod rekordbox_xml;
mod session;
mod tracking;
mod visuals;
mod web;
use curado_audio::{
    decode::decode_with_limit,
    output::{self, Output, OutputInfo},
    Action, Control, Snapshot,
};
use library::{Database, ImportReport, Library, LibraryEdit};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tauri::{Emitter, Manager};
use web::{close_web_services, open_web_service};

pub struct AppState {
    db: Mutex<Database>,
    output: Mutex<Option<Output>>,
    audio_error: Mutex<Option<String>>,
    decode_gate: Mutex<()>,
    midi: Mutex<midi::Midi>,
    visuals: Mutex<visuals::Visuals>,
    tracking: Arc<tracking::Tracking>,
    data_dir: PathBuf,
    cache_dir: PathBuf,
    queue: Mutex<playback::Queue>,
    theme: Mutex<preferences::Theme>,
    mode: Mutex<session::Mode>,
    annotations: Mutex<annotations::Annotations>,
    persistence_error: Mutex<Option<String>>,
    generation: AtomicU64,
    running: AtomicBool,
    pcm: memory::PcmBudget,
    devices: devices::Devices,
    /// Latest load per slot: an older one still decoding stops and lets the newer one run.
    load_requests: [AtomicU64; curado_audio::SLOTS],
}
impl AppState {
    fn decode_track(&self, id: u64) -> Result<Arc<curado_audio::decode::Clip>, String> {
        self.decode_track_unless(id, &|| false)
    }
    fn decode_track_unless(
        &self,
        id: u64,
        replaced: &dyn Fn() -> bool,
    ) -> Result<Arc<curado_audio::decode::Clip>, String> {
        let (path, (cues, ends), music) = {
            let db = self.db.lock().unwrap();
            (
                db.path(id)?,
                db.saved_cues(id),
                // Estimates from an older analysis are recalculated; manual ones are kept.
                db.music(id).filter(|music| music.is_current()),
            )
        };
        let room = self.pcm.available();
        let mut clip = curado_audio::decode::decode_bounded(
            &path,
            id,
            music,
            room.min(curado_audio::decode::MAX_CLIP_BYTES),
            &|| replaced() || !self.running.load(Ordering::Relaxed),
        )
        .map_err(|error| memory::explain(error, room))?;
        {
            let db = self.db.lock().unwrap();
            if music.is_none() {
                db.save(&format!("music:{id}"), &clip.music)?;
            }
            clip.beats = db.beats(id).map(Into::into);
        }
        clip.cues = self.annotations.lock().unwrap().state(id, cues, ends);
        {
            let db = self.db.lock().unwrap();
            // Tracks imported before schema v3 gain their band waveform on first load.
            let _ = db.fill_waveform(id, &clip);
            db.complete_reference(id, &clip)?;
        }
        let clip = Arc::new(clip);
        self.pcm.track(&clip);
        Ok(clip)
    }
    fn flush_cues(&self) {
        let result = {
            let mut annotations = self.annotations.lock().unwrap();
            let db = self.db.lock().unwrap();
            annotations.flush(&db)
        };
        *self.persistence_error.lock().unwrap() = result.err();
    }

    fn action(&self, action: Action) -> Result<(), String> {
        action.validate()?;
        let mode = self.mode.lock().unwrap();
        if !mode.allows(&action) {
            return Err("Ese control está disponible en el modo Mezcla".into());
        }

        if action.kind == "panic" {
            self.queue.lock().unwrap().enabled = false;
        }
        self.output
            .lock()
            .unwrap()
            .as_ref()
            .ok_or("Motor de audio no disponible")?
            .handle
            .action(action)
    }
    /// For a MIDI button: the value that flips an on/off control.
    fn toggled(&self, action: &Action) -> Option<f64> {
        let output = self.output.lock().unwrap();
        midi::flipped(&output.as_ref()?.handle.snapshot(), action)
    }
    fn status(&self) -> AppSnapshot {
        let (audio, output) = {
            let out = self.output.lock().unwrap();
            (
                out.as_ref().map(|o| o.handle.snapshot()),
                out.as_ref().map(|o| o.info.clone()),
            )
        };
        // Each guard ends before the next service is consulted. In particular,
        // never hold queue while waiting for mode (commands take mode -> queue).
        let error = self.audio_error.lock().unwrap().clone();
        let midi = self.midi.lock().unwrap().state.lock().unwrap().clone();
        let queue = self.queue.lock().unwrap().status();
        let persistence_error = self.persistence_error.lock().unwrap().clone();
        let mode = *self.mode.lock().unwrap();
        let devices = self.devices.list();
        AppSnapshot {
            audio,
            output,
            error,
            midi,
            queue,
            persistence_error,
            mode,
            devices,
        }
    }
}
#[derive(Clone, Serialize)]
struct AppSnapshot {
    audio: Option<Snapshot>,
    output: Option<OutputInfo>,
    error: Option<String>,
    midi: midi::MidiStatus,
    queue: playback::Status,
    #[serde(rename = "persistenceError")]
    persistence_error: Option<String>,
    mode: session::Mode,
    devices: Vec<devices::DeviceInfo>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KeyBinding {
    code: String,
    shift: bool,
    #[serde(default)]
    ctrl: bool,
    #[serde(default)]
    alt: bool,
    action: Action,
}
async fn work<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn set_mode(
    mode: session::Mode,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let s = state.inner().clone();
    work(move || {
        let mut current = s.mode.lock().unwrap();
        if *current == mode {
            return Ok(());
        }
        let mut queue = s.queue.lock().unwrap();
        let output = s.output.lock().unwrap();
        if let Some(output) = output.as_ref() {
            output
                .handle
                .control(0, Control::SetMode(mode == session::Mode::Mix))?;
        }
        queue.enabled = false;
        *current = mode;
        drop(output);
        drop(queue);
        drop(current);
        s.db.lock().unwrap().save("mode", &mode)
    })
    .await
}
#[tauri::command]
async fn get_theme(state: tauri::State<'_, Arc<AppState>>) -> Result<preferences::Theme, String> {
    Ok(state.theme.lock().unwrap().clone())
}
#[tauri::command]
async fn save_theme(
    mut theme: preferences::Theme,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    theme.validate()?;
    theme.base = theme.base.to_ascii_lowercase();
    theme.version = 2;
    let s = state.inner().clone();
    work(move || {
        s.db.lock().unwrap().save("theme", &theme)?;
        *s.theme.lock().unwrap() = theme;
        Ok(())
    })
    .await
}
#[tauri::command]
async fn status(state: tauri::State<'_, Arc<AppState>>) -> Result<AppSnapshot, String> {
    let s = state.inner().clone();
    work(move || Ok(s.status())).await
}
#[tauri::command]
fn audio_action(action: Action, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    state.action(action)
}
/// Raw bytes (`analysis::encode`) reach JS as an ArrayBuffer, avoiding a JSON number array.
#[tauri::command]
async fn track_waveform(
    id: u64,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<tauri::ipc::Response, String> {
    let s = state.inner().clone();
    work(move || s.db.lock().unwrap().waveform(id))
        .await
        .map(tauri::ipc::Response::new)
}
#[tauri::command]
async fn get_library(state: tauri::State<'_, Arc<AppState>>) -> Result<Library, String> {
    let s = state.inner().clone();
    work(move || s.db.lock().unwrap().library()).await
}
#[tauri::command]
async fn save_track_music(
    id: u64,
    mut music: curado_audio::music::MusicAnalysis,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Library, String> {
    if !music.valid() {
        return Err("BPM, primer beat o tonalidad inválidos".into());
    }
    music.version = curado_audio::music::ANALYSIS_VERSION;
    music.manual = true;
    if let Some(bpm) = music.bpm {
        music.beat_offset = music.beat_offset.rem_euclid(60. / bpm);
    }
    let s = state.inner().clone();
    work(move || {
        let db = s.db.lock().unwrap();
        db.path(id)?;
        db.save(&format!("music:{id}"), &music)?;
        let library = db.library()?;
        drop(db);
        if let Some(output) = s.output.lock().unwrap().as_ref() {
            output.handle.set_music(id, music)?;
        }
        Ok(library)
    })
    .await
}
#[tauri::command]
async fn analyze_track(id: u64, state: tauri::State<'_, Arc<AppState>>) -> Result<Library, String> {
    let s = state.inner().clone();
    work(move || {
        let _decode = s.decode_gate.lock().unwrap();
        let path = s.db.lock().unwrap().path(id)?;
        let room = s.pcm.available();
        let clip = decode_with_limit(
            &path,
            id,
            None,
            room.min(curado_audio::decode::MAX_CLIP_BYTES),
        )
        .map_err(|error| memory::explain(error, room))?;
        let db = s.db.lock().unwrap();
        db.save(&format!("music:{id}"), &clip.music)?;
        let library = db.library()?;
        drop(db);
        if let Some(output) = s.output.lock().unwrap().as_ref() {
            output.handle.set_music(id, clip.music)?;
        }
        Ok(library)
    })
    .await
}
#[tauri::command]
async fn edit_library(
    edit: LibraryEdit,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Library, String> {
    let s = state.inner().clone();
    work(move || {
        let db = s.db.lock().unwrap();
        db.edit(edit)?;
        db.library()
    })
    .await
}
fn import_paths(s: &AppState, files: Vec<PathBuf>) -> ImportReport {
    let mut report = ImportReport::default();
    for file in files {
        // Per file, so that a load in the middle of a large import waits for one track at most.
        let _guard = s.decode_gate.lock().unwrap();
        let room = s.pcm.available();
        let decoded = decode_with_limit(
            &file,
            0,
            None,
            room.min(curado_audio::decode::MAX_CLIP_BYTES),
        )
        .map_err(|error| memory::explain(error, room));
        match decoded.and_then(|c| s.db.lock().unwrap().insert(&file, &c)) {
            Ok(true) => report.added += 1,
            Ok(false) => report.skipped += 1,
            Err(e) => {
                if report.errors.len() < 30 {
                    report.errors.push(format!(
                        "{}: {e}",
                        file.file_name().unwrap_or_default().to_string_lossy()
                    ));
                }
            }
        }
    }
    report
}
#[tauri::command]
async fn import_media(
    folder: bool,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<ImportReport, String> {
    let s = state.inner().clone();
    let dialog = rfd::AsyncFileDialog::new()
        .set_title("Importar música a curado")
        .add_filter(
            "Audio",
            &[
                "mp3", "wav", "flac", "aif", "aiff", "m4a", "aac", "ogg", "caf",
            ],
        );
    let roots = if folder {
        dialog
            .pick_folder()
            .await
            .map(|f| vec![f.path().to_path_buf()])
    } else {
        dialog.pick_files().await.map(|fs| {
            fs.into_iter()
                .map(|f| f.path().to_path_buf())
                .collect::<Vec<_>>()
        })
    };
    work(move || {
        let Some(roots) = roots else {
            return Ok(ImportReport::default());
        };
        let mut files = Vec::new();
        for root in roots {
            library::walk(root, &mut files)?;
        }
        Ok(import_paths(&s, files))
    })
    .await
}
#[tauri::command]
async fn create_demo(state: tauri::State<'_, Arc<AppState>>) -> Result<Library, String> {
    let s = state.inner().clone();
    work(move || {
        let files = demo::create(&s.data_dir.join("demo"))?;
        let report = import_paths(&s, files);
        if !report.errors.is_empty() {
            return Err(report.errors.join("\n"));
        }
        s.db.lock().unwrap().library()
    })
    .await
}
/// Decodes and installs a track. `Ok(None)` means a newer load for this slot replaced it.
fn load_into_slot(
    s: &AppState,
    slot: usize,
    request: u64,
    resolve: impl FnOnce() -> Result<u64, String>,
) -> Result<Option<u64>, String> {
    let replaced = || s.load_requests[slot].load(Ordering::SeqCst) != request;
    let _guard = s.decode_gate.lock().unwrap();
    if replaced() {
        return Ok(None);
    }
    let generation = s.generation.load(Ordering::Acquire);
    let id = resolve()?;
    let clip = match s.decode_track_unless(id, &replaced) {
        Err(error) if error == curado_audio::decode::CANCELLED => return Ok(None),
        other => other?,
    };
    let output = s.output.lock().unwrap();
    if s.generation.load(Ordering::Acquire) != generation {
        return Err("El dispositivo cambió durante la carga; vuelve a cargar la pista".into());
    }
    let handle = output.as_ref().ok_or("Motor no disponible")?.handle.clone();
    let ticket = handle.load(slot, clip)?;
    drop(output);
    let started = std::time::Instant::now();
    while !handle.is_loaded(slot, ticket) {
        if started.elapsed() > Duration::from_secs(2) {
            return Err("El dispositivo no confirmó la carga. Recupera la salida de audio y vuelve a intentar.".into());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    Ok(Some(id))
}
fn start_load(s: &AppState, slot: usize) -> Result<u64, String> {
    if slot >= curado_audio::SLOTS {
        return Err("Slot inválido".into());
    }
    if slot == 0 {
        s.queue.lock().unwrap().enabled = false;
    }
    Ok(s.load_requests[slot].fetch_add(1, Ordering::SeqCst) + 1)
}
/// Returns false when a newer load for this slot replaced this one.
#[tauri::command]
async fn load_track(
    slot: usize,
    id: u64,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<bool, String> {
    let s = state.inner().clone();
    let request = start_load(&s, slot)?;
    work(move || Ok(load_into_slot(&s, slot, request, || Ok(id))?.is_some())).await
}
#[tauri::command]
async fn device_library(
    device: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<rekordbox::Export, String> {
    let s = state.inner().clone();
    work(move || Ok((*s.devices.export(&device)?).clone())).await
}
#[tauri::command]
async fn device_browse(
    device: String,
    folder: Option<u64>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<devices::DeviceFolder, String> {
    let s = state.inner().clone();
    work(move || s.devices.browse(&device, folder)).await
}
/// Loads a track from a drive's rekordbox library, with its grid, key and cues.
#[tauri::command]
async fn device_load_track(
    slot: usize,
    device: String,
    track: u32,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Option<u64>, String> {
    let s = state.inner().clone();
    let request = start_load(&s, slot)?;
    work(move || {
        load_into_slot(&s, slot, request, || {
            let export = s.devices.export(&device)?;
            let track = export
                .tracks
                .iter()
                .find(|candidate| candidate.id == track)
                .ok_or("La pista ya no está en el dispositivo")?;
            let root = s.devices.root(&device)?;
            let mut annotations = s.annotations.lock().unwrap();
            let db = s.db.lock().unwrap();
            devices::import_track(&db, &mut annotations, &root, track)
        })
    })
    .await
}
/// Loads a file found browsing a drive. Takes the listing's entry, never a path.
#[tauri::command]
async fn device_load_file(
    slot: usize,
    entry: u64,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Option<u64>, String> {
    let s = state.inner().clone();
    let request = start_load(&s, slot)?;
    work(move || {
        load_into_slot(&s, slot, request, || {
            let path = s.devices.file(entry)?;
            devices::register_file(&s.db.lock().unwrap(), &path)
        })
    })
    .await
}
/// Opt-in per track: finds whether its tempo moves and keeps the beats, or removes them. Takes
/// effect the next time the track loads, because PCM on a deck never changes under it.
#[tauri::command]
async fn track_follow_tempo(
    id: u64,
    enabled: bool,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<bool, String> {
    let s = state.inner().clone();
    work(move || {
        if !enabled {
            s.db.lock().unwrap().forget(&format!("beats:{id}"))?;
            return Ok(false);
        }
        let (path, music) = {
            let db = s.db.lock().unwrap();
            (db.path(id)?, db.music(id))
        };
        let music = music.ok_or("Analiza la pista primero")?;
        let clip = curado_audio::decode::decode(&path, id)?;
        let beats = curado_audio::music::follow_tempo(&clip.samples, clip.sample_rate, &music);
        let db = s.db.lock().unwrap();
        match beats {
            Some(beats) => {
                db.save(&format!("beats:{id}"), &beats)?;
                Ok(true)
            }
            None => {
                db.forget(&format!("beats:{id}"))?;
                Ok(false)
            }
        }
    })
    .await
}
/// Starts the camera helper. Everything it reports goes to the interface as events. Not async,
/// like the actions: a stop and the start after it must run in the order they were asked.
#[tauri::command]
fn tracking_start(
    mode: tracking::Mode,
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let program = tracking::helper(&state.cache_dir)?;
    state.tracking.start(program, mode, move |frame| {
        let _ = app.emit("tracking:frame", frame);
    });
    Ok(())
}
#[tauri::command]
fn tracking_stop(state: tauri::State<'_, Arc<AppState>>) {
    state.tracking.stop();
}
#[tauri::command]
fn tracking_preview(on: bool, state: tauri::State<'_, Arc<AppState>>) {
    state.tracking.set_preview(on);
}
#[tauri::command]
async fn track_beats(
    id: u64,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Option<Vec<f64>>, String> {
    let s = state.inner().clone();
    work(move || Ok(s.db.lock().unwrap().beats(id))).await
}
#[tauri::command]
async fn track_cue_info(
    id: u64,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<devices::CueInfo, String> {
    let s = state.inner().clone();
    work(move || Ok(devices::current_cue_info(&s.db.lock().unwrap(), id))).await
}
/// The destination comes from the native dialog alone: the interface cannot name a path.
#[tauri::command]
async fn export_rekordbox_xml(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<rekordbox_xml::ExportReport, String> {
    let s = state.inner().clone();
    let file = rfd::AsyncFileDialog::new()
        .set_title("Exportar a rekordbox")
        .set_file_name("curado-rekordbox.xml")
        .add_filter("rekordbox XML", &["xml"])
        .save_file()
        .await;
    let Some(file) = file else {
        return Ok(rekordbox_xml::ExportReport::default());
    };
    work(move || {
        s.flush_cues();
        let plan = rekordbox_xml::plan(&s.db.lock().unwrap())?;
        rekordbox_xml::export(plan, file.path())
    })
    .await
}
#[tauri::command]
async fn audio_devices() -> Result<Vec<String>, String> {
    work(output::devices).await
}
#[tauri::command]
async fn configure_audio(
    device: Option<String>,
    frames: Option<u32>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let s = state.inner().clone();
    work(move || {
        let _decode = s.decode_gate.lock().unwrap();
        s.queue.lock().unwrap().enabled = false;
        // Create first: a failed configuration leaves the previous output alive.
        let output = output::start(device, frames)?;
        let mode = s.mode.lock().unwrap();
        output
            .handle
            .control(0, Control::SetMode(*mode == session::Mode::Mix))?;
        let mut current = s.output.lock().unwrap();
        *current = Some(output);
        s.generation.fetch_add(1, Ordering::Release);
        *s.audio_error.lock().unwrap() = None;
        Ok(())
    })
    .await
}
#[tauri::command]
async fn get_keyboard(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Option<Vec<KeyBinding>>, String> {
    let s = state.inner().clone();
    work(move || Ok(s.db.lock().unwrap().setting("keyboard"))).await
}
#[tauri::command]
async fn save_keyboard(
    bindings: Vec<KeyBinding>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    if bindings.len() > 120 {
        return Err("Máximo 120 atajos".into());
    }
    let mut used = std::collections::HashSet::new();
    for binding in &bindings {
        // These two move through a list in the interface and never reach the engine.
        if !matches!(binding.action.kind.as_str(), "listMove" | "listLoad") {
            binding.action.validate()?;
        }
        let combination = (
            binding.code.clone(),
            binding.shift,
            binding.ctrl,
            binding.alt,
        );
        if binding.code.len() > 30 || !used.insert(combination) {
            return Err("Atajo duplicado o inválido".into());
        }
    }
    let s = state.inner().clone();
    work(move || s.db.lock().unwrap().save("keyboard", &bindings)).await
}
#[tauri::command]
async fn midi_ports() -> Result<Vec<String>, String> {
    work(midi::Midi::ports).await
}
#[tauri::command]
async fn midi_connect(
    name: Option<String>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let s = state.inner().clone();
    work(move || s.midi.lock().unwrap().connect(name)).await
}
#[tauri::command]
fn midi_learn(
    action: Option<Action>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    if let Some(a) = &action {
        a.validate()?;
    }
    let midi = state.midi.lock().unwrap();
    let mut m = midi.state.lock().unwrap();
    if action.is_some() && m.connected.is_none() {
        return Err("Conecta primero un dispositivo MIDI".into());
    }
    m.learning = action;
    Ok(())
}
#[tauri::command]
async fn midi_remove(index: usize, state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    let s = state.inner().clone();
    work(move || {
        let midi = s.midi.lock().unwrap();
        let mut m = midi.state.lock().unwrap();
        if index >= m.bindings.len() {
            return Err("Mapeo no encontrado".into());
        }
        m.bindings.remove(index);
        s.db.lock().unwrap().save("midi", &m.bindings)
    })
    .await
}
#[tauri::command]
async fn open_visuals(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    let s = state.inner().clone();
    work(move || s.visuals.lock().unwrap().open(&s.cache_dir)).await
}

#[tauri::command]
async fn play_queue(
    ids: Vec<u64>,
    repeat: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    if ids.is_empty() || ids.len() > 2000 || !["off", "one", "all"].contains(&repeat.as_str()) {
        return Err("Cola inválida".into());
    }
    let s = state.inner().clone();
    work(move || {
        let _guard = s.decode_gate.lock().unwrap();
        let id = ids[0];
        let clip = s.decode_track(id)?;
        let mode = s.mode.lock().unwrap();
        if *mode != session::Mode::Player {
            return Err("Cambia al modo Reproductor para iniciar una cola".into());
        }
        let mut q = s.queue.lock().unwrap();
        let output = s.output.lock().unwrap();
        let handle = &output.as_ref().ok_or("Motor no disponible")?.handle;
        // Player mode already owns crossfader=0. Loading and playing must be indivisible.
        handle.load_and_play(0, clip)?;
        q.set(ids, repeat);
        Ok(())
    })
    .await
}
#[tauri::command]
fn stop_queue(state: tauri::State<'_, Arc<AppState>>) {
    state.queue.lock().unwrap().enabled = false;
}

fn tray(app: &tauri::App) -> tauri::Result<()> {
    use tauri::{
        menu::{Menu, MenuItem, PredefinedMenuItem},
        tray::TrayIconBuilder,
    };
    let show = MenuItem::with_id(app, "show", "Abrir curado", true, None::<&str>)?;
    let play = MenuItem::with_id(app, "play", "Play / pausa · Deck A", true, None::<&str>)?;
    let stop = MenuItem::with_id(app, "stop", "Silenciar todo", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Salir de curado", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &show,
            &play,
            &stop,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;
    let mut builder = TrayIconBuilder::with_id("curado")
        .tooltip("curado · música en tus manos")
        .menu(&menu)
        .on_menu_event(|app, event| {
            let s = app.state::<Arc<AppState>>();
            match event.id.as_ref() {
                "show" => {
                    if let Some(w) = app.get_window("main") {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
                "play" => {
                    if let Some(o) = s.output.lock().unwrap().as_ref() {
                        let _ = o.handle.control(0, Control::Toggle);
                    }
                }
                "stop" => {
                    s.queue.lock().unwrap().enabled = false;
                    if let Some(o) = s.output.lock().unwrap().as_ref() {
                        let _ = o.handle.control(0, Control::Panic);
                    }
                }
                "quit" => {
                    s.running.store(false, Ordering::Relaxed);
                    app.exit(0);
                }
                _ => {}
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    Ok(())
}
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(w) = app.get_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let cache_dir = app.path().app_cache_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            std::fs::create_dir_all(&cache_dir)?;
            let db =
                Database::open(&data_dir.join("curado.sqlite")).map_err(std::io::Error::other)?;
            let bindings = db.setting("midi").unwrap_or_default();
            let mode = db.setting::<session::Mode>("mode").unwrap_or_default();
            let theme = db
                .setting::<preferences::Theme>("theme")
                .filter(|t| t.validate().is_ok())
                .unwrap_or_default();
            let (output, error) = match output::start(None, None) {
                Ok(o) => (Some(o), None),
                Err(e) => (None, Some(e)),
            };
            if let Some(out) = &output {
                let _ = out
                    .handle
                    .control(0, Control::SetMode(mode == session::Mode::Mix));
            }
            let state = Arc::new(AppState {
                db: Mutex::new(db),
                output: Mutex::new(output),
                audio_error: Mutex::new(error),
                decode_gate: Mutex::new(()),
                midi: Mutex::new(midi::Midi::new(bindings)),
                visuals: Mutex::new(visuals::Visuals::new()),
                tracking: Arc::default(),
                data_dir,
                cache_dir,
                queue: Mutex::new(playback::Queue::default()),
                theme: Mutex::new(theme),
                mode: Mutex::new(mode),
                annotations: Mutex::new(annotations::Annotations::default()),
                persistence_error: Mutex::new(None),
                generation: AtomicU64::new(0),
                running: AtomicBool::new(true),
                pcm: Default::default(),
                devices: Default::default(),
                load_requests: Default::default(),
            });
            app.manage(state.clone());
            let drives = state.clone();
            std::thread::Builder::new()
                .name("curado-devices".into())
                .spawn(move || {
                    while drives.running.load(Ordering::Relaxed) {
                        drives.devices.refresh();
                        std::thread::sleep(Duration::from_secs(2));
                    }
                })?;
            if let Err(error) = tray(app) {
                eprintln!("curado tray unavailable: {error}");
            }
            let handle = app.handle().clone();
            let telemetry = state.clone();
            std::thread::Builder::new()
                .name("curado-telemetry".into())
                .spawn(move || {
                    // Three position updates per full state: about 60 Hz, and only while a deck
                    // moves, against 20 Hz for everything else.
                    let mut tick = 0_u32;
                    while telemetry.running.load(Ordering::Relaxed) {
                        if tick.is_multiple_of(3) {
                            let snap = telemetry.status();
                            if let Some(o) = telemetry.output.lock().unwrap().as_ref() {
                                o.handle.collect();
                            }
                            let _ = handle.emit("engine:state", snap);
                        } else {
                            let live = telemetry
                                .output
                                .lock()
                                .unwrap()
                                .as_ref()
                                .and_then(|o| o.handle.live_positions());
                            if let Some(positions) = live {
                                let _ = handle.emit("engine:position", positions);
                            }
                        }
                        tick = tick.wrapping_add(1);
                        std::thread::sleep(Duration::from_millis(16));
                    }
                })?;
            let autosave = state.clone();
            std::thread::Builder::new()
                .name("curado-autosave".into())
                .spawn(move || {
                    while autosave.running.load(Ordering::Relaxed) {
                        autosave.flush_cues();
                        std::thread::sleep(Duration::from_millis(50));
                    }
                })?;
            let playback = state.clone();
            std::thread::Builder::new()
                .name("curado-playback".into())
                .spawn(move || {
                    while playback.running.load(Ordering::Relaxed) {
                        playback::tick(&playback);
                        std::thread::sleep(Duration::from_millis(50));
                    }
                })?;
            let input = state.clone();
            std::thread::Builder::new()
                .name("curado-midi".into())
                .spawn(move || {
                    while input.running.load(Ordering::Relaxed) {
                        input.midi.lock().unwrap().tick(&input);
                        std::thread::sleep(Duration::from_millis(2));
                    }
                })?;
            std::thread::Builder::new()
                .name("curado-visuals".into())
                .spawn(move || {
                    while state.running.load(Ordering::Relaxed) {
                        let audio = state
                            .output
                            .lock()
                            .unwrap()
                            .as_ref()
                            .map(|o| o.handle.clone());
                        if let Some(audio) = audio {
                            let theme = state.theme.lock().unwrap().clone();
                            state.visuals.lock().unwrap().send(&audio, &theme);
                        }
                        std::thread::sleep(Duration::from_millis(50));
                    }
                })?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // Whether or not the interface gets to run first, leaving the window frees the camera.
            if window.label() == "main" {
                if let tauri::WindowEvent::Focused(false)
                | tauri::WindowEvent::CloseRequested { .. } = event
                {
                    if let Some(state) = window.try_state::<Arc<AppState>>() {
                        state.tracking.stop();
                    }
                }
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if web::hide_on_close(
                    window.label(),
                    window.app_handle().tray_by_id("curado").is_some(),
                ) {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler({
            let handler: fn(tauri::ipc::Invoke) -> bool = tauri::generate_handler![
                status,
                audio_action,
                get_library,
                track_waveform,
                edit_library,
                import_media,
                export_rekordbox_xml,
                track_beats,
                tracking_start,
                tracking_stop,
                tracking_preview,
                track_follow_tempo,
                create_demo,
                load_track,
                device_library,
                device_browse,
                device_load_track,
                device_load_file,
                track_cue_info,
                audio_devices,
                configure_audio,
                get_keyboard,
                save_keyboard,
                midi_ports,
                midi_connect,
                midi_learn,
                midi_remove,
                open_visuals,
                play_queue,
                stop_queue,
                get_theme,
                save_theme,
                set_mode,
                save_track_music,
                analyze_track,
                open_web_service,
                close_web_services
            ];
            move |invoke: tauri::ipc::Invoke| {
                if !web::allows_native_commands(invoke.message.webview_ref().label()) {
                    invoke
                        .resolver
                        .reject("Esta ventana web no tiene acceso a comandos nativos");
                    return true;
                }
                handler(invoke)
            }
        })
        .build(tauri::generate_context!())
        .expect("No se pudo iniciar curado")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                let state = app.state::<Arc<AppState>>();
                state.running.store(false, Ordering::Relaxed);
                state.flush_cues();
            }
        });
}
