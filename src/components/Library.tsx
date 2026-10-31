import { tr, useLanguage } from '../lib/language';
import { libraryValue, sortTracks } from '../lib/sorting';
import type { SortColumn, TrackSort } from '../lib/sorting';
import { ariaSort, SortButton, SortReset } from './SortButton';
import { memo, useEffect, useMemo, useRef, useState } from 'react';
import {
  Search,
  ArrowUpRight,
  Star,
  Play,
  X,
  Music2,
  FolderOpen,
  Disc3,
  Share,
} from 'lucide-react';
import type { Library as LibraryData, LibraryEdit, Track } from '../lib/types';
import { TrackArt } from './Player';
import { time } from '../lib/types';
import { keyLabel } from '../lib/music';
export const Library = memo(function Library({
  data,
  view,
  selected,
  sort,
  onSort,
  listOrder,
  onSelect,
  onLoad,
  onEdit,
  onImport,
  onExport,
  onQueue,
  busy,
  mode,
}: {
  data: LibraryData;
  view: string;
  selected: number | null;
  sort: TrackSort | null;
  onSort: (column: SortColumn | null) => void;
  /** What the list shows, in its order, for the keys that walk it. */
  listOrder: { current: number[] | null };
  onSelect: (id: number) => void;
  onLoad: (slot: number, id: number) => void;
  onEdit: (edit: LibraryEdit) => void;
  onImport: (folder: boolean) => void;
  onExport: () => void;
  onQueue: (ids: number[], repeat: 'off' | 'one' | 'all') => void;
  busy: string | null;
  mode: 'player' | 'mix';
}) {
  useLanguage();
  const [query, setQuery] = useState('');
  const [repeat, setRepeat] = useState<'off' | 'one' | 'all'>('off');
  const [playlist, setPlaylist] = useState('');
  const active = data.playlists.find((p) => `playlist:${p.id}` === view);
  const filtered = useMemo(() => {
    let tracks = data.tracks;
    if (active) {
      const byId = new Map(tracks.map((t) => [t.id, t]));
      tracks = active.tracks.flatMap((id) => (byId.has(id) ? [byId.get(id)!] : []));
    }
    if (view === 'favorites') tracks = tracks.filter((t) => t.favorite);
    const q = query.toLocaleLowerCase();
    return sortTracks(
      tracks.filter((t) => `${t.title} ${t.artist}`.toLocaleLowerCase().includes(q)),
      sort,
      libraryValue,
    );
  }, [data, active, view, query, sort]);
  listOrder.current = filtered.slice(0, 500).map((t) => t.id);
  useEffect(
    () => () => {
      listOrder.current = null;
    },
    [listOrder],
  );
  const selection = data.tracks.find((t) => t.id === selected);
  const subtitle = active
    ? tr('Tu selección, en el orden que elegiste.', 'Your selection, in the order you chose.')
    : view === 'favorites'
      ? tr('Las pistas a las que siempre vuelves.', 'The tracks you keep coming back to.')
      : tr(
          'Tu colección local. Lista para escuchar o mezclar.',
          'Your local collection. Ready to listen or mix.',
        );
  return (
    <section className="library">
      <div className="library-heading">
        <div>
          <div className="eyebrow">{tr('tu espacio sonoro', 'your sound space')}</div>
          <h2>
            {active?.name ??
              (view === 'favorites' ? tr('favoritos', 'favorites') : tr('biblioteca', 'library'))}
            <span>{filtered.length}</span>
          </h2>
          <p>{subtitle}</p>
        </div>
        <div className="library-actions">
          {mode === 'player' && filtered.length > 0 && (
            <select
              aria-label={tr('Repetición de cola', 'Queue repeat')}
              value={repeat}
              onChange={(e) => setRepeat(e.target.value as 'off' | 'one' | 'all')}
            >
              <option value="off">{tr('Repetir: no', 'Repeat: off')}</option>
              <option value="all">{tr('Repetir: todo', 'Repeat: all')}</option>
              <option value="one">{tr('Repetir: pista', 'Repeat: track')}</option>
            </select>
          )}
          {mode === 'player' && filtered.length > 0 && (
            <button
              onClick={() =>
                onQueue(
                  filtered.map((t) => t.id),
                  repeat,
                )
              }
            >
              <Play size={12} />
              {tr('Escuchar selección', 'Play selection')}{' '}
            </button>
          )}
          <button
            onClick={onExport}
            disabled={!!busy}
            aria-label={tr('A rekordbox', 'To rekordbox')}
            title={tr(
              'Guarda un rekordbox.xml con tus cues, loops y grids corregidos. No escribe en ningún USB.',
              'Saves a rekordbox.xml with your cues, loops and corrected grids. Writes to no USB drive.',
            )}
          >
            <Share size={14} />
            <span className="action-label">{tr('A rekordbox', 'To rekordbox')}</span>
          </button>
          <button
            onClick={() => onImport(true)}
            disabled={!!busy}
            aria-label={tr('Carpeta', 'Folder')}
            title={tr('Importar una carpeta', 'Import a folder')}
          >
            <FolderOpen size={14} />
            <span className="action-label">{tr('Carpeta', 'Folder')}</span>
          </button>
          <button
            className="primary"
            onClick={() => onImport(false)}
            disabled={!!busy}
            aria-label={tr('Importar música', 'Import music')}
            title={tr('Importar música', 'Import music')}
          >
            <span>+</span>
            <span className="action-label">{tr('Importar música', 'Import music')}</span>
          </button>
        </div>
      </div>
      <div className="library-toolbar">
        <label className="search">
          <Search size={15} />
          <input
            aria-label={tr('Buscar música', 'Search music')}
            placeholder={tr('Buscar por canción o artista…', 'Search by song or artist…')}
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
          {filtered.length} {tr('PISTAS ·', 'TRACKS ·')}{' '}
          {time(filtered.reduce((n, t) => n + t.duration, 0))}
        </small>
        {active && (
          <button
            className="subtle danger"
            onClick={() => {
              if (
                confirm(
                  tr(
                    `¿Eliminar la playlist «${active.name}»? Tus archivos y pistas se conservan.`,
                    `Delete playlist “${active.name}”? Your files and tracks will be kept.`,
                  ),
                )
              )
                onEdit({ kind: 'deletePlaylist', id: active.id });
            }}
          >
            {tr('Eliminar playlist', 'Delete playlist')}{' '}
          </button>
        )}
      </div>
      {selection && (
        <div className="selection-toolbar">
          <span>
            <Music2 size={13} />
            <b>{selection.title}</b>
          </span>
          <button onClick={() => onLoad(0, selection.id)}>
            {tr('Cargar en A', 'Load into A')}
            <ArrowUpRight size={13} />
          </button>
          {mode === 'mix' && (
            <button onClick={() => onLoad(1, selection.id)}>
              {tr('Cargar en B', 'Load into B')}
              <ArrowUpRight size={13} />
            </button>
          )}
          <select
            aria-label={tr('Añadir selección a playlist', 'Add selection to playlist')}
            value={playlist}
            onChange={(e) => {
              setPlaylist('');
              if (e.target.value)
                onEdit({ kind: 'addToPlaylist', playlist: +e.target.value, track: selection.id });
            }}
          >
            <option value="">{tr('+ Añadir a playlist', '+ Add to playlist')}</option>
            {data.playlists.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
          <button
            className="subtle"
            title={tr(
              'Retirar de biblioteca sin borrar el archivo',
              'Remove from library without deleting the file',
            )}
            onClick={() => {
              if (
                confirm(
                  tr(
                    `¿Retirar «${selection.title}» de la biblioteca? El archivo se conserva.`,
                    `Remove “${selection.title}” from the library? The file will be kept.`,
                  ),
                )
              )
                onEdit({ kind: 'removeTrack', track: selection.id });
            }}
          >
            {tr('Retirar', 'Remove')}{' '}
          </button>
        </div>
      )}
      <div className="track-table-wrap">
        <table className="track-table">
          <thead>
            <tr>
              <th className="number">
                <SortReset onSort={onSort} />
              </th>
              <th aria-sort={ariaSort(sort, 'title', 'artist')}>
                <SortButton
                  column="title"
                  label={tr('pista', 'track')}
                  sort={sort}
                  onSort={onSort}
                />
                {' / '}
                <SortButton
                  column="artist"
                  label={tr('artista', 'artist')}
                  sort={sort}
                  onSort={onSort}
                />
              </th>
              <th aria-sort={ariaSort(sort, 'format')}>
                <SortButton
                  column="format"
                  label={tr('formato', 'format')}
                  sort={sort}
                  onSort={onSort}
                />
              </th>
              <th aria-sort={ariaSort(sort, 'duration')}>
                <SortButton
                  column="duration"
                  label={tr('duración', 'duration')}
                  sort={sort}
                  onSort={onSort}
                />
              </th>
              <th aria-sort={ariaSort(sort, 'bpm')}>
                <SortButton column="bpm" label="BPM" sort={sort} onSort={onSort} />
              </th>
              <th aria-sort={ariaSort(sort, 'key')}>
                <SortButton column="key" label="key / Camelot" sort={sort} onSort={onSort} />
              </th>
              <th aria-label={tr('Favoritos', 'Favorites')} />
              <th aria-label={tr('Cargar pista', 'Load track')} />
            </tr>
          </thead>
          <tbody>
            {filtered.slice(0, 500).map((track, i) => (
              <TrackRow
                key={track.id}
                track={track}
                index={i}
                selected={track.id === selected}
                onSelect={onSelect}
                onLoad={onLoad}
                onEdit={onEdit}
                activeId={active?.id}
                mix={mode === 'mix'}
              />
            ))}
          </tbody>
        </table>
        {!filtered.length && (
          <div className="empty">
            <div className="empty-disc">
              <Disc3 size={30} strokeWidth={1} />
            </div>
            <h3>
              {query
                ? tr('No encontramos esa pista', 'No tracks found')
                : data.tracks.length
                  ? tr('Esta selección está vacía', 'This selection is empty')
                  : tr('Todo empieza con una canción', 'It all starts with a song')}
            </h3>
            <p>
              {query
                ? tr('Prueba con otro título o artista.', 'Try another title or artist.')
                : tr(
                    'Importa tus archivos o carga la sesión de prueba desde el menú lateral.',
                    'Import your files or load the demo session from the sidebar.',
                  )}
            </p>
            {!data.tracks.length && (
              <button onClick={() => onImport(false)} disabled={!!busy}>
                {tr('Elegir archivos', 'Choose files')} <ArrowUpRight size={14} />
              </button>
            )}
          </div>
        )}
      </div>
      <div className="library-foot">
        <span>
          <span className="status-dot active" />
          {tr(
            'Archivos locales · tus originales se conservan',
            'Local files · your originals stay untouched',
          )}{' '}
        </span>
        <span>
          {filtered.length > 500
            ? tr(
                'Mostrando 500 pistas. Usa la búsqueda.',
                'Showing 500 tracks. Use search to find more.',
              )
            : 'MP3 · FLAC · WAV · AAC · ALAC · AIFF · OGG'}
        </span>
      </div>
    </section>
  );
});
function TrackRow({
  track,
  index,
  selected,
  onSelect,
  onLoad,
  onEdit,
  activeId,
  mix,
}: {
  track: Track;
  index: number;
  selected: boolean;
  onSelect: (id: number) => void;
  onLoad: (slot: number, id: number) => void;
  onEdit: (edit: LibraryEdit) => void;
  activeId?: number;
  mix: boolean;
}) {
  const row = useRef<HTMLTableRowElement>(null);
  useEffect(() => {
    if (selected) row.current?.scrollIntoView({ block: 'nearest' });
  }, [selected]);
  return (
    <tr
      ref={row}
      className={selected ? 'selected' : ''}
      onClick={() => onSelect(track.id)}
      onDoubleClick={() => onLoad(0, track.id)}
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === 'Enter') {
          e.stopPropagation();
          onSelect(track.id);
        }
      }}
      aria-selected={selected}
    >
      <td className="number">{String(index + 1).padStart(2, '0')}</td>
      <td>
        <div className="table-title">
          <TrackArt id={track.id} size="sm" />
          <div>
            <strong>{track.title}</strong>
            <small>{track.artist}</small>
          </div>
        </div>
      </td>
      <td>
        <span className="format">{track.path.split('.').pop()?.toUpperCase()}</span>
        <small className="rate">{(track.sampleRate / 1000).toFixed(1)} kHz</small>
      </td>
      <td className="duration">{time(track.duration)}</td>
      <td className="mono">{track.music?.bpm?.toFixed(1) ?? '—'}</td>
      <td
        className="mono"
        title={
          track.music?.manual
            ? tr('Corrección manual', 'Manual correction')
            : tr(
                'Estimación automática; corrige en el deck',
                'Automatic estimate; correct it in the deck',
              )
        }
      >
        {keyLabel(track.music?.key ?? null)}{' '}
        <small>{keyLabel(track.music?.key ?? null, 'camelot')}</small>
      </td>
      <td>
        <button
          className={`icon-button star ${track.favorite ? 'is-favorite' : ''}`}
          aria-label={
            track.favorite
              ? tr(`Quitar de favoritos ${track.title}`, `Remove ${track.title} from favorites`)
              : tr(`Añadir a favoritos ${track.title}`, `Add ${track.title} to favorites`)
          }
          onClick={(e) => {
            e.stopPropagation();
            onEdit({ kind: 'favorite', track: track.id, value: !track.favorite });
          }}
        >
          <Star size={14} fill={track.favorite ? 'currentColor' : 'none'} />
        </button>
      </td>
      <td>
        <div className="row-load">
          <button
            onClick={(e) => {
              e.stopPropagation();
              onLoad(0, track.id);
            }}
          >
            A<ArrowUpRight size={11} />
          </button>
          {mix && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                onLoad(1, track.id);
              }}
            >
              B<ArrowUpRight size={11} />
            </button>
          )}
          {activeId && (
            <button
              aria-label={tr(
                `Retirar ${track.title} de playlist`,
                `Remove ${track.title} from playlist`,
              )}
              onClick={(e) => {
                e.stopPropagation();
                onEdit({ kind: 'removeFromPlaylist', playlist: activeId, track: track.id });
              }}
            >
              <X size={12} />
            </button>
          )}
        </div>
      </td>
    </tr>
  );
}
