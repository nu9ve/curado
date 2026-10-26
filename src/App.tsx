import { useCallback, useEffect, useRef, useState } from 'react';
import {
  AudioLines,
  Library as LibraryIcon,
  Star,
  ListMusic,
  Plus,
  Settings2,
  Keyboard,
  Headphones,
  SlidersHorizontal,
  Radio,
  ArrowUpRight,
  X,
  LoaderCircle,
  Play,
  Pause,
  SkipBack,
  SkipForward,
  ChevronRight,
} from 'lucide-react';
import { connectTheme } from './lib/theme-store';
import {
  call,
  connect,
  control,
  getSnapshot,
  native,
  parameter,
  useSnapshot,
  useMode,
  setSessionMode,
} from './lib/bridge';
import type { ImportReport, KeyBinding, Library as LibraryData, LibraryEdit } from './lib/types';
import { EMPTY_SLOT, time } from './lib/types';
import { DEFAULT_KEYS, matchKey } from './lib/keyboard';
import { Deck } from './components/Deck';
import { Mixer } from './components/Mixer';
import { Library } from './components/Library';
import { Visual } from './components/Visual';
import { Settings } from './components/Settings';
import { Fader, Key, Meter } from './components/Controls';
import { Waveform } from './components/Waveform';
export default function App() {
  const [library, setLibrary] = useState<LibraryData>({ tracks: [], playlists: [] });
  const [view, setView] = useState('all');
  const mode = useMode();
  const [keys, setKeys] = useState(DEFAULT_KEYS);
  const [selected, setSelected] = useState<number | null>(null);
  const [settings, setSettings] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);
  const [loading, setLoading] = useState<number[]>([]);
  const [notice, setNotice] = useState<{ text: string; error: boolean } | null>(null);
  const [newPlaylist, setNewPlaylist] = useState(false);
  const [playlistName, setPlaylistName] = useState('');
  const [editingPlaylist, setEditingPlaylist] = useState<number | null>(null);
  const noticeTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const notify = useCallback((text: string, error = false) => {
    setNotice({ text, error });
    clearTimeout(noticeTimer.current);
    noticeTimer.current = setTimeout(() => setNotice(null), error ? 14000 : 7000);
  }, []);
  const onError = useCallback(
    (e: unknown) => notify(e instanceof Error ? e.message : String(e), true),
    [notify],
  );
  useEffect(() => {
    void connectTheme().catch(onError);
    let disposed = false;
    let off: undefined | (() => void);
    connect()
      .then((fn) => {
        if (disposed) fn();
        else off = fn;
      })
      .catch(onError);
    if (native) {
      call<LibraryData>('get_library')
        .then((data) => {
          if (!disposed) setLibrary(data);
        })
        .catch(onError);
      call<KeyBinding[] | null>('get_keyboard')
        .then((data) => {
          if (data && !disposed) setKeys(data);
        })
        .catch(onError);
    }
    return () => {
      disposed = true;
      off?.();
      clearTimeout(noticeTimer.current);
    };
  }, [onError]);
  const saveKeys = useCallback(async (bindings: KeyBinding[]) => {
    await call('save_keyboard', { bindings });
    setKeys(bindings);
  }, []);
  useEffect(() => {
    if (settings || newPlaylist) return;
    const handle = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      if (target.closest('input,textarea,select,[contenteditable="true"],[role="slider"]')) return;
      if (target.closest('button') && ['Space', 'Enter'].includes(e.code)) return;
      const binding = matchKey(e, keys, mode);
      if (binding) {
        e.preventDefault();
        call('audio_action', { action: binding.action }).catch(onError);
      }
    };
    window.addEventListener('keydown', handle);
    return () => window.removeEventListener('keydown', handle);
  }, [keys, mode, settings, newPlaylist, onError]);
  const load = useCallback(
    async (slot: number, id: number) => {
      setLoading((prev) => [...prev, slot]);
      try {
        await call('load_track', { slot, id });
        notify(`Pista lista en ${slot < 2 ? `deck ${slot ? 'B' : 'A'}` : `sample ${slot - 1}`}.`);
      } catch (e) {
        onError(e);
      } finally {
        setLoading((prev) => prev.filter((i) => i !== slot));
      }
    },
    [notify, onError],
  );
  const edit = useCallback(
    async (edit: LibraryEdit) => {
      try {
        const data = await call<LibraryData>('edit_library', { edit });
        setLibrary(data);
        if (edit.kind === 'deletePlaylist') setView('all');
        return true;
      } catch (e) {
        onError(e);
        return false;
      }
    },
    [onError],
  );
  const importMedia = useCallback(
    async (folder: boolean) => {
      setBusy('Importando música…');
      try {
        const report = await call<ImportReport>('import_media', { folder });
        setLibrary(await call<LibraryData>('get_library'));
        if (report.added || report.skipped || report.errors.length)
          notify(
            `${report.added} pistas añadidas · ${report.skipped} existentes${report.errors.length ? ` · ${report.errors.join('\n')}` : ''}`,
            !!report.errors.length,
          );
      } catch (e) {
        onError(e);
      } finally {
        setBusy(null);
      }
    },
    [notify, onError],
  );
  const demo = async () => {
    setBusy('Preparando sesión de prueba…');
    try {
      const data = await call<LibraryData>('create_demo');
      setLibrary(data);
      const demos = data.tracks
        .filter((t) => t.path.includes('/demo/') || t.path.includes('\\demo\\'))
        .sort((a, b) => a.title.localeCompare(b.title));
      for (let i = 0; i < Math.min(6, demos.length); i++)
        await call('load_track', { slot: i, id: demos[i].id });
      notify('Sesión de prueba lista. Dale Play; cambia a Mezcla para usar ambos decks y samples.');
    } catch (e) {
      onError(e);
    } finally {
      setBusy(null);
    }
  };
  const changeMode = async (next: 'player' | 'mix') => {
    if (next === mode) return;
    try {
      await setSessionMode(next);
    } catch (e) {
      onError(e);
    }
  };
  return (
    <div className="app-shell">
      <div className="window-drag-region" data-tauri-drag-region aria-hidden="true" />
      <aside className="sidebar">
        <a className="brand" href="#" onClick={(e) => e.preventDefault()}>
          <span className="brand-icon">
            <AudioLines size={23} />
          </span>
          curado<span className="brand-period">.</span>
        </a>
        <span className="sidebar-label">HECHO PARA ESCUCHAR</span>
        <div className="side-nav">
          <span className="eyebrow">COLECCIÓN</span>
          <button className={view === 'all' ? 'active' : ''} onClick={() => setView('all')}>
            <LibraryIcon size={17} />
            Toda la música<span>{library.tracks.length}</span>
          </button>
          <button
            className={view === 'favorites' ? 'active' : ''}
            onClick={() => setView('favorites')}
          >
            <Star size={17} />
            Favoritos<span>{library.tracks.filter((t) => t.favorite).length}</span>
          </button>
        </div>
        <div className="side-nav playlists">
          <div className="section-line">
            <span className="eyebrow">PLAYLISTS</span>
            <button
              className="icon-button"
              aria-label="Crear playlist"
              onClick={() => setNewPlaylist(true)}
            >
              <Plus size={15} />
            </button>
          </div>
          {library.playlists.map((p) => (
            <button
              key={p.id}
              className={view === `playlist:${p.id}` ? 'active' : ''}
              onClick={() => setView(`playlist:${p.id}`)}
              title="Doble clic para renombrar"
              onDoubleClick={() => {
                setEditingPlaylist(p.id);
                setPlaylistName(p.name);
                setNewPlaylist(true);
              }}
            >
              <ListMusic size={16} />
              <span className="playlist-name">{p.name}</span>
              <span>{p.tracks.length}</span>
            </button>
          ))}
          {!library.playlists.length && (
            <p className="sidebar-hint">
              Un lugar para cada mood.
              <br />
              Crea tu primera playlist.
            </p>
          )}
          <button className="add-playlist" onClick={() => setNewPlaylist(true)}>
            <Plus size={14} />
            Nueva playlist
          </button>
        </div>
        <div className="sidebar-bottom">
          <div className="demo-card">
            <span className="tiny-disc">
              <AudioLines size={18} />
            </span>
            <h3>Explora el sonido.</h3>
            <p>Dos tracks originales y cuatro samples para probar tu setup.</p>
            <button disabled={!!busy} onClick={demo}>
              Cargar sesión de prueba
              <ArrowUpRight size={14} />
            </button>
          </div>
          <button onClick={() => setSettings(true)}>
            <Settings2 size={16} />
            Configuración<span>⌘</span>
          </button>
          <span className="build-label">CURADO 0.1 · LOCAL FIRST</span>
        </div>
      </aside>
      <main>
        <header className="topbar" data-tauri-drag-region>
          <div className="breadcrumb" data-tauri-drag-region>
            Tu espacio
            <ChevronRight size={12} />
            <b>{mode === 'player' ? 'Reproductor' : 'Estudio de mezcla'}</b>
          </div>
          <div className="mode-switch" role="group" aria-label="Modo de uso">
            <button
              className={mode === 'player' ? 'active' : ''}
              onClick={() => changeMode('player')}
            >
              <Headphones size={14} />
              Reproductor
            </button>
            <button className={mode === 'mix' ? 'active' : ''} onClick={() => changeMode('mix')}>
              <SlidersHorizontal size={14} />
              Mezcla
            </button>
          </div>
          <div className="header-right">
            <ConnectionStatus />
            <button
              className="icon-button"
              aria-label="Abrir configuración de teclado y MIDI"
              onClick={() => setSettings(true)}
            >
              <Keyboard size={17} />
            </button>
          </div>
        </header>
        <div className="main-content">
          <PersistenceNotice />
          <div className="page-intro">
            <div>
              <span className="eyebrow">MENOS RUIDO. MÁS MÚSICA.</span>
              <h1>
                {mode === 'player' ? 'Encuentra tu frecuencia.' : 'Todo el control. Tu mezcla.'}
              </h1>
            </div>
            <span className="session-label">
              <span className="status-dot active" />
              SESIÓN LOCAL
            </span>
          </div>
          {!native && (
            <div className="notice">
              Vista previa de la interfaz · el audio y las integraciones requieren la app de
              escritorio.
            </div>
          )}
          {mode === 'mix' ? (
            <>
              <div className="decks">
                <Deck
                  index={0}
                  tracks={library.tracks}
                  keys={keys}
                  onError={onError}
                  loading={loading.includes(0)}
                />
                <Deck
                  index={1}
                  tracks={library.tracks}
                  keys={keys}
                  onError={onError}
                  loading={loading.includes(1)}
                />
              </div>
              <Mixer
                tracks={library.tracks}
                keys={keys}
                onError={onError}
                onLoad={load}
                selected={selected}
              />
            </>
          ) : (
            <div className="listen-stage">
              <Player tracks={library.tracks} onError={onError} />
              <Visual onError={onError} />
            </div>
          )}
          <Library
            data={library}
            view={view}
            selected={selected}
            onSelect={setSelected}
            onLoad={load}
            onEdit={edit}
            onImport={importMedia}
            onQueue={(ids, repeat) =>
              call('play_queue', { ids, repeat })
                .then(() =>
                  notify(
                    repeat === 'off'
                      ? 'Cola en reproducción. Se detendrá al terminar la selección.'
                      : 'Cola en reproducción con repetición activada.',
                  ),
                )
                .catch(onError)
            }
            busy={busy}
            mode={mode}
          />
          <div className="workspace-foot">
            <span>
              <Keyboard size={12} />
              {mode === 'mix'
                ? 'Teclas de cada control en pantalla · Shift + cue para guardar'
                : 'Usa los atajos guardados · Shift + cue para guardar'}
            </span>
            <button onClick={() => setSettings(true)}>
              Ver controles
              <ArrowUpRight size={11} />
            </button>
          </div>
        </div>
      </main>
      <BottomBar
        tracks={library.tracks}
        onError={onError}
        onNext={(direction) => {
          const current = getSnapshot().audio?.slots[0].id;
          const i = library.tracks.findIndex((t) => t.id === current);
          const next =
            library.tracks[(i + direction + library.tracks.length) % library.tracks.length];
          if (next) load(0, next.id);
        }}
      />
      {busy && (
        <div className="busy-indicator" role="status">
          <LoaderCircle size={15} className="spinning" />
          {busy}
        </div>
      )}
      {notice && (
        <div
          className={`toast ${notice.error ? 'error' : ''}`}
          role={notice.error ? 'alert' : 'status'}
        >
          <span>{notice.text}</span>
          <button aria-label="Cerrar aviso" onClick={() => setNotice(null)}>
            <X size={15} />
          </button>
        </div>
      )}
      {settings && (
        <Settings
          keys={keys}
          onKeys={saveKeys}
          onClose={() => setSettings(false)}
          onError={onError}
        />
      )}
      {newPlaylist && (
        <div className="modal-backdrop">
          <form
            className="modal playlist-modal"
            onSubmit={async (e) => {
              e.preventDefault();
              if (!playlistName.trim()) return;
              const saved = await edit(
                editingPlaylist !== null
                  ? { kind: 'renamePlaylist', id: editingPlaylist, name: playlistName }
                  : { kind: 'createPlaylist', name: playlistName },
              );
              if (saved) {
                setNewPlaylist(false);
                setPlaylistName('');
                setEditingPlaylist(null);
              }
            }}
          >
            <div className="modal-header">
              <h2>{editingPlaylist !== null ? 'Renombrar playlist' : 'Nueva playlist'}</h2>
              <button
                type="button"
                aria-label="Cancelar"
                onClick={() => {
                  setNewPlaylist(false);
                  setEditingPlaylist(null);
                  setPlaylistName('');
                }}
              >
                <X size={18} />
              </button>
            </div>
            <label>
              Nombre
              <input
                autoFocus
                maxLength={80}
                value={playlistName}
                onChange={(e) => setPlaylistName(e.target.value)}
                placeholder="Una selección para…"
                required
              />
            </label>
            <button className="primary" type="submit">
              <Plus size={14} />
              {editingPlaylist !== null ? 'Guardar nombre' : 'Crear playlist'}
            </button>
          </form>
        </div>
      )}
    </div>
  );
}
function ConnectionStatus() {
  const s = useSnapshot();
  const failed = !!s.error || s.audio?.streamError;
  return (
    <span
      className={`connection ${failed ? 'failed' : ''}`}
      title={s.error ?? s.output?.device ?? 'Iniciando motor'}
    >
      <span className={`status-dot ${!failed && s.output ? 'active' : ''}`} />
      {failed
        ? 'REVISAR AUDIO'
        : s.output
          ? `${s.output.backend} · ${s.output.sampleRate / 1000} kHz`
          : 'AUDIO NATIVO'}
    </span>
  );
}
function Player({
  tracks,
  onError,
}: {
  tracks: LibraryData['tracks'];
  onError: (e: unknown) => void;
}) {
  const s = useSnapshot();
  const slot = s.audio?.slots[0] ?? EMPTY_SLOT;
  const track = tracks.find((t) => t.id === slot.id);
  return (
    <section className="player-card">
      <div className="section-line">
        <span className="eyebrow">AHORA SUENA</span>
        <span className="small-badge">
          {s.queue.active ? `COLA ${s.queue.index + 1}/${s.queue.count}` : 'DECK A'}
        </span>
      </div>
      <div className={`album-art ${slot.playing ? 'playing' : ''}`}>
        <div className="orbit orbit-1" />
        <div className="orbit orbit-2" />
        <div className="orbit orbit-3" />
        <div className="album-core">
          <AudioLines size={29} strokeWidth={1.3} />
        </div>
        <span>CURADO SELECTS</span>
      </div>
      {s.queue.error && <p className="error-text">Cola detenida: {s.queue.error}</p>}
      <div className="player-info">
        <h2>{track?.title ?? 'Dale espacio a tu música.'}</h2>
        <p>{track?.artist ?? 'Importa una pista o prueba la sesión de ejemplo.'}</p>
      </div>
      <Waveform
        compact
        peaks={track?.peaks ?? []}
        progress={slot.duration ? slot.position / slot.duration : 0}
        onSeek={track ? (f) => control(0, 'seek', f * slot.duration).catch(onError) : undefined}
      />
      <div className="player-times">
        <span>{time(slot.position)}</span>
        <span>{time(slot.duration)}</span>
      </div>
      <div className="player-transport">
        <span className="eyebrow">{slot.playing ? 'REPRODUCIENDO' : 'LISTO PARA ESCUCHAR'}</span>
        <button
          className="round-play"
          disabled={!track}
          aria-label={slot.playing ? 'Pausar' : 'Reproducir'}
          onClick={() => control(0, 'toggle').catch(onError)}
        >
          {slot.playing ? (
            <Pause size={21} fill="currentColor" />
          ) : (
            <Play size={21} fill="currentColor" />
          )}
        </button>
        <Key>ESPACIO</Key>
      </div>
    </section>
  );
}
function BottomBar({
  tracks,
  onError,
  onNext,
}: {
  tracks: LibraryData['tracks'];
  onError: (e: unknown) => void;
  onNext: (direction: number) => void;
}) {
  const s = useSnapshot();
  const a = s.audio?.slots[0] ?? EMPTY_SLOT;
  const track = tracks.find((t) => t.id === a.id);
  return (
    <footer className="bottom-bar">
      <div className="bottom-track">
        <div className="mini-art">
          <AudioLines size={19} />
        </div>
        <span>
          <strong>{track?.title ?? 'Tu música, a tu ritmo.'}</strong>
          <small>{track?.artist ?? 'Selecciona una pista para comenzar'}</small>
        </span>
      </div>
      <div className="bottom-transport">
        <button aria-label="Pista anterior" disabled={!tracks.length} onClick={() => onNext(-1)}>
          <SkipBack size={15} />
        </button>
        <button
          className="mini-play"
          disabled={!track}
          aria-label="Play o pausa deck A"
          onClick={() => control(0, 'toggle').catch(onError)}
        >
          {a.playing ? (
            <Pause size={16} fill="currentColor" />
          ) : (
            <Play size={16} fill="currentColor" />
          )}
        </button>
        <button aria-label="Pista siguiente" disabled={!tracks.length} onClick={() => onNext(1)}>
          <SkipForward size={15} />
        </button>
        <span>{time(a.position)}</span>
        <div className="bottom-progress">
          <i style={{ width: `${a.duration ? (a.position / a.duration) * 100 : 0}%` }} />
        </div>
        <span>{time(a.duration)}</span>
      </div>
      <div className="bottom-master">
        <Meter value={s.audio?.peak ?? 0} />
        <Fader
          label="Master"
          value={s.audio?.master ?? 0.75}
          onChange={(v) => parameter(0, 'master', v, onError)}
        />
        <Radio size={15} />
        <span>LOCAL</span>
      </div>
    </footer>
  );
}

function PersistenceNotice() {
  const { persistenceError } = useSnapshot();
  return persistenceError ? (
    <div role="alert" className="notice error-text">
      No se pudieron guardar los cues: {persistenceError}. Comprueba el espacio en disco antes de
      salir.
    </div>
  ) : null;
}
