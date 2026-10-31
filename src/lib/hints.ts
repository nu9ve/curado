import type { Action, AppSnapshot, Audio, KeyBinding } from './types';
import { actionLabel, allowedInMode, bindingLabel } from './keyboard';
import { currentLanguage, tr } from './language';

export interface HintTarget {
  label: string;
  title?: string;
  actions?: string;
  key?: string;
  help?: string;
  disabled?: boolean;
  slider?: boolean;
}
export interface Hint {
  label: string;
  summary: string;
  detail: string;
  shortcuts: string[];
  tone: 'a' | 'b' | 'master';
  status?: string;
}

const descriptions: Record<string, [string, string]> = {
  toggle: [
    'Alterna reproducción y pausa de este deck.',
    'Cada deck conserva su propio transporte. Pausar uno no detiene al otro ni a los samples.',
  ],
  play: [
    'Inicia la reproducción de este deck.',
    'Si llegó al final, vuelve al inicio. El volumen final también depende del canal, crossfader y master.',
  ],
  pause: [
    'Pausa este deck conservando la posición.',
    'Los demás decks y samples continúan. Usa un hot cue para saltar y volver a reproducir.',
  ],
  cue: [
    'Salta al hot cue y reproduce este deck.',
    'Suena exactamente donde lo guardaste; si es un loop de rekordbox, también lo activa. Con SYNC y Q activos en el seguidor espera al siguiente beat del master y usa el beat más cercano para no perder la fase. Sin posición guardada empieza en el primer sonido de la pista, ajustado al grid con Q. Shift + clic guarda. Un salto fuera del loop lo desactiva.',
  ],
  setCue: [
    'Guarda la posición actual en este hot cue.',
    'Con Q activo se ajusta al beat más cercano. Se guarda por pista y se comparte si cargas la misma pista en ambos decks. Guardar no cambia la reproducción.',
  ],
  playBoth: [
    'Inicia A y B en el mismo bloque de audio.',
    'Conserva la posición de cada deck, salvo si llegó al final. No cambia el crossfader ni el master y no sincroniza BPM.',
  ],
  pauseBoth: [
    'Pausa A y B en el mismo bloque de audio.',
    'Conserva ambas posiciones. Los samples siguen independientes.',
  ],
  cueBoth: [
    'Salta al hot cue de A y B y reproduce ambos a la vez.',
    'Una sola acción del motor aplica los dos saltos en el mismo bloque. Cada pista usa su propio cue; sin cue guardado empieza en su primer sonido. No ajusta BPM ni restaura un master silenciado.',
  ],
  seek: [
    'Busca una posición dentro de esta pista.',
    'Conserva reproducción o pausa. Un salto fuera del loop lo desactiva; los otros decks siguen independientes.',
  ],
  nudge: [
    'Desplaza la posición de este deck unos milisegundos.',
    'Sirve para corregir alineación a oído, también con SYNC activo; se conserva hasta que algo vuelva a alinear el deck, como un cue del master o reactivar SYNC. No cambia permanentemente la velocidad.',
  ],
  loop: [
    'Activa o desactiva el loop de este deck.',
    'Con BPM conocido usa la longitud elegida en beats y Q alinea el comienzo. Sin beatgrid crea hasta cuatro segundos. Los límites se funden para evitar clicks sin recortar el ataque del inicio.',
  ],
  loopIn: [
    'Marca el comienzo del loop en la posición actual.',
    'Con Q activo se ajusta al beat más cercano. Desactiva el loop mientras defines su final con OUT. Solo afecta a este deck.',
  ],
  loopOut: [
    'Marca el final y activa el loop si está después de IN.',
    'Con Q activo se ajusta al beat más cercano. El tramo se repite en este deck y los límites se funden para evitar clicks.',
  ],
  trigger: [
    'Reproduce el sample desde el inicio.',
    'Cada pad tiene un transporte independiente. Los samples pasan al master y no dependen del crossfader A/B.',
  ],
  gain: [
    'Ajusta el volumen de este canal antes del crossfader.',
    'El medidor del canal está antes del crossfader y master: puede mostrar señal aunque el canal no llegue a la salida. Doble clic restaura el valor inicial.',
  ],
  rate: [
    'Cambia la velocidad de este deck.',
    'Con key lock se conserva la tonalidad y el cambio se aplica al instante. Sin key lock funciona como varispeed. Doble clic vuelve a velocidad original.',
  ],
  low: [
    'Ajusta los graves de este canal.',
    'Centro = ganancia unitaria. Arrastra verticalmente, usa rueda o flechas; Shift afina y doble clic restaura.',
  ],
  mid: [
    'Ajusta los medios de este canal.',
    'Centro = ganancia unitaria. Arrastra verticalmente, usa rueda o flechas; Shift afina y doble clic restaura.',
  ],
  high: [
    'Ajusta los agudos de este canal.',
    'Centro = ganancia unitaria. Arrastra verticalmente, usa rueda o flechas; Shift afina y doble clic restaura.',
  ],
  filter: [
    'Atenúa los agudos con un filtro paso bajo.',
    'A la derecha queda abierto (OFF). Un filtro muy cerrado puede hacer que una pista parezca casi silenciosa.',
  ],
  echo: [
    'Añade echo a este canal.',
    'El tiempo sigue el BPM de salida y la división elegida en beats; sin análisis usa 120 BPM. Doble clic elimina el efecto.',
  ],
  highPass: [
    'Atenúa los graves con un filtro paso alto.',
    'OFF conserva el paso completo. Es independiente del filtro paso bajo; ambos tienen suavizado para evitar saltos.',
  ],
  echoBeats: [
    'Elige la duración del echo en beats.',
    '0,5 equivale a medio beat; 0,75 es una división con puntillo. Los cambios de tiempo se suavizan y pueden producir un barrido de tono en la cola.',
  ],
  quantize: [
    'Activa o desactiva el ajuste al beatgrid.',
    'Está activado por defecto. Ajusta al beat más cercano los loops y los cues que guardas; un cue ya guardado no se mueve. Sin BPM conocido no se puede cuantizar. Corrige BPM y offset si la detección no coincide.',
  ],
  loopBeats: [
    'Crea un loop con una longitud en beats.',
    'Con Q activo comienza en el beat más cercano. El beatgrid pertenece a esta pista; sync conserva la relación de tempo con el master.',
  ],
  sync: [
    'Vincula tempo y fase de este deck al master.',
    'Elige el otro deck como master. El tempo sigue sus cambios dentro del rango ±50 %. Un cambio manual de velocidad de este deck desactiva sync.',
  ],
  masterDeck: [
    'Elige el deck que guía la sincronización.',
    'El otro deck puede seguirlo con SYNC. Cambiar de master no inicia reproducción ni ajusta por sí solo la tonalidad.',
  ],
  tempo: [
    'Ajusta el BPM de reproducción.',
    'No modifica el BPM original detectado. En un deck sincronizado, cambia el tempo desde su master; key lock conserva tonalidad al ajustar velocidad.',
  ],
  transpose: [
    'Cambia la tonalidad en semitonos sin cambiar el BPM.',
    'Se aplica al instante. No cambia modo mayor/menor. En notas muy graves el tono puede quedar algo alto.',
  ],
  transposeStep: [
    'Sube o baja un semitono sin cambiar el BPM.',
    'Se aplica al instante, conservando transporte y cues.',
  ],
  keyLock: [
    'Conserva la tonalidad al cambiar el tempo.',
    'Activado por defecto. Sin key lock la velocidad cambia también el tono; la UI muestra el desplazamiento estimado en cents.',
  ],
  keySync: [
    'Iguala tonalidad o relativa compatible con el master.',
    'Se elige la transposición más cercana. Si los modos mayor/menor difieren se usa la relativa; transponer no convierte un acorde menor en mayor. Es una acción puntual, no un seguimiento de cambios futuros.',
  ],
  crossfader: [
    'Distribuye la mezcla entre A y B.',
    'Izquierda = A; centro = ambos; derecha = B. Usa una curva equal-power. No inicia ni pausa los decks.',
  ],
  master: [
    'Ajusta el volumen total de salida.',
    'Afecta decks y samples. Si usaste silencio total, vuelve a subir el master y reanuda los decks que quieras escuchar.',
  ],
  panic: [
    'Detiene decks y samples y baja el master a cero.',
    'También cancela la cola. Para volver a escuchar, sube el master y reproduce de nuevo; un cue no restaura el volumen.',
  ],
};

