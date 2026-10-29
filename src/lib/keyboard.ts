import type { KeyBinding } from './types';
import { currentLanguage } from './language';
const key = (code: string, slot: number, kind: string, value = 0, shift = false): KeyBinding => ({
  code,
  shift,
  action: { slot, kind, value },
});
export const DEFAULT_KEYS: KeyBinding[] = [
  key('Space', 0, 'toggle'),
  key('KeyQ', 0, 'toggle'),
  key('KeyA', 0, 'cue', 0),
  key('KeyW', 0, 'loop'),
  key('KeyE', 0, 'loopIn'),
  key('KeyR', 0, 'loopOut'),
  key('Digit1', 0, 'cue', 0),
  key('Digit2', 0, 'cue', 1),
  key('Digit3', 0, 'cue', 2),
  key('Digit1', 0, 'setCue', 0, true),
  key('Digit2', 0, 'setCue', 1, true),
  key('Digit3', 0, 'setCue', 2, true),
  key('KeyP', 1, 'toggle'),
  key('KeyL', 1, 'cue', 0),
  key('KeyO', 1, 'loop'),
  key('KeyI', 1, 'loopIn'),
  key('KeyU', 1, 'loopOut'),
  key('Digit8', 1, 'cue', 0),
  key('Digit9', 1, 'cue', 1),
  key('Digit0', 1, 'cue', 2),
  key('Digit8', 1, 'setCue', 0, true),
  key('Digit9', 1, 'setCue', 1, true),
  key('Digit0', 1, 'setCue', 2, true),
  key('KeyD', 0, 'nudge', -0.05),
  key('KeyF', 0, 'nudge', 0.05),
  key('KeyJ', 1, 'nudge', -0.05),
  key('KeyK', 1, 'nudge', 0.05),
  key('KeyZ', 2, 'trigger'),
  key('KeyX', 3, 'trigger'),
  key('KeyC', 4, 'trigger'),
  key('KeyV', 5, 'trigger'),
  key('ArrowLeft', 0, 'crossfader', 0),
  key('ArrowDown', 0, 'crossfader', 0.5),
  key('ArrowRight', 0, 'crossfader', 1),
  key('Escape', 0, 'panic'),
];
export function keyLabel(code: string) {
  return (
    (
      { Space: '␣', ArrowLeft: '←', ArrowRight: '→', ArrowDown: '↓', Escape: 'ESC' } as Record<
        string,
        string
      >
    )[code] ?? code.replace('Key', '').replace('Digit', '')
  );
}
export function bindingLabel(binding: KeyBinding) {
  return `${binding.shift ? '⇧ ' : ''}${keyLabel(binding.code)}`;
}
export function actionLabel(kind: string) {
  if (currentLanguage() === 'en') {
    const english: Record<string, string> = {
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
  return mode === 'mix' || (action.slot === 0 && action.kind !== 'crossfader');
}
export function matchKey(
  event: Pick<KeyboardEvent, 'code' | 'shiftKey' | 'metaKey' | 'ctrlKey' | 'altKey' | 'repeat'>,
  bindings: KeyBinding[],
  mode: 'player' | 'mix',
) {
  if (event.metaKey || event.ctrlKey || event.altKey || event.repeat) return undefined;
  const binding = bindings.find((b) => b.code === event.code && b.shift === event.shiftKey);
  return binding && allowedInMode(binding.action, mode) ? binding : undefined;
}
