mod annotations;
mod demo;
mod library;
mod midi;
mod playback;
mod preferences;
mod session;
mod tuning;
mod visuals;
mod web;
use curado_audio::{
    decode::decode,
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
    data_dir: PathBuf,
    cache_dir: PathBuf,
    queue: Mutex<playback::Queue>,
    theme: Mutex<preferences::Theme>,
    mode: Mutex<session::Mode>,
    annotations: Mutex<annotations::Annotations>,
    persistence_error: Mutex<Option<String>>,
    generation: AtomicU64,
    running: AtomicBool,
    tuning_error: Mutex<Option<String>>,
}
impl AppState {
    fn decode_track(&self, id: u64) -> Result<Arc<curado_audio::decode::Clip>, String> {
        let (path, initial, music) = {
            let db = self.db.lock().unwrap();
            (
                db.path(id)?,
                db.setting::<[f64; 3]>(&format!("cues:{id}"))
                    .unwrap_or([0.; 3]),
                db.music(id),
            )
        };
        let mut clip = curado_audio::decode::decode_with_music(&path, id, music)?;
        if music.is_none() {
            self.db
                .lock()
                .unwrap()
                .save(&format!("music:{id}"), &clip.music)?;
        }
        clip.cues = self.annotations.lock().unwrap().state(id, initial);
        // Tracks imported before schema v3 gain their band waveform on first load.
        let _ = self.db.lock().unwrap().fill_waveform(id, &clip);
        Ok(Arc::new(clip))
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
        let tuning_error = self.tuning_error.lock().unwrap().clone();
        AppSnapshot {
            audio,
            output,
            error,
            midi,
            queue,
            persistence_error,
            mode,
            tuning_error,
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
    #[serde(rename = "tuningError")]
    tuning_error: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct KeyBinding {
    code: String,
    shift: bool,
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
    music.version = 1;
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
        let clip = decode(&path, id)?;
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
    let _guard = s.decode_gate.lock().unwrap();
    let mut report = ImportReport::default();
    for file in files {
        match decode(&file, 0).and_then(|c| s.db.lock().unwrap().insert(&file, &c)) {
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
#[tauri::command]
async fn load_track(
    slot: usize,
    id: u64,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    if slot >= curado_audio::SLOTS {
        return Err("Slot inválido".into());
    }
    let s = state.inner().clone();
    if slot == 0 {
        s.queue.lock().unwrap().enabled = false;
    }
    work(move || {
        let _guard = s.decode_gate.lock().unwrap();
        let generation = s.generation.load(Ordering::Acquire);
        let clip = s.decode_track(id)?;
        let output = s.output.lock().unwrap();
        if s.generation.load(Ordering::Acquire) != generation {
            return Err("El dispositivo cambió durante la carga; vuelve a cargar la pista".into());
        }
        let handle = output.as_ref().ok_or("Motor no disponible")?.handle.clone();
        let ticket = handle.load(slot, clip)?;
        drop(output);
        let started = std::time::Instant::now();
        while !handle.is_loaded(slot, ticket) {
            if started.elapsed() > Duration::from_secs(2) { return Err("El dispositivo no confirmó la carga. Recupera la salida de audio y vuelve a intentar.".into()); }
            std::thread::sleep(Duration::from_millis(2));
        }
        Ok(())
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
    if bindings.len() > 100 {
        return Err("Máximo 100 atajos".into());
    }
    let mut used = std::collections::HashSet::new();
    for binding in &bindings {
        binding.action.validate()?;
        if binding.code.len() > 30 || !used.insert((binding.code.clone(), binding.shift)) {
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
                    if let Some(w) = app.get_webview_window("main") {
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
            if let Some(w) = app.get_webview_window("main") {
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
                data_dir,
                cache_dir,
                queue: Mutex::new(playback::Queue::default()),
                theme: Mutex::new(theme),
                mode: Mutex::new(mode),
                annotations: Mutex::new(annotations::Annotations::default()),
                persistence_error: Mutex::new(None),
                generation: AtomicU64::new(0),
                running: AtomicBool::new(true),
                tuning_error: Mutex::new(None),
            });
            app.manage(state.clone());
            let pitch = state.clone();
            std::thread::Builder::new()
                .name("curado-tuning".into())
                .spawn(move || tuning::run(pitch))?;
            if let Err(error) = tray(app) {
                eprintln!("curado tray unavailable: {error}");
            }
            let handle = app.handle().clone();
            let telemetry = state.clone();
            std::thread::Builder::new()
                .name("curado-telemetry".into())
                .spawn(move || {
                    while telemetry.running.load(Ordering::Relaxed) {
                        let snap = telemetry.status();
                        if let Some(o) = telemetry.output.lock().unwrap().as_ref() {
                            o.handle.collect();
                        }
                        let _ = handle.emit("engine:state", snap);
                        std::thread::sleep(Duration::from_millis(50));
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
                            .map(|o| o.handle.snapshot());
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
                create_demo,
                load_track,
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
