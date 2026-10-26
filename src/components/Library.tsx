import { memo, useMemo, useState } from 'react';
import { Search, ArrowUpRight, Star, Play, X, Music2, FolderOpen, Disc3 } from 'lucide-react';
import type { Library as LibraryData, LibraryEdit, Track } from '../lib/types';
import { TrackArt } from './Player';
import { time } from '../lib/types';
export const Library = memo(function Library({
  data,
  view,
  selected,
  onSelect,
  onLoad,
  onEdit,
  onImport,
  onQueue,
  busy,
  mode,
}: {
  data: LibraryData;
  view: string;
  selected: number | null;
  onSelect: (id: number) => void;
  onLoad: (slot: number, id: number) => void;
  onEdit: (edit: LibraryEdit) => void;
  onImport: (folder: boolean) => void;
  onQueue: (ids: number[], repeat: 'off' | 'one' | 'all') => void;
  busy: string | null;
  mode: 'player' | 'mix';
}) {
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
    return tracks.filter((t) => `${t.title} ${t.artist}`.toLocaleLowerCase().includes(q));
  }, [data, active, view, query]);
  const selection = data.tracks.find((t) => t.id === selected);
  const subtitle = active
    ? 'Tu selección, en el orden que elegiste.'
    : view === 'favorites'
      ? 'Las pistas a las que siempre vuelves.'
      : 'Tu colección local. Lista para escuchar o mezclar.';
  return (
    <section className="library">
      <div className="library-heading">
        <div>
          <div className="eyebrow">tu espacio sonoro</div>
          <h2>
            {active?.name ?? (view === 'favorites' ? 'favoritos' : 'biblioteca')}
            <span>{filtered.length}</span>
          </h2>
          <p>{subtitle}</p>
        </div>
        <div className="library-actions">
          {mode === 'player' && filtered.length > 0 && (
            <select
              aria-label="Repetición de cola"
              value={repeat}
              onChange={(e) => setRepeat(e.target.value as 'off' | 'one' | 'all')}
            >
              <option value="off">Repetir: no</option>
              <option value="all">Repetir: todo</option>
              <option value="one">Repetir: pista</option>
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
              Escuchar selección
            </button>
          )}
          <button onClick={() => onImport(true)} disabled={!!busy}>
            <FolderOpen size={14} />
            Carpeta
          </button>
          <button className="primary" onClick={() => onImport(false)} disabled={!!busy}>
            <span>+</span>Importar música
          </button>
        </div>
      </div>
      <div className="library-toolbar">
        <label className="search">
          <Search size={15} />
          <input
            aria-label="Buscar música"
            placeholder="Buscar por canción o artista…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          {query && (
            <button aria-label="Limpiar búsqueda" onClick={() => setQuery('')}>
              <X size={13} />
            </button>
          )}
        </label>
        <small>
          {filtered.length} PISTAS · {time(filtered.reduce((n, t) => n + t.duration, 0))}
        </small>
        {active && (
          <button
            className="subtle danger"
            onClick={() => {
              if (
                confirm(
                  `¿Eliminar la playlist «${active.name}»? Tus archivos y pistas se conservan.`,
                )
              )
                onEdit({ kind: 'deletePlaylist', id: active.id });
            }}
          >
            Eliminar playlist
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
            Cargar en A<ArrowUpRight size={13} />
          </button>
          {mode === 'mix' && (
            <button onClick={() => onLoad(1, selection.id)}>
              Cargar en B<ArrowUpRight size={13} />
            </button>
          )}
          <select
            aria-label="Añadir selección a playlist"
            value={playlist}
            onChange={(e) => {
              setPlaylist('');
              if (e.target.value)
                onEdit({ kind: 'addToPlaylist', playlist: +e.target.value, track: selection.id });
            }}
          >
            <option value="">+ Añadir a playlist</option>
            {data.playlists.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name}
              </option>
            ))}
          </select>
          <button
            className="subtle"
            title="Retirar de biblioteca sin borrar el archivo"
            onClick={() => {
              if (
                confirm(`¿Retirar «${selection.title}» de la biblioteca? El archivo se conserva.`)
              )
                onEdit({ kind: 'removeTrack', track: selection.id });
            }}
          >
            Retirar
          </button>
        </div>
      )}
      <div className="track-table-wrap">
        <table className="track-table">
          <thead>
            <tr>
              <th className="number">#</th>
              <th>pista / artista</th>
              <th>formato</th>
              <th>duración</th>
              <th aria-label="Favoritos" />
              <th aria-label="Cargar pista" />
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
                ? 'No encontramos esa pista'
                : data.tracks.length
                  ? 'Esta selección está vacía'
                  : 'Todo empieza con una canción'}
            </h3>
            <p>
              {query
                ? 'Prueba con otro título o artista.'
                : 'Importa tus archivos o carga la sesión de prueba desde el menú lateral.'}
            </p>
            {!data.tracks.length && (
              <button onClick={() => onImport(false)} disabled={!!busy}>
                Elegir archivos
                <ArrowUpRight size={14} />
              </button>
            )}
          </div>
        )}
      </div>
      <div className="library-foot">
        <span>
          <span className="status-dot active" />
          Archivos locales · tus originales se conservan
        </span>
        <span>
          {filtered.length > 500
            ? 'Mostrando 500 pistas. Usa la búsqueda.'
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
  return (
    <tr
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
      <td>
        <button
          className={`icon-button star ${track.favorite ? 'is-favorite' : ''}`}
          aria-label={`${track.favorite ? 'Quitar de' : 'Añadir a'} favoritos ${track.title}`}
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
              aria-label={`Retirar ${track.title} de playlist`}
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
