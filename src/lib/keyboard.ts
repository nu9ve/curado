import type { Audio, KeyBinding } from './types';
import { currentLanguage } from './language';
type Modifier = '' | 'shift' | 'ctrl' | 'alt';
const key = (
  code: string,
  slot: number,
  kind: string,
  value = 0,
  mod: Modifier = '',
): KeyBinding => ({
  code,
  shift: mod === 'shift',
  ...(mod === 'ctrl' ? { ctrl: true } : {}),
  ...(mod === 'alt' ? { alt: true } : {}),
  action: { slot, kind, value },
});
/** What each hand reaches, mirrored: the same finger does the same job on either deck. */
const HANDS = [
  {
    slot: 0,
    pads: ['Digit1', 'Digit2', 'Digit3', 'Digit4', 'KeyQ', 'KeyW', 'KeyE', 'KeyR'],
    play: 'KeyF',
    cue: 'KeyD',
    sync: 'KeyG',
    back: 'KeyA',
    forward: 'KeyS',
    loopIn: 'KeyZ',
    loopOut: 'KeyX',
    reloop: 'KeyC',
    slip: 'KeyV',
    fx: 'KeyT',
  },
  {
    slot: 1,
    pads: ['Digit7', 'Digit8', 'Digit9', 'Digit0', 'KeyU', 'KeyI', 'KeyO', 'KeyP'],
    play: 'KeyJ',
    cue: 'KeyK',
    sync: 'KeyH',
    back: 'KeyL',
    forward: 'Semicolon',
    loopIn: 'Slash',
    loopOut: 'Period',
    reloop: 'Comma',
    slip: 'KeyM',
    fx: 'KeyY',
  },
];
export const DEFAULT_KEYS: KeyBinding[] = [
  // Everyone's first reach: the bar stops or starts the whole mix.
  key('Space', 0, 'toggleBoth'),
  key('Escape', 0, 'panic'),
  // The list: up and down to choose, Shift and a side to load into that deck.
  key('ArrowUp', 0, 'listMove', -1),
  key('ArrowDown', 0, 'listMove', 1),
  key('ArrowUp', 0, 'listMove', -8, 'shift'),
  key('ArrowDown', 0, 'listMove', 8, 'shift'),
  key('ArrowLeft', 0, 'listLoad', 0, 'shift'),
  key('ArrowRight', 1, 'listLoad', 0, 'shift'),
  key('ArrowLeft', 0, 'crossfader', 0, 'ctrl'),
  key('ArrowDown', 0, 'crossfader', 0.5, 'ctrl'),
  key('ArrowRight', 0, 'crossfader', 1, 'ctrl'),
  key('KeyB', 2, 'beatFxOn', 1, 'alt'),
  ...HANDS.flatMap((hand) => [
    // Plain plays a cue, Shift saves it, Control deletes it.
    ...hand.pads.flatMap((code, cue) => [
      key(code, hand.slot, 'cue', cue),
      key(code, hand.slot, 'setCue', cue, 'shift'),
      key(code, hand.slot, 'clearCue', cue, 'ctrl'),
    ]),
    key(hand.play, hand.slot, 'toggle'),
    key(hand.cue, hand.slot, 'cue', 0),
    key(hand.sync, hand.slot, 'sync', 1),
    key(hand.sync, hand.slot, 'masterDeck', 0, 'shift'),
    key(hand.sync, hand.slot, 'keyLock', 1, 'ctrl'),
    key(hand.back, hand.slot, 'nudge', -0.05),
    key(hand.forward, hand.slot, 'nudge', 0.05),
    key(hand.back, hand.slot, 'beatJump', -1, 'shift'),
    key(hand.forward, hand.slot, 'beatJump', 1, 'shift'),
    key(hand.back, hand.slot, 'beatJump', -4, 'ctrl'),
    key(hand.forward, hand.slot, 'beatJump', 4, 'ctrl'),
    key(hand.loopIn, hand.slot, 'loopIn'),
    key(hand.loopOut, hand.slot, 'loopOut'),
    key(hand.reloop, hand.slot, 'loop'),
    key(hand.loopIn, hand.slot, 'loopScale', 0.5, 'shift'),
    key(hand.loopOut, hand.slot, 'loopScale', 2, 'shift'),
    key(hand.reloop, hand.slot, 'quantize', 1, 'shift'),
    key(hand.slip, hand.slot, 'slip', 1),
    key(hand.slip, hand.slot, 'reverse', 1, 'shift'),
    key(hand.fx, hand.slot, 'beatFxOn', 1),
  ]),
  // Option with a pad's key fires the sample at that place.
  ...['Digit1', 'Digit2', 'Digit3', 'Digit4', 'Digit7', 'Digit8', 'Digit9', 'Digit0'].map(
    (code, i) => key(code, i + 2, 'trigger', 0, 'alt'),
  ),
];
/** A key can only say "pressed": these flip whatever the engine reports now. */
export const TOGGLE_KINDS = new Set(['slip', 'reverse', 'beatFxOn', 'keyLock', 'sync', 'quantize']);
/** Handled by the interface: they move through a list, which the engine knows nothing about. */
export const LOCAL_KINDS = new Set(['listMove', 'listLoad']);
export function keyLabel(code: string) {
  return (
    (
      {
        Space: '␣',
        ArrowLeft: '←',
        ArrowRight: '→',
        ArrowDown: '↓',
        ArrowUp: '↑',
        Escape: 'ESC',
        Semicolon: ';',
        Comma: ',',
        Period: '.',
        Slash: '/',
      } as Record<string, string>
    )[code] ?? code.replace('Key', '').replace('Digit', '')
  );
}
export function bindingLabel(binding: KeyBinding) {
  const modifier = `${binding.ctrl ? '⌃' : ''}${binding.alt ? '⌥' : ''}${binding.shift ? '⇧' : ''}`;
  return `${modifier ? `${modifier} ` : ''}${keyLabel(binding.code)}`;
}
export function actionLabel(kind: string) {
  if (currentLanguage() === 'en') {
    const english: Record<string, string> = {
      toggleBoth: 'Play / pause all',
      listMove: 'Move in the list',
      listLoad: 'Load the selection',
      releaseFx: 'Release FX',
      channelCue: 'Headphone cue',
      masterCue: 'Master in headphones',
      clearCue: 'Delete cue',
      crossfaderCurve: 'Crossfader curve',
      trim: 'Trim',
      color: 'Color FX knob',
      colorFx: 'Color FX on this channel',
      colorParam: 'Color FX parameter',
      beatFx: 'Beat FX',
      beatFxOn: 'Beat FX on/off',
      beatFxBeats: 'Beat FX division',
      beatFxDepth: 'Beat FX level/depth',
      beatFxBpm: 'Beat FX tempo',
      slip: 'Slip',
      reverse: 'Reverse',
      roll: 'Roll',
      rollEnd: 'Roll release',
      beatJump: 'Beat jump',
      loopScale: 'Loop length',
      jogTouch: 'Platter touch',
      jogMove: 'Platter',
      jogBend: 'Tempo bend',
      brake: 'Vinyl brake',
      sampler: 'Sampler level',
      autoGain: 'Auto gain',
      nudge: 'Nudge (seconds)',
      toggle: 'Play / pause',
      cue: 'Hot cue',
      setCue: 'Save cue',
      playBoth: 'Play A + B',
      pauseBoth: 'Pause A + B',
      cueBoth: 'Hot cue A + B',
      loop: 'Loop on/off',
      loopIn: 'Loop IN',
      loopOut: 'Loop OUT',
      trigger: 'Trigger sample',
      crossfader: 'Crossfader',
      panic: 'Mute all',
      gain: 'Volume',
      rate: 'Speed',
      low: 'Low',
      mid: 'Mid',
      high: 'High',
      filter: 'LP filter',
      echo: 'Echo',
      highPass: 'HP filter',
      echoBeats: 'Echo in beats',
      quantize: 'Quantize',
      loopBeats: 'Beat loop',
      sync: 'Tempo sync',
      masterDeck: 'Master deck',
      tempo: 'Tempo',
      transpose: 'Transpose',
      transposeStep: 'Transpose one semitone',
      keyLock: 'Key lock',
      keySync: 'Key sync',
      master: 'Master',
    };
    return english[kind] ?? kind;
  }
  return (
    (
      {
        toggleBoth: 'Play / pausa general',
        listMove: 'Moverse en la lista',
        listLoad: 'Cargar la selección',
        releaseFx: 'Release FX',
        channelCue: 'Cue de audífonos',
        masterCue: 'Master en audífonos',
        clearCue: 'Borrar cue',
        crossfaderCurve: 'Curva del crossfader',
        trim: 'Trim',
        color: 'Perilla Color FX',
        colorFx: 'Color FX de este canal',
        colorParam: 'Parámetro Color FX',
        beatFx: 'Beat FX',
        beatFxOn: 'Beat FX on/off',
        beatFxBeats: 'División Beat FX',
        beatFxDepth: 'Nivel/profundidad Beat FX',
        beatFxBpm: 'Tempo Beat FX',
        slip: 'Slip',
        reverse: 'Reversa',
        roll: 'Roll',
        rollEnd: 'Soltar roll',
        beatJump: 'Salto en beats',
        loopScale: 'Largo del loop',
        jogTouch: 'Tocar plato',
        jogMove: 'Plato',
        jogBend: 'Doblar tempo',
        brake: 'Freno de vinilo',
        sampler: 'Nivel del sampler',
        autoGain: 'Ganancia automática',
        nudge: 'Nudge (segundos)',
        toggle: 'Play / pausa',
        cue: 'Hot cue',
        setCue: 'Guardar cue',
        playBoth: 'Reproducir A + B',
        pauseBoth: 'Pausar A + B',
        cueBoth: 'Hot cue A + B',
        loop: 'Loop on/off',
        loopIn: 'Loop IN',
        loopOut: 'Loop OUT',
        trigger: 'Disparar sample',
        crossfader: 'Crossfader',
        panic: 'Silenciar todo',
        gain: 'Volumen',
        rate: 'Velocidad',
        low: 'Graves',
        mid: 'Medios',
        high: 'Agudos',
        filter: 'Filtro LP',
        echo: 'Echo',
        highPass: 'Filtro HP',
        echoBeats: 'Echo en beats',
        quantize: 'Cuantizado',
        loopBeats: 'Loop en beats',
        sync: 'Sync de tempo',
        masterDeck: 'Deck master',
        tempo: 'Tempo',
        transpose: 'Transposición',
        transposeStep: 'Transponer un semitono',
        keyLock: 'Key lock',
        keySync: 'Key sync',
        master: 'Master',
      } as Record<string, string>
    )[kind] ?? kind
  );
}
/** Player mode keeps only deck A transport (mirrors `session.rs`, slightly stricter). */
export function allowedInMode(action: KeyBinding['action'], mode: 'player' | 'mix') {
  if (['playBoth', 'pauseBoth', 'cueBoth'].includes(action.kind)) return mode === 'mix';
  // The bar means play and pause in both modes: in the player that is its one deck.
  if (action.kind === 'toggleBoth') return true;
  return mode === 'mix' || (action.slot === 0 && action.kind !== 'crossfader');
}
/** What a key really sends, from what the engine reports at this moment. */
export function resolveAction(
  action: KeyBinding['action'],
  audio: Audio | null | undefined,
  mode: 'player' | 'mix',
): KeyBinding['action'] {
  if (action.kind === 'toggleBoth' && mode === 'player')
    return { slot: 0, kind: 'toggle', value: 0 };
  if (!TOGGLE_KINDS.has(action.kind)) return action;
  const slot = audio?.slots[action.slot];
  const on = {
    slip: slot?.slip,
    reverse: slot?.reverse,
    beatFxOn: audio?.beatFx?.[action.slot]?.on,
    keyLock: slot?.tuning.keyLock,
    sync: slot?.sync,
    quantize: slot?.quantize,
  }[action.kind];
  return { ...action, value: on ? 0 : 1 };
}
export function matchKey(
  event: Pick<KeyboardEvent, 'code' | 'shiftKey' | 'metaKey' | 'ctrlKey' | 'altKey' | 'repeat'>,
  bindings: KeyBinding[],
  mode: 'player' | 'mix',
) {
  // Command stays the system's and the app's: quit, copy, close.
  if (event.metaKey) return undefined;
  const binding = bindings.find(
    (b) =>
      b.code === event.code &&
      b.shift === event.shiftKey &&
      !!b.ctrl === event.ctrlKey &&
      !!b.alt === event.altKey,
  );
  // Holding an arrow runs down a list; holding anything else must not stutter the music.
  if (event.repeat && binding?.action.kind !== 'listMove') return undefined;
  return binding && allowedInMode(binding.action, mode) ? binding : undefined;
}
