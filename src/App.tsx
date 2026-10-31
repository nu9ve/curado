import { tr, useLanguage } from './lib/language';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { CSSProperties, ReactNode } from 'react';
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
  Folder,
  HardDrive,
  Usb,
  PanelLeftClose,
  PanelLeftOpen,
  PanelRightClose,
  PanelRightOpen,
} from 'lucide-react';
import { connectTheme } from './lib/theme-store';
import {
  call,
  connect,
  getSnapshot,
  native,
  useDevices,
  useSnapshot,
  useMode,
  setSessionMode,
} from './lib/bridge';
import type {
  DeviceExport,
  DeviceInfo,
  ImportReport,
  RekordboxExportReport,
  KeyBinding,
  Library as LibraryData,
  LibraryEdit,
} from './lib/types';
import { EMPTY_SLOT } from './lib/types';
import { DEFAULT_KEYS, allowedInMode, matchKey, resolveAction } from './lib/keyboard';
import { feedback } from './lib/feedback';
import { refreshWaveform } from './lib/waveform';
import { invalidateCueInfo } from './lib/cues';
import {
  deviceViewId,
  exportSummary,
  parseDeviceView,
  playlistTree,
  tracksInSection,
} from './lib/devices';
import type { PlaylistRow } from './lib/devices';
import { setMiniWindow } from './lib/window';
import { readMixLayout, saveMixLayout } from './lib/mix-layout';
import type { MixLayout } from './lib/mix-layout';
import { Deck } from './components/Deck';
import { Mixer, SampleBank } from './components/Mixer';
import { DeckTools, HardwareConsole, KeyboardGuide } from './components/HardwareConsole';
import type { ScreenKey } from './components/HardwareConsole';
import { Library } from './components/Library';
import { DeviceBrowser } from './components/DeviceBrowser';
import { Visual } from './components/Visual';
import { Settings } from './components/Settings';
import { ZoomWave } from './components/Waveform';
import { KeyHud } from './components/KeyboardMap';
import { BottomBar, MiniPlayer, NowPlaying } from './components/Player';
import { Logo, Wordmark } from './components/Logo';
import { WebExplorer } from './components/WebExplorer';
import { setHapticsEnabled } from './lib/haptics';
import { TrackInfo } from './components/TrackInfo';
import { BeatFxPanel, ColorFxPanel } from './components/Performance';
import { clampSplit, DEFAULT_WORKSPACE, useWorkspace } from './lib/workspace';
import type { SidePanel, Workspace } from './lib/workspace';

