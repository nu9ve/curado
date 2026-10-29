import { tr, useLanguage } from './lib/language';
import { useCallback, useEffect, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import {
  Library as LibraryIcon,
  Star,
  ListMusic,
  Plus,
  Settings2,
  Headphones,
  SlidersHorizontal,
  X,
  LoaderCircle,
  Sparkles,
  PictureInPicture2,
  Radio,
} from 'lucide-react';
import { connectTheme } from './lib/theme-store';
import {
  call,
  connect,
  getSnapshot,
  native,
  useSnapshot,
  useMode,
  setSessionMode,
} from './lib/bridge';
import type { ImportReport, KeyBinding, Library as LibraryData, LibraryEdit } from './lib/types';
import { EMPTY_SLOT } from './lib/types';
import { DEFAULT_KEYS, allowedInMode, matchKey } from './lib/keyboard';
import { feedback } from './lib/feedback';
import { refreshWaveform } from './lib/waveform';
import { setMiniWindow } from './lib/window';
import { Deck } from './components/Deck';
import { Mixer, SampleBank } from './components/Mixer';
import { Library } from './components/Library';
import { Visual } from './components/Visual';
import { Settings } from './components/Settings';
import { ZoomWave } from './components/Waveform';
import { KeyboardMap, KeyHud } from './components/KeyboardMap';
import { BottomBar, MiniPlayer, NowPlaying } from './components/Player';
import { Logo, Wordmark } from './components/Logo';
import { WebExplorer } from './components/WebExplorer';

export default function App() {
  useLanguage();
  const [library, setLibrary] = useState<LibraryData>({ tracks: [], playlists: [] });
  const [view, setView] = useState('all');
  const mode = useMode();
  const [keys, setKeys] = useState(DEFAULT_KEYS);
  const [selected, setSelected] = useState<number | null>(null);
  const [settings, setSettings] = useState(false);
  const [mini, setMini] = useState(false);
  const [helpExpanded, setHelpExpanded] = useState(false);
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
  /** Keyboard, keycap clicks and the HUD share one dispatch path to the native queue. */
  const trigger = useCallback(
    (binding: KeyBinding) => {
      const allowed = allowedInMode(binding.action, mode);
      feedback(binding, allowed);
      if (allowed) call('audio_action', { action: binding.action }).catch(onError);
    },
    [mode, onError],
  );
  useEffect(() => {
    if (settings || newPlaylist) return;
    const handle = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      if (target.closest('input,textarea,select,[contenteditable="true"],[role="slider"]')) return;
      if (target.closest('button') && ['Space', 'Enter'].includes(e.code)) return;
      // Match ignoring mode first so blocked keys still explain themselves in the HUD.
      const binding = matchKey(e, keys, 'mix');
      if (!binding) return;
      e.preventDefault();
      trigger(binding);
    };
    window.addEventListener('keydown', handle);
    return () => window.removeEventListener('keydown', handle);
  }, [keys, settings, newPlaylist, trigger]);
  const load = useCallback(
    async (slot: number, id: number) => {
      setLoading((prev) => [...prev, slot]);
      try {
        await call('load_track', { slot, id });
        setLibrary(await call<LibraryData>('get_library'));
        void refreshWaveform(id);
        const destination = slot < 2 ? `deck ${slot ? 'B' : 'A'}` : `sample ${slot - 1}`;
        notify(tr(`Pista lista en ${destination}.`, `Track ready in ${destination}.`));
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
      setBusy(tr('Importando y analizando música…', 'Importing and analyzing music…'));
      try {
        const report = await call<ImportReport>('import_media', { folder });
        setLibrary(await call<LibraryData>('get_library'));
        if (report.added || report.skipped || report.errors.length)
          notify(
            tr(
              `${report.added} pistas añadidas · ${report.skipped} existentes`,
              `${report.added} tracks added · ${report.skipped} already present`,
            ) + (report.errors.length ? ` · ${report.errors.join('\n')}` : ''),
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
    setBusy(tr('Preparando sesión de prueba…', 'Preparing demo session…'));
    try {
      const data = await call<LibraryData>('create_demo');
      setLibrary(data);
      const demos = data.tracks
        .filter((t) => t.path.includes('/demo/') || t.path.includes('\\demo\\'))
        .sort((a, b) => a.title.localeCompare(b.title));
      for (let i = 0; i < Math.min(6, demos.length); i++)
        await call('load_track', { slot: i, id: demos[i].id });
      notify(
        tr(
          'Sesión de prueba lista. Dale Play; cambia a Mezcla para usar ambos decks y samples.',
          'Demo ready. Press Play; switch to Mix to use both decks and samples.',
        ),
      );
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
  const toggleMini = async (on: boolean) => {
    try {
      await setMiniWindow(on);
      setMini(on);
    } catch (e) {
      onError(e);
    }
  };
  const next = (direction: number) => {
    const current = getSnapshot().audio?.slots[0].id;
    const i = library.tracks.findIndex((t) => t.id === current);
    const track = library.tracks[(i + direction + library.tracks.length) % library.tracks.length];
    if (track) load(0, track.id);
  };
  const favorites = library.tracks.filter((t) => t.favorite).length;

  if (mini)
    return (
      <div className="app-shell is-mini">
        <MiniPlayer
          tracks={library.tracks}
          onError={onError}
          onExit={() => toggleMini(false)}
          onNext={next}
        />
        <KeyHud />
        {notice && <Toast notice={notice} onClose={() => setNotice(null)} />}
      </div>
    );

  return (
    <div className={`app-shell view-${mode} ${helpExpanded ? 'help-expanded' : ''}`}>
      <div className="window-drag-region" data-tauri-drag-region aria-hidden="true" />
      <aside className="sidebar">
        <div className="brand" data-tauri-drag-region>
          <Logo size={26} />
          <Wordmark />
        </div>
        <nav className="side-nav" aria-label={tr('Colección', 'Collection')}>
          <span className="eyebrow">{tr('colección', 'collection')}</span>
          <NavItem
            active={view === 'all'}
            onClick={() => setView('all')}
            icon={<LibraryIcon size={16} />}
            label={tr('toda la música', 'all music')}
            count={library.tracks.length}
          />
          <NavItem
            active={view === 'favorites'}
            onClick={() => setView('favorites')}
            icon={<Star size={16} />}
            label={tr('favoritos', 'favorites')}
            count={favorites}
          />
        </nav>
        <nav className="side-nav playlists" aria-label="Playlists">
          <div className="section-line">
            <span className="eyebrow">playlists</span>
            <button
              className="icon-button"
              aria-label={tr('Crear playlist', 'Create playlist')}
              onClick={() => setNewPlaylist(true)}
            >
              <Plus size={14} />
            </button>
          </div>
          {library.playlists.map((p) => (
            <NavItem
              key={p.id}
              active={view === `playlist:${p.id}`}
              onClick={() => setView(`playlist:${p.id}`)}
              onDoubleClick={() => {
                setEditingPlaylist(p.id);
                setPlaylistName(p.name);
                setNewPlaylist(true);
              }}
              title="Doble clic para renombrar"
              icon={<ListMusic size={15} />}
              label={p.name}
              count={p.tracks.length}
            />
          ))}
          {!library.playlists.length && (
            <button className="sidebar-hint" onClick={() => setNewPlaylist(true)}>
              <Plus size={13} /> {tr('un lugar para cada mood', 'a place for every mood')}{' '}
            </button>
          )}
        </nav>
        <div className="sidebar-bottom">
          <button
            className="settings-button"
            onClick={() => setView('web')}
            aria-pressed={view === 'web'}
            data-help="web-explorer"
          >
            <Radio size={15} />
            radio / web
          </button>
          <button className="demo-button" disabled={!!busy} onClick={demo}>
            <Sparkles size={15} />
            <span>
              {tr('sesión de prueba', 'demo session')}{' '}
              <small>{tr('2 tracks + 4 samples locales', '2 tracks + 4 local samples')}</small>
            </span>
          </button>
          <button className="settings-button" onClick={() => setSettings(true)}>
            <Settings2 size={15} />
            {tr('configuración', 'settings')}{' '}
          </button>
          <span className="build-label">curado 0.1 · local first</span>
        </div>
      </aside>
      <header className="topbar" data-tauri-drag-region>
        <div className="breadcrumb" data-tauri-drag-region>
          <span>{tr('tu espacio', 'your space')}</span>
          <i>/</i>
          <b>
            {view === 'web'
              ? 'radio / web'
              : mode === 'player'
                ? tr('reproductor', 'player')
                : tr('mezcla', 'mix')}
          </b>
        </div>
        <div
          className={`mode-switch is-${mode}`}
          role="group"
          aria-label={tr('Modo de uso', 'Mode')}
        >
          <i className="mode-indicator" aria-hidden="true" />
          <button
            className={mode === 'player' ? 'active' : ''}
            aria-pressed={mode === 'player'}
            data-help="player"
            onClick={() => changeMode('player')}
          >
            <Headphones size={14} />
            {tr('reproductor', 'player')}{' '}
          </button>
          <button
            className={mode === 'mix' ? 'active' : ''}
            aria-pressed={mode === 'mix'}
            data-help="mix"
            onClick={() => changeMode('mix')}
          >
            <SlidersHorizontal size={14} />
            {tr('mezcla', 'mix')}{' '}
          </button>
        </div>
        <div className="header-right">
          <ConnectionStatus />
          <button
            className="icon-button"
            aria-label={tr('Vista mini siempre visible', 'Always-on-top mini view')}
            data-help="mini"
            title={tr('Vista mini · siempre encima', 'Mini view · always on top')}
            onClick={() => toggleMini(true)}
          >
            <PictureInPicture2 size={16} />
          </button>
          <button
            className="icon-button"
            aria-label={tr('Abrir configuración', 'Open settings')}
            data-help="settings"
            title={tr('Configuración', 'Settings')}
            onClick={() => setSettings(true)}
          >
            <Settings2 size={16} />
          </button>
        </div>
      </header>
      <main className="sheet">
        <div className={`sheet-scroll ${view === 'web' ? 'is-web' : ''}`}>
          <PersistenceNotice />
          {!native && (
            <div className="notice">
              {tr(
                'Vista previa de la interfaz · el audio y las integraciones requieren la app de escritorio.',
                'Interface preview · audio and integrations require the desktop app.',
              )}{' '}
            </div>
          )}
          {view === 'web' ? (
            <WebExplorer onError={onError} suspended={settings || newPlaylist} />
          ) : mode === 'mix' ? (
            <MixView
              library={library}
              keys={keys}
              onError={onError}
              loading={loading}
              onLoad={load}
              selected={selected}
              onTrigger={trigger}
              onLibrary={setLibrary}
            />
          ) : (
            <div className="listen-stage">
              <NowPlaying tracks={library.tracks} keys={keys} onError={onError} />
              <Visual onError={onError} />
            </div>
          )}
          {view !== 'web' && (
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
                        ? tr(
                            'Cola en reproducción. Se detendrá al terminar la selección.',
                            'Queue playing. It will stop at the end of the selection.',
                          )
                        : tr(
                            'Cola en reproducción con repetición activada.',
                            'Queue playing with repeat enabled.',
                          ),
                    ),
                  )
                  .catch(onError)
              }
              busy={busy}
              mode={mode}
            />
          )}
        </div>
        <KeyHud />
      </main>
      <BottomBar
        tracks={library.tracks}
        keys={keys}
        helpExpanded={helpExpanded}
        onHelpExpand={setHelpExpanded}
        onError={onError}
        onNext={next}
      />
      {busy && (
        <div className="busy-indicator" role="status">
          <LoaderCircle size={15} className="spinning" />
          {busy}
        </div>
      )}
      {notice && <Toast notice={notice} onClose={() => setNotice(null)} />}
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
            onKeyDown={(e) => {
              if (e.key === 'Escape') {
                setNewPlaylist(false);
                setEditingPlaylist(null);
                setPlaylistName('');
              }
            }}
          >
            <div className="modal-header">
              <h2>
                {editingPlaylist !== null
                  ? tr('renombrar playlist', 'rename playlist')
                  : tr('nueva playlist', 'new playlist')}
              </h2>
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
              {tr('Nombre', 'Name')}{' '}
              <input
                autoFocus
                maxLength={80}
                value={playlistName}
                onChange={(e) => setPlaylistName(e.target.value)}
                placeholder={tr('Una selección para…', 'A selection for…')}
                required
              />
            </label>
            <button className="primary" type="submit">
              <Plus size={14} />
              {editingPlaylist !== null
                ? tr('Guardar nombre', 'Save name')
                : tr('Crear playlist', 'Create playlist')}
            </button>
          </form>
        </div>
      )}
    </div>
  );
}

function NavItem({
  active,
  icon,
  label,
  count,
  ...rest
}: {
  active: boolean;
  icon: ReactNode;
  label: string;
  count: number;
  onClick: () => void;
  onDoubleClick?: () => void;
  title?: string;
}) {
  return (
    <button
      className={`nav-item ${active ? 'active' : ''}`}
      aria-current={active ? 'page' : undefined}
      {...rest}
    >
      {icon}
      <span className="nav-label">{label}</span>
      <span className="nav-count">{count}</span>
    </button>
  );
}

function MixView({
  library,
  keys,
  onError,
  loading,
  onLoad,
  selected,
  onTrigger,
  onLibrary,
}: {
  library: LibraryData;
  keys: KeyBinding[];
  onError: (e: unknown) => void;
  loading: number[];
  onLoad: (slot: number, id: number) => void;
  selected: number | null;
  onTrigger: (binding: KeyBinding) => void;
  onLibrary: (data: LibraryData) => void;
}) {
  const { audio } = useSnapshot();
  const slots = [0, 1].map((i) => audio?.slots[i] ?? EMPTY_SLOT);
  return (
    <div className="mix-view">
      <div className="zoom-stack">
        {slots.map((slot, i) => (
          <ZoomWave
            key={i}
            name={i ? 'B' : 'A'}
            tone={i ? 'b' : 'a'}
            trackId={library.tracks.some((t) => t.id === slot.id) ? slot.id : undefined}
            slot={slot}
            onSeek={
              slot.id
                ? (s) =>
                    call('audio_action', { action: { slot: i, kind: 'seek', value: s } }).catch(
                      onError,
                    )
                : undefined
            }
          />
        ))}
      </div>
      <div className="console">
        <Deck
          index={0}
          onLibrary={onLibrary}
          tracks={library.tracks}
          keys={keys}
          onError={onError}
          loading={loading.includes(0)}
        />
        <Mixer keys={keys} onError={onError} />
        <Deck
          index={1}
          onLibrary={onLibrary}
          tracks={library.tracks}
          keys={keys}
          onError={onError}
          loading={loading.includes(1)}
        />
      </div>
      <div className="console-foot">
        <SampleBank
          tracks={library.tracks}
          keys={keys}
          onError={onError}
          onLoad={onLoad}
          selected={selected}
        />
        <KeyboardMap keys={keys} mode="mix" onTrigger={onTrigger} />
      </div>
    </div>
  );
}

function Toast({
  notice,
  onClose,
}: {
  notice: { text: string; error: boolean };
  onClose: () => void;
}) {
  return (
    <div
      className={`toast ${notice.error ? 'error' : ''}`}
      role={notice.error ? 'alert' : 'status'}
    >
      <span>{notice.text}</span>
      <button aria-label={tr('Cerrar aviso', 'Close notice')} onClick={onClose}>
        <X size={15} />
      </button>
    </div>
  );
}

function ConnectionStatus() {
  const s = useSnapshot();
  const failed = !!s.error || s.audio?.streamError;
  return (
    <span
      className={`connection ${failed ? 'failed' : ''}`}
      title={s.error ?? s.output?.device ?? tr('Iniciando motor', 'Starting engine')}
    >
      <span className={`status-dot ${!failed && s.output ? 'active' : ''}`} />
      {failed
        ? tr('revisar audio', 'check audio')
        : s.output
          ? `${s.output.backend} · ${s.output.sampleRate / 1000} kHz`
          : tr('audio nativo', 'native audio')}
    </span>
  );
}

function PersistenceNotice() {
  const { persistenceError } = useSnapshot();
  return persistenceError ? (
    <div role="alert" className="notice error-text">
      {tr('No se pudieron guardar los cues:', 'Could not save cues:')} {persistenceError}
      {tr(
        '. Comprueba el espacio en disco antes de salir.',
        '. Check disk space before quitting.',
      )}{' '}
    </div>
  ) : null;
}
