import { useEffect, useMemo, useRef, useState } from 'react';
import { ArrowUpRight, CornerLeftUp, Disc3, FileAudio, Folder, Search, Usb, X } from 'lucide-react';
import { call } from '../lib/bridge';
import { matchesQuery, tracksInSection } from '../lib/devices';
import type { DeviceSection } from '../lib/devices';
import { tr, useLanguage } from '../lib/language';
import { TrackArt } from './Player';
import { time } from '../lib/types';
import type { DeviceExport, DeviceFolder, DeviceInfo, DeviceTrack } from '../lib/types';

const VISIBLE_ROWS = 500;

function LoadButtons({
  onLoad,
  mix,
  disabled,
}: {
  onLoad: (slot: number) => void;
  mix: boolean;
  disabled: boolean;
}) {
  return (
    <div className="row-load">
      <button
        disabled={disabled}
        onClick={(e) => {
          e.stopPropagation();
          onLoad(0);
        }}
      >
        A<ArrowUpRight size={11} />
      </button>
      {mix && (
        <button
          disabled={disabled}
          onClick={(e) => {
            e.stopPropagation();
            onLoad(1);
          }}
        >
          B<ArrowUpRight size={11} />
        </button>
      )}
    </div>
  );
}

function DeviceRow({
  device,
  track,
  index,
  chosen,
  mix,
  loading,
  onSelect,
  onLoad,
}: {
  device: string;
  track: DeviceTrack;
  index: number;
  chosen: boolean;
  mix: boolean;
  loading: boolean;
  onSelect: (track: number) => void;
  onLoad: (slot: number, track: number) => void;
}) {
  const row = useRef<HTMLTableRowElement>(null);
  useEffect(() => {
    if (chosen) row.current?.scrollIntoView({ block: 'nearest' });
  }, [chosen]);
  return (
    <tr
      ref={row}
      className={chosen ? 'selected' : ''}
      aria-selected={chosen}
      tabIndex={0}
      onClick={() => onSelect(track.id)}
      onDoubleClick={() => onLoad(0, track.id)}
    >
      <td className="number">{String(index + 1).padStart(2, '0')}</td>
      <td>
        <div className="table-title">
          <TrackArt device={device} deviceTrack={track.id} size="sm" />
          <div>
            <strong>{track.title}</strong>
            <small>{track.artist || track.album}</small>
          </div>
        </div>
      </td>
      <td>
        <small>{track.genre}</small>
      </td>
      <td className="duration">{time(track.duration)}</td>
      <td className="mono">{track.bpm > 0 ? track.bpm.toFixed(2) : '—'}</td>
      <td className="mono">{track.key ?? '—'}</td>
      <td>
        <LoadButtons mix={mix} disabled={loading} onLoad={(slot) => onLoad(slot, track.id)} />
      </td>
    </tr>
  );
}