const englishDescriptions: Record<string, [string, string]> = {
  toggle: [
    'Toggle playback for this deck.',
    'Each deck has its own transport; pausing one does not stop the other deck or samples.',
  ],
  play: [
    'Start this deck.',
    'At the end it starts again from the beginning. Output also depends on channel, crossfader and master levels.',
  ],
  pause: [
    'Pause this deck at its current position.',
    'Other decks and samples continue. Use a hot cue to jump and play.',
  ],
  cue: [
    'Jump to the hot cue and play.',
    'It plays exactly where you saved it; a rekordbox loop also starts looping. With SYNC and Q on the follower, it waits for the next master beat and uses the nearest beat to keep phase. An unsaved cue starts at the track’s first sound, on the grid with Q. Shift-click saves. A jump outside the loop disables it.',
  ],
  setCue: [
    'Save the current position to this hot cue.',
    'With Q on, it snaps to the nearest beat. Cues are saved per track and shared by both decks. Saving does not change playback.',
  ],
  playBoth: [
    'Start A and B in the same audio block.',
    'Positions are preserved unless a deck has ended. This does not change levels or sync BPM.',
  ],
  pauseBoth: [
    'Pause A and B in the same audio block.',
    'Both positions are preserved; samples remain independent.',
  ],
  cueBoth: [
    'Jump to the hot cue and play both decks together.',
    'Each track uses its own cue. This does not sync BPM or restore a muted master.',
  ],
  seek: [
    'Seek within this track.',
    'Playback or pause is preserved. A jump outside the loop disables it.',
  ],
  nudge: [
    'Move this deck a few milliseconds.',
    'Use it to adjust alignment by ear, including with SYNC on; it lasts until something realigns the deck, such as a master cue or re-enabling SYNC. It does not permanently change speed.',
  ],
  loop: [
    'Toggle this deck’s loop.',
    'With BPM it uses the selected beat length and Q aligns the start. Without a grid it creates up to four seconds. Boundaries are faded to avoid clicks without cutting the attack at the start.',
  ],
  loopIn: [
    'Mark the loop start at the current position.',
    'With Q on, it snaps to the nearest beat. The loop is disabled until you set its end with OUT.',
  ],
  loopOut: [
    'Mark the end and enable the loop if it follows IN.',
    'With Q on, it snaps to the nearest beat. The section repeats on this deck and boundaries are faded to avoid clicks.',
  ],
  trigger: [
    'Play the sample from the start.',
    'Each pad has independent transport and routes to master, outside the A/B crossfader.',
  ],
  gain: [
    'Adjust channel volume before the crossfader.',
    'The meter is before crossfader and master. Double-click to restore the default.',
  ],
  rate: [
    'Change this deck’s speed.',
    'Key lock keeps pitch and applies at once; without it, speed also changes pitch. Double-click to reset.',
  ],
  low: [
    'Adjust channel bass.',
    'Center is unity. Drag, use the wheel or arrows; Shift fine-tunes and double-click resets.',
  ],
  mid: [
    'Adjust channel mids.',
    'Center is unity. Drag, use the wheel or arrows; Shift fine-tunes and double-click resets.',
  ],
  high: [
    'Adjust channel treble.',
    'Center is unity. Drag, use the wheel or arrows; Shift fine-tunes and double-click resets.',
  ],
  filter: [
    'Reduce highs with a low-pass filter.',
    'Fully right is open (OFF). A closed filter can make a track nearly silent.',
  ],
  highPass: [
    'Reduce lows with a high-pass filter.',
    'OFF passes the full signal. It is independent of the low-pass filter.',
  ],
  echo: [
    'Add echo to this channel.',
    'Time follows output BPM and the chosen beat division; without analysis it uses 120 BPM. Double-click removes the effect.',
  ],
  echoBeats: [
    'Choose echo length in beats.',
    '0.5 is half a beat and 0.75 is dotted. Smoothed time changes can sweep the pitch of the tail.',
  ],
  quantize: [
    'Toggle beatgrid quantization.',
    'Enabled by default. It snaps loops and the cues you save to the nearest beat; a saved cue never moves. Correct BPM and offset if detection does not match.',
  ],
  loopBeats: [
    'Create a loop of the selected beat length.',
    'With Q on, it starts at the nearest beat. The beatgrid belongs to this track.',
  ],
  sync: [
    'Link this deck’s tempo and phase to the master.',
    'Choose the other deck as master. A manual speed change disables sync on this deck.',
  ],
  masterDeck: [
    'Choose the deck that guides synchronization.',
    'The other deck can follow it with SYNC. Changing master does not start playback.',
  ],
  tempo: [
    'Adjust playback BPM.',
    'The original detected BPM is unchanged. Key lock preserves pitch when speed changes.',
  ],
  transpose: [
    'Change key in semitones without changing BPM.',
    'It applies at once and does not change major or minor. Very low notes can sit slightly sharp.',
  ],
  transposeStep: [
    'Move one semitone without changing BPM.',
    'It applies at once; transport and cues are preserved.',
  ],
  keyLock: [
    'Preserve pitch when changing tempo.',
    'On by default. Without it, speed also changes pitch.',
  ],
  keySync: [
    'Match the master key or its compatible relative.',
    'This is a one-time adjustment; it does not follow later changes.',
  ],
  crossfader: [
    'Distribute the mix between A and B.',
    'Left is A, center is both, right is B. It does not start or pause decks.',
  ],
  master: [
    'Adjust the total output volume.',
    'Affects decks and samples. After panic, raise master and resume the desired decks.',
  ],
  panic: [
    'Stop decks and samples and turn master to zero.',
    'Also cancels the queue. Raise master and play again to resume; cues do not restore volume.',
  ],
};

