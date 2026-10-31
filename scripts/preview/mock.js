// Stand-in for the native side, with invented tracks, for tests and documentation pictures.
(() => {
  const params = new URLSearchParams(location.search);
  const THEME = JSON.parse(localStorage.getItem('__shot_theme') || 'null');
  const TITLES = [
    ['Cinder Road', 'Nadia Vale'],
    ['Glass Harbor', 'Otto Renn'],
    ['Slow Comet', 'Ines Marlow'],
    ['Paper Sun', 'The Low Tides'],
    ['Night Ferry', 'Kaito Mere'],
    ['Velvet Static', 'Ana Duarte'],
    ['Saltwater Radio', 'Otto Renn'],
    ['Hollow Bloom', 'Lumen Park'],
    ['Second Orbit', 'Nadia Vale'],
    ['Copper Lines', 'Sol Ferrer'],
    ['Quiet Engines', 'The Low Tides'],
    ['After Rain', 'Ines Marlow'],
    ['North Pier', 'Kaito Mere'],
    ['Dust Parade', 'Lumen Park'],
    ['Soft Machine Room', 'Ana Duarte'],
    ['Red Lantern', 'Sol Ferrer'],
    ['Long Exposure', 'Otto Renn'],
    ['Tin Roof', 'Nadia Vale'],
    ['Moth Season', 'The Low Tides'],
    ['Afterglow Drive', 'Kaito Mere'],
    ['Sable', 'Ana Duarte'],
    ['Open Water', 'Lumen Park'],
    ['Static Garden', 'Sol Ferrer'],
    ['Last Tram', 'Ines Marlow'],
  ];
  const cbs = {};
  let n = 1;
  const music = (i) => ({
    version: 3,
    bpm: [124, 128, 122, 126, 118, 130, 125][i % 7],
    beatOffset: 0.12,
    bpmConfidence: 0.92,
    key: { tonic: (i * 5) % 12, minor: i % 3 !== 0 },
    keyConfidence: 0.8,
    manual: false,
    downbeat: 0.12,
    firstSound: 0.1,
    imported: i % 3 === 0,
  });
  const tracks = TITLES.map(([title, artist], i) => ({
    id: i + 1,
    title,
    artist,
    path: `/music/${i}.flac`,
    duration: 200 + ((i * 37) % 180),
    sampleRate: 44100,
    favorite: i % 5 === 0,
    music: music(i),
    device: false,
  }));
  const library = {
    tracks,
    playlists: [
      { id: 1, name: 'warm up', tracks: [1, 2, 3, 4, 5] },
      { id: 2, name: 'peak time', tracks: [6, 7, 8] },
      { id: 3, name: 'closing', tracks: [9, 10] },
    ],
  };
  const slot = (id, o = {}) => ({
    trim: 1,
    color: 0,
    colorParam: 0.5,
    colorFx: 0,
    brake: 0,
    slip: false,
    reverse: false,
    rolling: false,
    loadTicket: id,
    music: id
      ? music(id - 1)
      : {
          version: 0,
          bpm: null,
          beatOffset: 0,
          bpmConfidence: 0,
          key: null,
          keyConfidence: 0,
          manual: false,
          downbeat: 0,
          firstSound: 0,
          imported: false,
        },
    tuning: { rate: 1, semitones: 0, keyLock: false },
    quantize: true,
    sync: false,
    loopBeats: 4,
    queued: false,
    id,
    position: id ? 92 : 0,
    duration: id ? tracks[id - 1].duration : 0,
    playing: !!id,
    looping: false,
    peak: id ? 0.55 : 0,
    gain: 0.8,
    rate: 1,
    low: 1,
    mid: 1,
    high: 1,
    filter: 1,
    echo: 0,
    highPass: 0,
    echoBeats: 0.5,
    cues: id ? [14, 46, 92, 124, 0, 0, 0, 0] : Array(8).fill(0),
    cueEnds: Array(8).fill(0),
    loopIn: 0,
    loopOut: 0,
    ...o,
  });
  const state = {
    devices: [{ id: 'SETS', name: 'SETS', rekordbox: true }],
    mode: params.get('mode') || 'mix',
    persistenceError: null,
    queue: { count: 0, index: 0, active: false, repeat: 'off', error: null },
    audio: {
      beatFx: [
        { kind: 1, on: false, beats: 1, depth: 0.5, bpm: 0 },
        { kind: 3, on: false, beats: 0.5, depth: 0.7, bpm: 0 },
        { kind: 7, on: false, beats: 1, depth: 1, bpm: 128 },
      ],
      sampler: 1,
      autoGain: true,
      crossfaderCurve: 1,
      cueMaster: false,
      cueMix: 0,
      cueLevel: 0.7,
      slots: [
        slot(1, { looping: true, loopIn: 88, loopOut: 95.74 }),
        slot(6, {
          position: 146,
          peak: 0.42,
          sync: true,
          tuning: { rate: 0.954, semitones: 0, keyLock: true },
        }),
        ...[11, 12, 13, 14, 0, 0, 0, 0].map((id) =>
          slot(id, { playing: false, position: 0, peak: 0, cues: Array(8).fill(0) }),
        ),
        slot(0),
      ],
      peak: 0.6,
      rms: 0.25,
      waveform: [],
      crossfader: 0.46,
      master: 0.75,
      streamError: false,
      callbacks: 1,
      clipped: 0,
      limited: 0,
      limiterDb: 0,
      masterDeck: 0,
    },
    output: {
      device: 'Studio Interface',
      backend: 'CoreAudio',
      sampleRate: 48000,
      channels: 4,
      bufferFrames: 256,
    },
    error: null,
    midi: {
      connected: null,
      bindings: [],
      learning: null,
      lastMessage: null,
      error: null,
      dropped: 0,
    },
  };
  window.__state = state;
  window.__calls = [];
  const emit = {};
  window.__emit = () =>
    emit.state?.({ event: 'engine:state', id: 1, payload: JSON.parse(JSON.stringify(state)) });
  const noise = (seed) => {
    let x = (seed * 2654435761) >>> 0;
    return () => (x = (x * 1664525 + 1013904223) >>> 0) / 4294967296;
  };
  /** Sections and a four-on-the-floor shape, so it reads like a track. */
  const waveform = (id) => {
    const rate = 50;
    const track = tracks[id - 1];
    const count = (track?.duration ?? 240) * rate;
    const bytes = new Uint8Array(4 + count * 4);
    bytes[0] = 1;
    bytes[1] = rate;
    const random = noise(id);
    const beat = 60 / (track?.music.bpm ?? 124);
    for (let i = 0; i < count; i++) {
      const t = i / rate;
      const phase = (t % beat) / beat;
      const section = Math.floor(t / (beat * 64));
      const energy = [0.45, 0.7, 1, 0.35, 0.9, 1, 0.6][(section + id) % 7];
      const kick = Math.exp(-phase * 6) * (energy > 0.4 ? 1 : 0.25);
      const hat = Math.exp(-((phase * 2) % 1) * 9) * energy;
      const low = 230 * kick * (0.8 + 0.2 * random());
      const mid = 150 * energy * (0.45 + 0.55 * random());
      const high = 170 * hat * (0.5 + 0.5 * random());
      const at = 4 + i * 4;
      bytes[at] = Math.min(255, Math.max(low, mid, high) + 18);
      bytes[at + 1] = low;
      bytes[at + 2] = mid;
      bytes[at + 3] = high;
    }
    return bytes.buffer;
  };
  /** Invented album art: flat shapes, different for each id. A third have none, as in real life. */
  const cover = async (id) => {
    if (id % 3 === 0) return new ArrayBuffer(0);
    const canvas = new OffscreenCanvas(320, 320);
    const c = canvas.getContext('2d');
    const random = noise(id * 7 + 3);
    const hue = random() * 360;
    const gradient = c.createLinearGradient(0, 0, 320, 320);
    gradient.addColorStop(0, `hsl(${hue} 55% 42%)`);
    gradient.addColorStop(1, `hsl(${(hue + 50) % 360} 60% 14%)`);
    c.fillStyle = gradient;
    c.fillRect(0, 0, 320, 320);
    for (let i = 0; i < 4; i++) {
      c.globalAlpha = 0.25 + random() * 0.5;
      c.fillStyle = `hsl(${(hue + 150 * i) % 360} 70% ${40 + random() * 40}%)`;
      c.beginPath();
      c.arc(random() * 320, random() * 320, 30 + random() * 110, 0, 7);
      c.fill();
    }
    const blob = await canvas.convertToBlob({ type: 'image/png' });
    return blob.arrayBuffer();
  };
  window.isTauri = true;
  window.__TAURI_INTERNALS__ = {
    transformCallback(f) {
      const id = n++;
      cbs[id] = f;
      return id;
    },
    unregisterCallback() {},
    convertFileSrc: (s) => s,
    metadata: {
      currentWindow: { label: 'main' },
      currentWebview: { label: 'main', windowLabel: 'main' },
    },
    invoke: async (cmd, args) => {
      if (!['track_cover', 'track_waveform', 'device_cover'].includes(cmd))
        window.__calls.push([cmd, args]);
      if (cmd === 'get_library') return library;
      if (cmd === 'status') return state;
      if (cmd === 'get_theme') return THEME;
      if (cmd === 'plugin:event|listen') {
        if (args.event === 'engine:state') emit.state = cbs[args.handler];
        return 1;
      }
      if (cmd === 'device_library')
        return {
          tracks: tracks.slice(0, 14).map((t, i) => ({
            id: t.id,
            title: t.title,
            artist: t.artist,
            album: '',
            genre: ['House', 'Techno', 'Disco'][i % 3],
            key: `${(i % 12) + 1}A`,
            bpm: t.music.bpm,
            duration: t.duration,
            rating: i % 6,
          })),
          playlists: [
            { id: 1, parent: 0, sort: 0, folder: true, name: 'Sets', tracks: [] },
            { id: 2, parent: 1, sort: 0, folder: false, name: 'Peak time', tracks: [1, 2, 3] },
            { id: 3, parent: 1, sort: 1, folder: false, name: 'Closing', tracks: [4] },
          ],
        };
      if (cmd === 'track_cue_info') return { hot: [], memory: [] };
      if (cmd === 'track_waveform') return waveform(args.id);
      if (cmd === 'track_cover') return cover(args.id);
      if (cmd === 'device_cover') return cover(args.track);
      if (cmd === 'track_beats') return null;
      if (cmd === 'audio_action') {
        const a = args.action;
        const s = state.audio.slots[a.slot];
        if (s && a.kind === 'autoLoop') s.looping = !s.looping;
        if (s && a.kind === 'loopBeats') {
          s.loopBeats = a.value;
          s.looping = true;
        }
        if (s && a.kind in s && a.kind !== 'loopBeats') s[a.kind] = a.value;
        window.__emit();
        return null;
      }
      return null;
    },
  };
})();