export default function App() {
  useLanguage();
  const [workspace, updateWorkspace] = useWorkspace();
  useEffect(() => setHapticsEnabled(workspace.haptics), [workspace.haptics]);
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
      // The engine starts with it off, and this is a preference rather than mix state.
      if (workspace.autoGain)
        call('audio_action', { action: { slot: 0, kind: 'autoGain', value: 1 } }).catch(onError);
      call<LibraryData>('get_library')
        .then((data) => {
          if (!disposed) setLibrary(data);
        })
        .catch(onError);
      call<KeyBinding[] | null>('get_keyboard')
        .then((data) => {
          // Saved under the old layout, without the bar's new meaning: it stays stored until a
          // new choice is saved, but the scheme it was made for is gone.
          if (data && !disposed && data.some((key) => key.action.kind === 'toggleBoth'))
            setKeys(data);
        })
        .catch(onError);
    }
    return () => {
      disposed = true;
      off?.();
      clearTimeout(noticeTimer.current);
    };
    // Applied once at startup; changes go through the console's own control.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [onError]);
  const saveKeys = useCallback(async (bindings: KeyBinding[]) => {
    await call('save_keyboard', { bindings });
    setKeys(bindings);
  }, []);
  const listKeys = useRef({ move: (_delta: number) => {}, load: (_slot: number) => {} });
  /** Keyboard, keycap clicks and the HUD share one dispatch path to the native queue. */
  const trigger = useCallback(
    (binding: KeyBinding) => {
      const allowed = allowedInMode(binding.action, mode);
      feedback(binding, allowed);
      if (!allowed) return;
      const { kind, slot, value } = binding.action;
      if (kind === 'listMove') listKeys.current.move(value);
      else if (kind === 'listLoad') listKeys.current.load(slot);
      else {
        const action = resolveAction(binding.action, getSnapshot().audio, mode);
        call('audio_action', { action }).catch(onError);
      }
    },
    [mode, onError],
  );
  useEffect(() => {
    if (settings || newPlaylist) return;
    const handle = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement;
      if (target.closest('input,textarea,select,[contenteditable="true"],[role="slider"]')) return;
      // A button keeps focus after a click, and the bar must still mean the whole mix then.
      if (target.closest('button:focus-visible') && ['Space', 'Enter'].includes(e.code)) return;
      // Match ignoring mode first so blocked keys still explain themselves in the HUD.
      const binding = matchKey(e, keys, 'mix');
      if (!binding) return;
      e.preventDefault();
      trigger(binding);
    };
    window.addEventListener('keydown', handle);
    return () => window.removeEventListener('keydown', handle);
  }, [keys, settings, newPlaylist, trigger]);
  const loadWith = useCallback(
    async (slot: number, start: () => Promise<number | null>) => {
      setLoading((prev) => [...prev, slot]);
      try {
        const id = await start();
        if (id === null) return;
        invalidateCueInfo(id);
        setLibrary(await call<LibraryData>('get_library'));
        void refreshWaveform(id);
        const destination = slot < 2 ? `deck ${slot ? 'B' : 'A'}` : `sample ${slot - 1}`;
        notify(tr(`Pista lista en ${destination}.`, `Track ready in ${destination}.`));
      } catch (e) {
        onError(e);
      } finally {
        // One entry per load: a slot stays busy while a newer load is still running.
        setLoading((prev) => prev.filter((value, i) => i !== prev.indexOf(slot) || value !== slot));
      }
    },
    [notify, onError],
  );
  const load = useCallback(
    (slot: number, id: number) =>
      loadWith(slot, async () => ((await call<boolean>('load_track', { slot, id })) ? id : null)),
    [loadWith],
  );
  const devices = useDevices();
  const deviceView = parseDeviceView(view);
  const activeDevice = deviceView
    ? (devices.find((device) => device.id === deviceView.device) ?? null)
    : null;
  const [deviceLibraries, setDeviceLibraries] = useState<Record<string, DeviceExport>>({});
  const [deviceError, setDeviceError] = useState<string | null>(null);
  const [deviceSelected, setDeviceSelected] = useState<number | null>(null);
  // Listed on opening each device, so a drive exported again while plugged in is picked up.
  const openDevice = activeDevice?.rekordbox ? activeDevice.id : null;
  useEffect(() => {
    setDeviceError(null);
    if (!openDevice) return;
    let current = true;
    call<DeviceExport>('device_library', { device: openDevice })
      .then((library) => {
        if (current) setDeviceLibraries((known) => ({ ...known, [openDevice]: library }));
      })
      .catch((error) => {
        if (current) setDeviceError(String(error));
      });
    return () => {
      current = false;
    };
  }, [openDevice]);
  useEffect(() => {
    // A drive that was pulled out takes its view with it. What is already loaded keeps playing.
    if (deviceView && !activeDevice) setView('all');
    setDeviceLibraries((known) => {
      const kept = Object.entries(known).filter(([id]) => devices.some((d) => d.id === id));
      return kept.length === Object.keys(known).length ? known : Object.fromEntries(kept);
    });
  }, [devices, deviceView, activeDevice]);
  const openedLibrary = activeDevice ? (deviceLibraries[activeDevice.id] ?? null) : null;
  const deviceRows = useMemo(
    () => (openedLibrary ? playlistTree(openedLibrary.playlists) : []),
    [openedLibrary],
  );
  const localLibrary = useMemo(
    () => ({ ...library, tracks: library.tracks.filter((track) => !track.device) }),
    [library],
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
  const exportRekordbox = useCallback(async () => {
    setBusy(tr('Exportando a rekordbox…', 'Exporting to rekordbox…'));
    try {
      const report = await call<RekordboxExportReport>('export_rekordbox_xml');
      if (report.saved) notify(exportSummary(report), report.onDevice > 0);
    } catch (e) {
      onError(e);
    } finally {
      setBusy(null);
    }
  }, [notify, onError]);
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

  /** The keys above the console's screen drive the side panel, which is where lists live. */
  const screenKey = (key: ScreenKey) => {
    if (key === 'menu') {
      setSettings(true);
      return;
    }
    updateWorkspace({ panelOpen: true, panel: 'library' });
    if (key === 'tag') setView('favorites');
    if (key === 'playlist' && library.playlists[0]) setView(`playlist:${library.playlists[0].id}`);
    if (key === 'search' || key === 'source') {
      const target =
        key === 'search' ? '.mix-panel .search input' : '.mix-panel .source-select select';
      requestAnimationFrame(() => document.querySelector<HTMLElement>(target)?.focus());
    }
  };
  /** The selector and the arrows move through what the panel lists, in its order. */
  const browseBy = (delta: number) => {
    if (deviceView && openedLibrary) {
      const listed = tracksInSection(
        openedLibrary.tracks,
        openedLibrary.playlists,
        deviceView.section,
      );
      if (!listed.length) return;
      const at = listed.findIndex((t) => t.id === deviceSelected);
      const next = at < 0 ? 0 : Math.max(0, Math.min(listed.length - 1, at + delta));
      setDeviceSelected(listed[next].id);
      return;
    }
    const active = library.playlists.find((p) => `playlist:${p.id}` === view);
    const listed = active
      ? active.tracks.flatMap((id) => localLibrary.tracks.filter((t) => t.id === id))
      : view === 'favorites'
        ? localLibrary.tracks.filter((t) => t.favorite)
        : localLibrary.tracks;
    if (!listed.length) return;
    const at = listed.findIndex((t) => t.id === selected);
    const next = at < 0 ? 0 : Math.max(0, Math.min(listed.length - 1, at + delta));
    setSelected(listed[next].id);
  };
  const loadDeviceTrack = (slot: number, track: number) => {
    if (!activeDevice) return;
    void loadWith(slot, () =>
      call<number | null>('device_load_track', { slot, device: activeDevice.id, track }),
    );
  };
  const loadChosen = (slot: number) => {
    if (deviceView) {
      if (deviceSelected !== null) loadDeviceTrack(slot, deviceSelected);
    } else if (selected !== null) load(slot, selected);
  };
  listKeys.current = { move: browseBy, load: loadChosen };
  const rail = mode === 'mix' ? workspace.railMix : workspace.railPlayer;
  const inWorkspace = mode === 'mix' && view !== 'web';
  const browser = (
    <>
      {inWorkspace && (
        <SourceSelect
          view={view}
          onView={setView}
          playlists={library.playlists}
          devices={devices}
          activeDevice={activeDevice?.id}
          rows={deviceRows}
        />
      )}
      {activeDevice && deviceView && (
        <DeviceBrowser
          device={activeDevice}
          section={deviceView.section}
          library={openedLibrary}
          error={deviceError}
          mix={mode === 'mix'}
          loading={loading.length > 0}
          onError={onError}
          selected={deviceSelected}
          onSelect={setDeviceSelected}
          onLoadTrack={loadDeviceTrack}
          onLoadFile={(slot, entry) =>
            void loadWith(slot, () => call<number | null>('device_load_file', { slot, entry }))
          }
        />
      )}
      {view !== 'web' && !deviceView && (
        <Library
          data={localLibrary}
          view={view}
          selected={selected}
          onSelect={setSelected}
          onLoad={load}
          onEdit={edit}
          onImport={importMedia}
          onExport={() => void exportRekordbox()}
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
    </>
  );
  return (
    <div
      className={`app-shell view-${mode} ${helpExpanded ? 'help-expanded' : ''} ${rail ? 'sidebar-rail' : ''} ${inWorkspace ? 'is-workspace' : ''}`}
    >
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
            count={localLibrary.tracks.length}
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
              title={`${p.name} · ${tr('doble clic para renombrar', 'double-click to rename')}`}
              icon={<ListMusic size={15} />}
              label={p.name}
              count={p.tracks.length}
              chip
            />
          ))}
          {!library.playlists.length && (
            <button className="sidebar-hint" onClick={() => setNewPlaylist(true)}>
              <Plus size={13} /> {tr('un lugar para cada mood', 'a place for every mood')}{' '}
            </button>
          )}
        </nav>
        {devices.length > 0 && (
          <nav
            className="side-nav devices"
            aria-label={tr('Dispositivos', 'Devices')}
            data-help="devices"
          >
            <span className="eyebrow">{tr('dispositivos', 'devices')}</span>
            {devices.map((device) => {
              const open = activeDevice?.id === device.id;
              const first = device.rekordbox ? 'tracks' : 'files';
              return (
                <div className="device-group" key={device.id}>
                  <NavItem
                    active={open}
                    onClick={() => setView(deviceViewId(device.id, { kind: first }))}
                    icon={device.rekordbox ? <Usb size={15} /> : <HardDrive size={15} />}
                    label={device.name}
                  />
                  {open && device.rekordbox && (
                    <NavItem
                      nested
                      active={deviceView?.section.kind === 'tracks'}
                      onClick={() => setView(deviceViewId(device.id, { kind: 'tracks' }))}
                      icon={<LibraryIcon size={13} />}
                      label={tr('todas las pistas', 'all tracks')}
                      count={openedLibrary?.tracks.length}
                    />
                  )}
                  {open &&
                    deviceRows.map(({ node, depth }) =>
                      node.folder ? (
                        <span
                          key={node.id}
                          className="nav-folder"
                          style={{ '--depth': depth } as CSSProperties}
                        >
                          <Folder size={12} />
                          {node.name}
                        </span>
                      ) : (
                        <NavItem
                          key={node.id}
                          nested
                          depth={depth}
                          active={
                            deviceView?.section.kind === 'playlist' &&
                            deviceView.section.id === node.id
                          }
                          onClick={() =>
                            setView(deviceViewId(device.id, { kind: 'playlist', id: node.id }))
                          }
                          icon={<ListMusic size={13} />}
                          label={node.name}
                          count={node.tracks.length}
                          chip
                        />
                      ),
                    )}
                  {open && (
                    <NavItem
                      nested
                      active={deviceView?.section.kind === 'files'}
                      onClick={() => setView(deviceViewId(device.id, { kind: 'files' }))}
                      icon={<Folder size={13} />}
                      label={tr('archivos', 'files')}
                    />
                  )}
                </div>
              );
            })}
          </nav>
        )}
        <div className="sidebar-bottom">
          <button
            className="settings-button"
            onClick={() => setView('web')}
            aria-pressed={view === 'web'}
            aria-label="radio / web"
            title="radio / web"
            data-help="web-explorer"
          >
            <Radio size={15} />
            <span className="nav-label">radio / web</span>
          </button>
          <button className="demo-button" disabled={!!busy} onClick={demo}>
            <Sparkles size={15} />
            <span>
              {tr('sesión de prueba', 'demo session')}{' '}
              <small>{tr('2 tracks + 4 samples locales', '2 tracks + 4 local samples')}</small>
            </span>
          </button>
          <button
            className="settings-button"
            aria-label={tr('Configuración', 'Settings')}
            title={tr('Configuración', 'Settings')}
            onClick={() => setSettings(true)}
          >
            <Settings2 size={15} />
            <span className="nav-label">{tr('configuración', 'settings')}</span>
          </button>
          <span className="build-label">curado 0.1 · local first</span>
        </div>
      </aside>
      <header className="topbar" data-tauri-drag-region>
        <div className="breadcrumb" data-tauri-drag-region>
          <button
            className="icon-button rail-toggle"
            aria-pressed={rail}
            aria-label={tr('Barra lateral compacta', 'Compact sidebar')}
            title={
              rail
                ? tr('Mostrar la barra lateral completa', 'Show the full sidebar')
                : tr('Compactar la barra lateral', 'Compact the sidebar')
            }
            onClick={() =>
              updateWorkspace(mode === 'mix' ? { railMix: !rail } : { railPlayer: !rail })
            }
          >
            {rail ? <PanelLeftOpen size={15} /> : <PanelLeftClose size={15} />}
          </button>
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
        <div
          className={`sheet-scroll ${view === 'web' ? 'is-web' : ''} ${inWorkspace ? 'is-workspace' : ''}`}
        >
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
              onEdit={edit}
              workspace={workspace}
              onWorkspace={updateWorkspace}
              browser={browser}
              onScreenKey={screenKey}
              onBrowse={browseBy}
            />
          ) : (
            <div className="listen-stage">
              <NowPlaying tracks={library.tracks} keys={keys} onError={onError} />
              <Visual onError={onError} />
            </div>
          )}
          {!inWorkspace && browser}
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
  nested = false,
  depth = 0,
  chip = false,
  title,
  ...rest
}: {
  active: boolean;
  icon: ReactNode;
  label: string;
  count?: number;
  nested?: boolean;
  depth?: number;
  /** On the rail its two first letters stand in, because many lists share one icon. */
  chip?: boolean;
  onClick: () => void;
  onDoubleClick?: () => void;
  title?: string;
}) {
  return (
    <button
      className={`nav-item ${active ? 'active' : ''} ${nested ? 'nested' : ''} ${chip ? 'has-chip' : ''}`}
      aria-current={active ? 'page' : undefined}
      aria-label={label}
      title={title ?? label}
      style={nested ? ({ '--depth': depth } as CSSProperties) : undefined}
      {...rest}
    >
      {icon}
      {chip && (
        <span className="nav-chip" aria-hidden="true">
          {label.slice(0, 2)}
        </span>
      )}
      <span className="nav-label">{label}</span>
      {count !== undefined && <span className="nav-count">{count}</span>}
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
  onEdit,
  workspace,
  onWorkspace,
  browser,
  onScreenKey,
  onBrowse,
}: {
  library: LibraryData;
  keys: KeyBinding[];
  onError: (e: unknown) => void;
  loading: number[];
  onLoad: (slot: number, id: number) => void;
  selected: number | null;
  onTrigger: (binding: KeyBinding) => void;
  onLibrary: (data: LibraryData) => void;
  onEdit: (edit: LibraryEdit) => unknown;
  workspace: Workspace;
  onWorkspace: (change: Partial<Workspace>) => void;
  browser: ReactNode;
  onScreenKey: (key: ScreenKey) => void;
  onBrowse: (delta: number) => void;
}) {
  const { audio } = useSnapshot();
  const [layout, setLayout] = useState<MixLayout>(() => {
    try {
      return readMixLayout(globalThis.localStorage);
    } catch {
      return 'classic';
    }
  });
  const chooseLayout = (next: MixLayout) => {
    setLayout(next);
    try {
      saveMixLayout(globalThis.localStorage, next);
    } catch {
      // Access to the storage object itself can be blocked by the WebView.
    }
  };
  const slots = [0, 1].map((i) => audio?.slots[i] ?? EMPTY_SLOT);
  const waves = (
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
  );
  const panels: { id: SidePanel; label: string }[] = [
    { id: 'library', label: tr('biblioteca', 'library') },
    { id: 'tools', label: tr('herramientas', 'tools') },
    { id: 'keyboard', label: tr('teclado', 'keyboard') },
  ];
  const workspaceRef = useRef<HTMLDivElement>(null);
  const moveSplit = (x: number) => {
    const bounds = workspaceRef.current?.getBoundingClientRect();
    if (bounds?.width) onWorkspace({ split: clampSplit((x - bounds.left) / bounds.width) });
  };
  return (
    <div
      ref={workspaceRef}
      className={`mix-workspace ${workspace.panelOpen ? '' : 'panel-closed'}`}
      style={{ '--split': workspace.split } as CSSProperties}
    >
      <section className="mix-main">
        <div
          className="mix-layout-bar"
          role="toolbar"
          aria-label={tr('Vista de mezcla', 'Mix layout')}
        >
          <button
            type="button"
            aria-pressed={layout === 'classic'}
            onClick={() => chooseLayout('classic')}
          >
            {tr('curado', 'curado')}
          </button>
          <button
            type="button"
            aria-pressed={layout === 'console'}
            onClick={() => chooseLayout('console')}
          >
            {tr('cabina', 'console')}
          </button>
          {layout === 'console' && (
            <button
              type="button"
              aria-pressed={workspace.badges}
              title={tr(
                'Muestra en cada control su atajo de teclado',
                'Shows each control its keyboard shortcut',
              )}
              onClick={() => onWorkspace({ badges: !workspace.badges })}
            >
              {tr('teclas', 'keys')}
            </button>
          )}
          <button
            type="button"
            aria-pressed={workspace.haptics}
            title={tr(
              'Toques al mover controles, en un trackpad con Force Touch',
              'Taps when moving controls, on a Force Touch trackpad',
            )}
            onClick={() => onWorkspace({ haptics: !workspace.haptics })}
          >
            {tr('toques', 'haptics')}
          </button>
          <span className="mix-bar-gap" />
          <button
            type="button"
            className="icon-button"
            aria-pressed={workspace.panelOpen}
            aria-label={tr('Panel lateral', 'Side panel')}
            title={
              workspace.panelOpen
                ? tr(
                    'Ocultar el panel para enfocarte en la mezcla',
                    'Hide the panel to focus on mixing',
                  )
                : tr('Mostrar el panel', 'Show the panel')
            }
            onClick={() => onWorkspace({ panelOpen: !workspace.panelOpen })}
          >
            {workspace.panelOpen ? <PanelRightClose size={15} /> : <PanelRightOpen size={15} />}
          </button>
        </div>
        <div className="mix-stage">
          {layout === 'console' ? (
            <HardwareConsole
              library={library}
              keys={keys}
              loading={loading}
              onError={onError}
              selected={selected}
              onLoad={onLoad}
              onEdit={onEdit}
              badges={workspace.badges}
              autoGain={workspace.autoGain}
              onAutoGain={(autoGain) => onWorkspace({ autoGain })}
              onScreenKey={onScreenKey}
              onBrowse={onBrowse}
            />
          ) : (
            <div className="mix-classic">
              {waves}
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
            </div>
          )}
        </div>
      </section>
      {workspace.panelOpen && (
        <>
          <div
            className="mix-divider"
            role="separator"
            aria-orientation="vertical"
            aria-label={tr('Ancho del panel lateral', 'Side panel width')}
            aria-valuemin={35}
            aria-valuemax={82}
            aria-valuenow={Math.round(workspace.split * 100)}
            tabIndex={0}
            title={tr(
              'Arrastra para ajustar · doble clic: restablecer',
              'Drag to adjust · double-click: reset',
            )}
            onPointerDown={(e) => e.currentTarget.setPointerCapture(e.pointerId)}
            onPointerMove={(e) => {
              if (e.currentTarget.hasPointerCapture(e.pointerId)) moveSplit(e.clientX);
            }}
            onDoubleClick={() => onWorkspace({ split: DEFAULT_WORKSPACE.split })}
            onKeyDown={(e) => {
              const step = { ArrowLeft: -0.02, ArrowRight: 0.02 }[e.key];
              if (!step) return;
              e.preventDefault();
              e.stopPropagation();
              onWorkspace({ split: clampSplit(workspace.split + step) });
            }}
          />
          <aside className="mix-panel" aria-label={tr('Panel lateral', 'Side panel')}>
            <TrackInfo
              library={library}
              selected={selected}
              open={workspace.info}
              onToggle={() => onWorkspace({ info: !workspace.info })}
              onLoad={onLoad}
              loading={loading}
            />
            <div className="mix-panel-tabs" role="tablist">
              {panels.map((panel) => (
                <button
                  key={panel.id}
                  type="button"
                  role="tab"
                  aria-selected={workspace.panel === panel.id}
                  onClick={() => onWorkspace({ panel: panel.id })}
                >
                  {panel.label}
                </button>
              ))}
            </div>
            <div className="mix-panel-body" role="tabpanel">
              {workspace.panel === 'library' && browser}
              {workspace.panel === 'tools' && (
                <div className="mix-tools">
                  {layout === 'console' ? (
                    <DeckTools
                      library={library}
                      loading={loading}
                      onError={onError}
                      onLibrary={onLibrary}
                    />
                  ) : (
                    <div className="classic-fx">
                      <div>
                        <ColorFxPanel slots={slots} onError={onError} />
                      </div>
                      <div>
                        <BeatFxPanel units={audio?.beatFx ?? []} onError={onError} />
                        <button
                          className="rx-fx-on"
                          aria-pressed={workspace.autoGain}
                          title={tr(
                            'Iguala el nivel de las pistas a −14 LUFS, hasta ±12 dB',
                            'Matches track levels to −14 LUFS, up to ±12 dB',
                          )}
                          onClick={() => {
                            onWorkspace({ autoGain: !workspace.autoGain });
                            call('audio_action', {
                              action: {
                                slot: 0,
                                kind: 'autoGain',
                                value: workspace.autoGain ? 0 : 1,
                              },
                            }).catch(onError);
                          }}
                        >
                          AUTO GAIN
                        </button>
                      </div>
                    </div>
                  )}
                  <SampleBank
                    tracks={library.tracks}
                    keys={keys}
                    onError={onError}
                    onLoad={onLoad}
                    selected={selected}
                  />
                  {layout === 'console' && (
                    <details className="rx-extra">
                      <summary>{tr('Mixer completo de curado', 'Full curado mixer')}</summary>
                      <Mixer keys={keys} onError={onError} />
                    </details>
                  )}
                </div>
              )}
              {workspace.panel === 'keyboard' && (
                <KeyboardGuide keys={keys} onTrigger={onTrigger} />
              )}
            </div>
          </aside>
        </>
      )}
    </div>
  );
}