/** Transport/routing facts only; a playing source may itself contain silence. */
export function deckStatus(audio: Audio | null, index: number): string {
  const slot = audio?.slots[index];
  if (!audio || !slot?.id) return tr('Sin pista', 'No track');
  if (audio.streamError) return tr('Error de salida', 'Output error');
  if (!slot.playing)
    return slot.duration > 0 && slot.position >= slot.duration
      ? tr('Fin de pista', 'End of track')
      : tr('En pausa', 'Paused');
  if (audio.master === 0) return tr('Master a cero', 'Master at zero');
  if (slot.gain === 0) return tr('Canal a cero', 'Channel at zero');
  if ((index === 0 && audio.crossfader === 1) || (index === 1 && audio.crossfader === 0))
    return tr('Fuera del crossfader', 'Outside crossfader');
  return tr('Reproduciendo', 'Playing');
}
const sections: Record<string, [string, string]> = {
  'web-explorer': [
    'Abre el panel de radio y servicios web.',
    'El navegador integrado tiene audio independiente. Salir de esta vista detiene el audio web y conserva los decks locales.',
  ],
  'web-fractos': [
    'Abre fractos radio en el navegador integrado.',
    'Usa la URL HTTPS confirmada. Sus iframes de YouTube y SoundCloud suenan fuera del mixer; controla el audio desde esa web o usa detener audio web.',
  ],
  'web-soundcloud': [
    'Abre SoundCloud en el navegador integrado.',
    'Su player web no entra a los decks. La sesión de navegación es temporal; enlaces externos, popups y descargas están restringidos.',
  ],
  'web-close': [
    'Cierra el navegador integrado y detiene su reproducción web.',
    'Afecta solo a fractos/SoundCloud; no pausa los decks locales. El botón de silencio de curado controla únicamente el motor nativo.',
  ],
  player: [
    'Escucha con el deck A y la cola.',
    'Al cambiar a reproductor se pausan B y los samples y el crossfader va a A.',
  ],
  mix: [
    'Mezcla con dos decks y cuatro samples.',
    'Al entrar se centra el crossfader. Cada deck se reproduce por separado; los cues no sincronizan tempo ni beats.',
  ],
  settings: [
    'Configura audio, teclado, MIDI y apariencia.',
    'Cambiar dispositivo de audio descarga los slots. Escape cierra Configuración.',
  ],
  mini: [
    'Abre la vista mini siempre encima.',
    'Conserva el transporte y permite volver a la vista completa.',
  ],
  devices: [
    'Discos y USB conectados.',
    'Un USB de rekordbox muestra sus playlists, grids y cues; en cualquier disco puedes explorar sus carpetas con audio. curado no escribe nada en el dispositivo: lo que cambies aquí queda en curado, y “A rekordbox” en la biblioteca lo guarda en un XML que rekordbox puede importar. Lo cargado sigue sonando aunque lo desconectes.',
  ],
  limiter: [
    'Indica cuánto está bajando el limitador la mezcla.',
    'Evita el recorte a −1 dBTP. Una reducción pequeña en los golpes es normal; en rojo aplana la mezcla: baja canales, EQ o master.',
  ],
};

