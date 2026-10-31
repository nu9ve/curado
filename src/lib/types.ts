export interface Track {
  id: number;
  path: string;
  title: string;
  artist: string;
  duration: number;
  sampleRate: number;
  favorite: boolean;
  music: MusicAnalysis | null;
}
export interface MusicalKey {
  tonic: number;
  minor: boolean;
}
export interface MusicAnalysis {
  version: number;
  bpm: number | null;
  beatOffset: number;
  bpmConfidence: number;
  key: MusicalKey | null;
  keyConfidence: number;
  manual: boolean;
  /** Seconds of a bar's first beat; 0 when unknown, which reads as the first grid line. */
  downbeat: number;
  /** Seconds of the first audible sound: where an unsaved cue starts. */
  firstSound: number;
}
export interface Tuning {
  rate: number;
  semitones: number;
  keyLock: boolean;
}
export const EMPTY_MUSIC: MusicAnalysis = {
  version: 2,
  bpm: null,
  beatOffset: 0,
  bpmConfidence: 0,
  key: null,
  keyConfidence: 0,
  manual: false,
  downbeat: 0,
  firstSound: 0,
};
export const DEFAULT_TUNING: Tuning = { rate: 1, semitones: 0, keyLock: true };
export interface Playlist {
  id: number;
  name: string;
  tracks: number[];
}
export interface Library {
  tracks: Track[];
  playlists: Playlist[];
}
export interface Action {
  slot: number;
  kind: string;
  value: number;
}
export interface KeyBinding {
  code: string;
  shift: boolean;
  action: Action;
}
export interface MidiBinding {
  device: string;
  status: number;
  channel: number;
  number: number;
  action: Action;
}
export interface MidiState {
  connected: string | null;
  bindings: MidiBinding[];
  learning: Action | null;
  lastMessage: string | null;
  error: string | null;
  dropped: number;
}
export interface Slot {
  id: number;
  position: number;
  duration: number;
  playing: boolean;
  looping: boolean;
  peak: number;
  gain: number;
  rate: number;
  low: number;
  mid: number;
  high: number;
  filter: number;
  echo: number;
  highPass: number;
  echoBeats: number;
  cues: number[];
  loopIn: number;
  loopOut: number;
  loadTicket: number;
  music: MusicAnalysis;
  tuning: Tuning;
  quantize: boolean;
  sync: boolean;
  loopBeats: number;
  queued: boolean;
}
export interface Audio {
  slots: Slot[];
  peak: number;
  rms: number;
  waveform: number[];
  crossfader: number;
  master: number;
  streamError: boolean;
  callbacks: number;
  clipped: number;
  limited: number;
  limiterDb: number;
  masterDeck: number;
}
export interface Output {
  device: string;
  sampleRate: number;
  channels: number;
  bufferFrames: number | null;
  backend: string;
}
export interface QueueStatus {
  count: number;
  index: number;
  active: boolean;
  repeat: string;
  error: string | null;
}
export interface AppSnapshot {
  mode: 'player' | 'mix';
  persistenceError: string | null;
  queue: QueueStatus;
  audio: Audio | null;
  output: Output | null;
  error: string | null;
  midi: MidiState;
}
export interface ImportReport {
  added: number;
  skipped: number;
  errors: string[];
}
export type LibraryEdit =
  | { kind: 'createPlaylist'; name: string }
  | { kind: 'renamePlaylist'; id: number; name: string }
  | { kind: 'deletePlaylist'; id: number }
  | { kind: 'addToPlaylist' | 'removeFromPlaylist'; playlist: number; track: number }
  | { kind: 'favorite'; track: number; value: boolean }
  | { kind: 'removeTrack'; track: number };
export const EMPTY_SLOT: Slot = {
  loadTicket: 0,
  music: EMPTY_MUSIC,
  tuning: DEFAULT_TUNING,
  quantize: true,
  sync: false,
  loopBeats: 4,
  queued: false,
  id: 0,
  position: 0,
  duration: 0,
  playing: false,
  looping: false,
  peak: 0,
  gain: 0.8,
  rate: 1,
  low: 1,
  mid: 1,
  high: 1,
  filter: 1,
  echo: 0,
  highPass: 0,
  echoBeats: 0.5,
  cues: [0, 0, 0],
  loopIn: 0,
  loopOut: 0,
};
export const time = (seconds: number) =>
  `${Math.floor(seconds / 60)
    .toString()
    .padStart(2, '0')}:${Math.floor(seconds % 60)
    .toString()
    .padStart(2, '0')}`;
/** Seconds remaining, shown as −mm:ss like a club deck. */
export const remaining = (slot: Pick<Slot, 'position' | 'duration'>) =>
  `−${time(Math.max(0, slot.duration - slot.position))}`;
export const deckName = (slot: number) => (slot < 2 ? (slot ? 'B' : 'A') : `S${slot - 1}`);
