import { tr } from './language';
import type { DevicePlaylist, DeviceTrack, RekordboxExportReport } from './types';

export type DeviceSection =
  { kind: 'tracks' } | { kind: 'playlist'; id: number } | { kind: 'files' };
export interface DeviceView {
  device: string;
  section: DeviceSection;
}

/** A volume can be called anything, including text that looks like the rest of a view id. */
export function deviceViewId(device: string, section: DeviceSection): string {
  const name = encodeURIComponent(device);
  if (section.kind === 'playlist') return `device:${name}:playlist:${section.id}`;
  return `device:${name}:${section.kind}`;
}

export function parseDeviceView(view: string): DeviceView | null {
  const [prefix, name, kind, id] = view.split(':');
  if (prefix !== 'device' || !name) return null;
  let device: string;
  try {
    device = decodeURIComponent(name);
  } catch {
    return null;
  }
  if (kind === 'tracks' || kind === 'files') return { device, section: { kind } };
  if (kind === 'playlist' && Number.isInteger(Number(id)))
    return { device, section: { kind, id: Number(id) } };
  return null;
}

export interface PlaylistRow {
  node: DevicePlaylist;
  depth: number;
}
/**
 * Folders and playlists in their rekordbox order. The data comes from a drive, so a corrupt
 * tree with a cycle or a missing parent must still end, and its entries must not vanish.
 */
export function playlistTree(nodes: DevicePlaylist[]): PlaylistRow[] {
  const known = new Set(nodes.map((node) => node.id));
  const children = new Map<number, DevicePlaylist[]>();
  for (const node of nodes) {
    const parent = known.has(node.parent) && node.parent !== node.id ? node.parent : 0;
    children.set(parent, [...(children.get(parent) ?? []), node]);
  }
  const rows: PlaylistRow[] = [];
  const seen = new Set<number>();
  const visit = (parent: number, depth: number) => {
    const list = [...(children.get(parent) ?? [])].sort((a, b) => a.sort - b.sort || a.id - b.id);
    for (const node of list) {
      if (seen.has(node.id)) continue;
      seen.add(node.id);
      rows.push({ node, depth });
      visit(node.id, depth + 1);
    }
  };
  visit(0, 0);
  for (const node of nodes) {
    if (!seen.has(node.id)) {
      seen.add(node.id);
      rows.push({ node, depth: 0 });
    }
  }
  return rows;
}

export function tracksInSection(
  tracks: DeviceTrack[],
  playlists: DevicePlaylist[],
  section: DeviceSection,
): DeviceTrack[] {
  if (section.kind !== 'playlist') return tracks;
  const playlist = playlists.find((node) => node.id === section.id);
  if (!playlist) return [];
  const byId = new Map(tracks.map((track) => [track.id, track]));
  return playlist.tracks.flatMap((id) => {
    const track = byId.get(id);
    return track ? [track] : [];
  });
}

export function matchesQuery(track: DeviceTrack, query: string): boolean {
  const text = `${track.title} ${track.artist} ${track.album} ${track.genre}`;
  return text.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase());
}

/** What the file holds, then what rekordbox will not be able to match to its collection. */
export function exportSummary(report: RekordboxExportReport): string {
  const parts = [
    tr(`XML guardado: ${report.tracks} pistas`, `XML saved: ${report.tracks} tracks`),
    tr(`${report.hotCues} hot cues`, `${report.hotCues} hot cues`),
    tr(`${report.memoryCues} memory cues`, `${report.memoryCues} memory cues`),
    tr(`${report.grids} grids corregidos`, `${report.grids} corrected grids`),
  ];
  if (report.relocated)
    parts.push(
      tr(
        `${report.relocated} del USB enlazadas a su original`,
        `${report.relocated} from USB linked to their original`,
      ),
    );
  if (report.onDevice)
    parts.push(
      tr(
        `${report.onDevice} sin original seguro: rekordbox las verá como otra pista en el USB`,
        `${report.onDevice} with no certain original: rekordbox will see them as another track on the USB`,
      ),
    );
  if (!report.searchComplete)
    parts.push(
      tr('la búsqueda de originales no terminó', 'the search for originals did not finish'),
    );
  return parts.join(' · ');
}