function TrackTable({
  device,
  tracks,
  mix,
  loading,
  selected,
  onSelect,
  onLoad,
}: {
  device: string;
  tracks: DeviceTrack[];
  mix: boolean;
  loading: boolean;
  selected: number | null;
  onSelect: (track: number) => void;
  onLoad: (slot: number, track: number) => void;
}) {
  return (
    <div className="track-table-wrap">
      <table className="track-table">
        <thead>
          <tr>
            <th className="number">#</th>
            <th>{tr('pista / artista', 'track / artist')}</th>
            <th>{tr('género', 'genre')}</th>
            <th>{tr('duración', 'duration')}</th>
            <th>BPM</th>
            <th>key</th>
            <th aria-label={tr('Cargar pista', 'Load track')} />
          </tr>
        </thead>
        <tbody>
          {tracks.slice(0, VISIBLE_ROWS).map((track, i) => (
            <DeviceRow
              device={device}
              key={track.id}
              track={track}
              index={i}
              chosen={track.id === selected}
              mix={mix}
              loading={loading}
              onSelect={onSelect}
              onLoad={onLoad}
            />
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** Folders with audio and audio files only. Everything else on a drive is none of our business. */
function FileBrowser({
  device,
  mix,
  loading,
  onError,
  onLoad,
}: {
  device: DeviceInfo;
  mix: boolean;
  loading: boolean;
  onError: (e: unknown) => void;
  onLoad: (slot: number, entry: number) => void;
}) {
  const [folder, setFolder] = useState<DeviceFolder | null>(null);
  const [opening, setOpening] = useState<number | null>(null);
  const open = (entry: number | null) => {
    setOpening(entry);
    return call<DeviceFolder>('device_browse', { device: device.id, folder: entry })
      .then(setFolder)
      .catch(onError)
      .finally(() => setOpening(null));
  };
  useEffect(() => {
    setFolder(null);
    void open(null);
    // Only a different device starts over; the listing itself holds where we are.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [device.id]);
  if (!folder) {
    return <p className="device-note">{tr('Abriendo…', 'Opening…')}</p>;
  }
  const empty = !folder.folders.length && !folder.files.length;
  return (
    <div className="device-files">
      <div className="device-path">
        {folder.parent !== null && (
          <button
            aria-label={tr('Carpeta superior', 'Parent folder')}
            onClick={() => void open(folder.parent)}
          >
            <CornerLeftUp size={13} />
          </button>
        )}
        <strong>{folder.name}</strong>
      </div>
      <ul aria-busy={opening !== null}>
        {folder.folders.map((entry) => (
          <li key={entry.entry}>
            <button className="device-entry" onClick={() => void open(entry.entry)}>
              <Folder size={14} />
              <span>{entry.name}</span>
            </button>
          </li>
        ))}
        {folder.files.map((entry) => (
          <li key={entry.entry} onDoubleClick={() => onLoad(0, entry.entry)}>
            <span className="device-entry">
              <FileAudio size={14} />
              <span>{entry.name}</span>
            </span>
            <LoadButtons
              mix={mix}
              disabled={loading}
              onLoad={(slot) => onLoad(slot, entry.entry)}
            />
          </li>
        ))}
      </ul>
      {empty && (
        <p className="device-note">
          {tr('No hay audio en esta carpeta.', 'No audio in this folder.')}
        </p>
      )}
      {folder.truncated && (
        <p className="device-note">
          {tr(
            'Esta carpeta tiene demasiados archivos; se muestran los primeros.',
            'This folder has too many files; showing the first ones.',
          )}
        </p>
      )}
    </div>
  );
}

export function DeviceBrowser({
  device,
  section,
  library,
  error,
  mix,
  loading,
  onError,
  selected,
  onSelect,
  onLoadTrack,
  onLoadFile,
}: {
  device: DeviceInfo;
  section: DeviceSection;
  library: DeviceExport | null;
  error: string | null;
  mix: boolean;
  loading: boolean;
  onError: (e: unknown) => void;
  selected: number | null;
  onSelect: (track: number) => void;
  onLoadTrack: (slot: number, track: number) => void;
  onLoadFile: (slot: number, entry: number) => void;
}) {
  useLanguage();
  const [query, setQuery] = useState('');
  const inSection = useMemo(
    () => (library ? tracksInSection(library.tracks, library.playlists, section) : []),
    [library, section],
  );
  const shown = useMemo(
    () => inSection.filter((track) => matchesQuery(track, query)),
    [inSection, query],
  );
  const playlist =
    section.kind === 'playlist' ? library?.playlists.find((node) => node.id === section.id) : null;
  const title =
    section.kind === 'files'
      ? tr('archivos', 'files')
      : (playlist?.name ?? tr('todas las pistas', 'all tracks'));
  return (
    <section className="library device-browser" data-help="devices">
      <div className="library-heading">
        <div>
          <div className="eyebrow">
            <Usb size={11} /> {device.name}
          </div>
          <h2>
            {title}
            {section.kind !== 'files' && <span>{shown.length}</span>}
          </h2>
          <p>
            {section.kind === 'files'
              ? tr(
                  'Carpetas con audio de este dispositivo. curado no escribe nada en él.',
                  'Folders with audio on this device. curado writes nothing to it.',
                )
              : tr(
                  'Biblioteca de rekordbox con sus grids y cues. curado no escribe nada en el dispositivo.',
                  'rekordbox library with its grids and cues. curado writes nothing to the device.',
                )}
          </p>
        </div>
      </div>
      {section.kind === 'files' ? (
        <FileBrowser
          device={device}
          mix={mix}
          loading={loading}
          onError={onError}
          onLoad={onLoadFile}
        />
      ) : (
        <>
          <div className="library-toolbar">
            <label className="search">
              <Search size={15} />
              <input
                aria-label={tr('Buscar en el dispositivo', 'Search this device')}
                placeholder={tr(
                  'Buscar por canción, artista o género…',
                  'Search by song, artist or genre…',
                )}
                value={query}
                onChange={(e) => setQuery(e.target.value)}
              />
              {query && (
                <button
                  aria-label={tr('Limpiar búsqueda', 'Clear search')}
                  onClick={() => setQuery('')}
                >
                  <X size={13} />
                </button>
              )}
            </label>
            <small>
              {shown.length} {tr('PISTAS ·', 'TRACKS ·')}{' '}
              {time(shown.reduce((sum, track) => sum + track.duration, 0))}
            </small>
          </div>
          {error ? (
            <p className="error-text device-note">{error}</p>
          ) : !library ? (
            <p className="device-note">{tr('Leyendo biblioteca…', 'Reading library…')}</p>
          ) : (
            <TrackTable
              device={device.id}
              tracks={shown}
              mix={mix}
              loading={loading}
              selected={selected}
              onSelect={onSelect}
              onLoad={onLoadTrack}
            />
          )}
          {library && !shown.length && (
            <div className="empty">
              <div className="empty-disc">
                <Disc3 size={30} strokeWidth={1} />
              </div>
              <h3>
                {query
                  ? tr('No encontramos esa pista', 'No tracks found')
                  : tr('Sin pistas', 'No tracks')}
              </h3>
            </div>
          )}
          {shown.length > VISIBLE_ROWS && (
            <p className="device-note">
              {tr(
                `Mostrando ${VISIBLE_ROWS} pistas. Usa la búsqueda.`,
                `Showing ${VISIBLE_ROWS} tracks. Use search to find more.`,
              )}
            </p>
          )}
        </>
      )}
    </section>
  );
}
