import { expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import {
  deviceViewId,
  exportSummary,
  matchesQuery,
  parseDeviceView,
  playlistTree,
  tracksInSection,
} from './devices';
import { cueColor } from './cues';
import type { DevicePlaylist, DeviceTrack } from './types';

const node = (id: number, parent: number, sort: number, folder = false): DevicePlaylist => ({
  id,
  parent,
  sort,
  folder,
  name: `n${id}`,
  tracks: [],
});
const track = (id: number, title = `t${id}`): DeviceTrack => ({
  id,
  title,
  artist: 'Artista',
  album: 'Álbum',
  genre: 'House',
  key: '8A',
  bpm: 124,
  duration: 200,
  rating: 0,
});

test('view ids survive any volume name', () => {
  for (const name of ['USB', 'MI USB:2', 'a/b', 'música 100%', 'device:x:files']) {
    for (const section of [
      { kind: 'tracks' },
      { kind: 'files' },
      { kind: 'playlist', id: 7 },
    ] as const) {
      expect(parseDeviceView(deviceViewId(name, section))).toEqual({ device: name, section });
    }
  }
  for (const other of [
    'all',
    'favorites',
    'playlist:3',
    'web',
    'device:',
    'device:%E0%A4%A:tracks',
  ])
    expect(parseDeviceView(other)).toBeNull();
  expect(parseDeviceView('device:USB:playlist:x')).toBeNull();
});

test('the playlist tree keeps rekordbox order and depth', () => {
  const rows = playlistTree([node(3, 1, 2), node(1, 0, 1, true), node(2, 1, 1), node(4, 0, 0)]);
  expect(rows.map((r) => [r.node.id, r.depth])).toEqual([
    [4, 0],
    [1, 0],
    [2, 1],
    [3, 1],
  ]);
});

test('a corrupt playlist tree still ends and loses nothing', () => {
  const rows = playlistTree([node(1, 2, 0), node(2, 1, 0), node(3, 3, 0), node(4, 99, 0)]);
  expect(rows.map((r) => r.node.id).sort()).toEqual([1, 2, 3, 4]);
});

test('sections list their tracks in playlist order and ignore dangling entries', () => {
  const tracks = [track(1), track(2), track(3)];
  const list: DevicePlaylist = { ...node(9, 0, 0), tracks: [3, 77, 1] };
  expect(tracksInSection(tracks, [list], { kind: 'tracks' })).toHaveLength(3);
  expect(tracksInSection(tracks, [list], { kind: 'playlist', id: 9 }).map((t) => t.id)).toEqual([
    3, 1,
  ]);
  expect(tracksInSection(tracks, [list], { kind: 'playlist', id: 8 })).toEqual([]);
  expect(matchesQuery(track(1, 'Canción'), ' canCIÓN ')).toBe(true);
  expect(matchesQuery(track(1), 'house')).toBe(true);
  expect(matchesQuery(track(1), 'techno')).toBe(false);
});

test('hot cue colors are valid CSS or absent', () => {
  expect(cueColor({ color: [10, 20, 30], label: null })).toBe('rgb(10 20 30)');
  expect(cueColor({ color: null, label: 'x' })).toBeUndefined();
  expect(cueColor(null)).toBeUndefined();
  expect(cueColor(undefined)).toBeUndefined();
});

test('the export report warns about tracks rekordbox will not match', () => {
  const report = {
    saved: true,
    tracks: 3,
    changed: 2,
    hotCues: 4,
    memoryCues: 1,
    grids: 1,
    playlists: 2,
    relocated: 0,
    onDevice: 0,
    searchComplete: true,
  };
  expect(exportSummary(report)).toContain('3');
  expect(exportSummary(report)).not.toMatch(/USB/);
  const risky = exportSummary({ ...report, relocated: 1, onDevice: 2, searchComplete: false });
  expect(risky).toMatch(/\b1\b.*USB/);
  expect(risky).toMatch(/\b2\b.*USB/);
});

test('the export destination comes from the native dialog alone', () => {
  const app = readFileSync('src-tauri/src/lib.rs', 'utf8');
  const command = app.slice(app.indexOf('async fn export_rekordbox_xml'));
  const signature = command.slice(0, command.indexOf('->'));
  expect(signature).not.toContain('path');
  expect(command).toContain('save_file()');
  expect(readFileSync('src/App.tsx', 'utf8')).toContain("call<RekordboxExportReport>('export_rekordbox_xml')");
  const source = readFileSync('src-tauri/src/rekordbox_xml.rs', 'utf8').split('#[cfg(test)]')[0];
  // The only write in the export goes through the destination it was handed.
  expect(source.match(/fs::write|File::create|OpenOptions/g)).toEqual(['fs::write']);
  expect(source).toContain('fs::write(&temporary, text)');
});

test('the interface never handles paths on a device', () => {
  const read = (file: string) => readFileSync(file, 'utf8');
  const browser = read('src/components/DeviceBrowser.tsx');
  for (const command of ['device_browse']) expect(browser).toContain(command);
  // Native code resolves everything: the commands take a device name, a track id or an entry.
  const app = read('src-tauri/src/lib.rs');
  const devices = app.slice(
    app.indexOf('async fn device_library'),
    app.indexOf('async fn track_cue_info'),
  );
  expect(devices).not.toMatch(/path:\s*(String|PathBuf)/);
  expect(devices).toContain('entry: u64');
  expect(devices).toContain('track: u32');
  // Nothing in the device module opens a file for writing.
  const source =
    read('src-tauri/src/rekordbox.rs') +
    read('src-tauri/src/devices.rs').split('#[cfg(all(test, unix))]')[0];
  expect(source).not.toMatch(/fs::write|File::create|OpenOptions|create_dir|remove_file|rename\(/);
});