const englishSections: Record<string, [string, string]> = {
  'web-explorer': [
    'Open radio and web services.',
    'The integrated browser has independent audio. Leaving this view stops web audio and preserves local decks.',
  ],
  'web-fractos': [
    'Open fractos radio.',
    'Its embedded players play outside the mixer; control them on the website or stop web audio.',
  ],
  'web-soundcloud': [
    'Open SoundCloud.',
    'Web playback does not enter the decks. The session is temporary; external links, popups and downloads are restricted.',
  ],
  'web-close': [
    'Close the browser and stop web playback.',
    'This does not pause local decks. curado’s mute controls the native engine only.',
  ],
  player: [
    'Listen with deck A and the queue.',
    'Switching to player pauses B and samples and moves the crossfader to A.',
  ],
  mix: [
    'Mix with two decks and four samples.',
    'Entering centers the crossfader. Cues do not synchronize tempo or beats.',
  ],
  settings: [
    'Configure audio, keyboard, MIDI and appearance.',
    'Changing audio devices unloads slots. Escape closes settings.',
  ],
  mini: [
    'Open the always-on-top mini view.',
    'Transport is preserved and you can return to the full view.',
  ],
  devices: [
    'Connected drives and USB sticks.',
    'A rekordbox USB shows its playlists, grids and cues; on any drive you can browse folders with audio. curado writes nothing to the device: what you change here stays in curado, and “To rekordbox” in the library saves it to an XML rekordbox can import. What is loaded keeps playing if you unplug it.',
  ],
  limiter: [
    'Shows how much the limiter is turning the mix down.',
    'It prevents clipping at −1 dBTP. A little reduction on hits is normal; red flattens the mix, so lower channels, EQ or master.',
  ],
};