function SourceSelect({
  view,
  onView,
  playlists,
  devices,
  activeDevice,
  rows,
}: {
  view: string;
  onView: (view: string) => void;
  playlists: LibraryData['playlists'];
  devices: DeviceInfo[];
  activeDevice?: string;
  rows: PlaylistRow[];
}) {
  return (
    <label className="source-select">
      <span>{tr('Fuente', 'Source')}</span>
      <select
        aria-label={tr('Fuente de pistas', 'Track source')}
        value={view}
        onChange={(e) => onView(e.target.value)}
      >
        <option value="all">{tr('toda la música', 'all music')}</option>
        <option value="favorites">{tr('favoritos', 'favorites')}</option>
        {playlists.length > 0 && (
          <optgroup label="playlists">
            {playlists.map((playlist) => (
              <option key={playlist.id} value={`playlist:${playlist.id}`}>
                {playlist.name}
              </option>
            ))}
          </optgroup>
        )}
        {devices.map((device) => (
          <optgroup key={device.id} label={device.name}>
            {device.rekordbox && (
              <option value={deviceViewId(device.id, { kind: 'tracks' })}>
                {tr('todas las pistas', 'all tracks')}
              </option>
            )}
            {device.id === activeDevice &&
              rows
                .filter(({ node }) => !node.folder)
                .map(({ node, depth }) => (
                  <option
                    key={node.id}
                    value={deviceViewId(device.id, { kind: 'playlist', id: node.id })}
                  >
                    {`${'  '.repeat(depth)}${node.name}`}
                  </option>
                ))}
            <option value={deviceViewId(device.id, { kind: 'files' })}>
              {tr('archivos', 'files')}
            </option>
          </optgroup>
        ))}
      </select>
    </label>
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