export function parseHintAction(token = ''): Action | undefined {
  const parts = token.split(':');
  if (parts.length !== 3) return;
  const slot = Number(parts[0]);
  const kind = parts[1];
  const value = Number(parts[2]);
  if (!Number.isInteger(slot) || slot < 0 || slot > 5 || !kind || !Number.isFinite(value)) return;
  return { slot, kind, value };
}

/** Pure text resolution: the hovered control and saved mappings are the only authority. */
export function describeHint(target: HintTarget, keys: KeyBinding[], snapshot: AppSnapshot): Hint {
  const tokens =
    target.actions
      ?.split(/\s+/)
      .map(parseHintAction)
      .filter((a): a is Action => !!a) ?? [];
  const keyBindings = target.key ? keys.filter((k) => k.code === target.key) : [];
  const actions = tokens.length ? tokens : keyBindings.map((k) => k.action);
  const action = actions[0];
  const global =
    action &&
    ['master', 'panic', 'crossfader', 'playBoth', 'pauseBoth', 'cueBoth'].includes(action.kind);
  const tone = action && !global && action.slot < 2 ? (action.slot ? 'b' : 'a') : 'master';
  const scope =
    action && !global
      ? `${action.slot < 2 ? `Deck ${action.slot ? 'B' : 'A'}` : `Sample ${action.slot - 1}`} · `
      : '';
  const numbered =
    action && ['cue', 'setCue', 'cueBoth'].includes(action.kind) ? ` ${action.value + 1}` : '';
  const direction =
    action?.kind === 'nudge'
      ? ` ${action.value < 0 ? '−' : '+'}${Math.round(Math.abs(action.value) * 1000)} ms`
      : action?.kind === 'crossfader' && !target.slider
        ? ` · ${action.value === 0 ? 'A' : action.value === 1 ? 'B' : action.value === 0.5 ? 'centro' : `${Math.round(action.value * 100)}%`}`
        : '';
  const label = action
    ? `${scope}${actionLabel(action.kind)}${numbered}${direction}`
    : target.label || 'Ayuda contextual';
  const text =
    (action &&
      (currentLanguage() === 'en'
        ? englishDescriptions[action.kind]
        : descriptions[action.kind])) ||
    (target.help &&
      (currentLanguage() === 'en' ? englishSections[target.help] : sections[target.help]));
  const shortcuts = keys
    .filter((k) =>
      actions.some(
        (a) =>
          a.slot === k.action.slot &&
          a.kind === k.action.kind &&
          (target.slider || a.value === k.action.value),
      ),
    )
    .map(bindingLabel);
  let status = target.disabled
    ? tr('Control no disponible en este momento.', 'Control currently unavailable.')
    : undefined;
  if (action && !allowedInMode(action, snapshot.mode))
    status = tr('Disponible en modo Mezcla.', 'Available in Mix mode.');
  else if (!target.disabled && action && snapshot.audio) {
    const slot = snapshot.audio.slots[action.slot];
    if (slot && !global) {
      status = deckStatus(snapshot.audio, action.slot);
    }
  }
  return {
    label,
    summary: text
      ? text[0]
      : target.title || tr('Control de la interfaz de curado.', 'curado interface control.'),
    detail: text
      ? text[1]
      : tr(
          'Esta es la ayuda básica de este control. Los textos y consejos se ampliarán aquí.',
          'Basic help for this control.',
        ),
    shortcuts: [...new Set(shortcuts)],
    tone,
    status,
  };
}

/** Read a single nearest control, including icons inside buttons and disabled controls. */
export function readHintTarget(target: EventTarget | null): HintTarget | null {
  if (!(target instanceof Element) || target.closest('[data-context-help]')) return null;
  const el = target.closest<HTMLElement>(
    '[data-help], [data-action], button, input, select, textarea, a, [role="slider"], [role="meter"], [role="img"], [title]',
  );
  if (!el) return null;
  const label =
    el.getAttribute('aria-label') ||
    ('labels' in el ? (el as HTMLInputElement).labels?.[0]?.textContent : '') ||
    el.textContent ||
    el.getAttribute('title') ||
    '';
  return {
    label: label.replace(/\s+/g, ' ').trim().slice(0, 180),
    title: el.getAttribute('title') || undefined,
    actions: el.dataset.action,
    key: el.dataset.key,
    help: el.dataset.help,
    disabled: el.matches(':disabled, [aria-disabled="true"]'),
    slider: el.getAttribute('role') === 'slider',
  };
}
