import type { Action, AppSnapshot, Audio, KeyBinding } from './types';
import { actionLabel, allowedInMode, bindingLabel } from './keyboard';

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
    'Sin posición guardada, comienza en 0:00. Shift + clic en un pad guarda el cue; los atajos de guardar aparecen aquí si están asignados. Un salto fuera del loop lo desactiva.',
  ],
  setCue: [
    'Guarda la posición actual en este hot cue.',
    'Se guarda por pista y se comparte si cargas la misma pista en ambos decks. Guardar no cambia la reproducción.',
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
    'Una sola acción del motor aplica los dos saltos en el mismo bloque. Cada pista usa su propio cue; sin cue guardado empieza en 0:00. No ajusta BPM ni restaura un master silenciado.',
  ],
  seek: [
    'Busca una posición dentro de esta pista.',
    'Conserva reproducción o pausa. Un salto fuera del loop lo desactiva; los otros decks siguen independientes.',
  ],
  nudge: [
    'Desplaza la posición de este deck unos milisegundos.',
    'Sirve para corregir alineación a oído. No cambia permanentemente la velocidad ni sincroniza beats.',
  ],
  loop: [
    'Activa o desactiva el loop de este deck.',
    'Sin puntos válidos crea hasta cuatro segundos desde la posición actual. Los loops todavía no se ajustan a beats.',
  ],
  loopIn: [
    'Marca el comienzo del loop en la posición actual.',
    'Desactiva el loop mientras defines su final con OUT. Solo afecta a este deck.',
  ],
  loopOut: [
    'Marca el final y activa el loop si está después de IN.',
    'El tramo se repite en este deck. Los límites todavía no tienen crossfade.',
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
    'Cambia la velocidad y el tono de este deck.',
    'No hay key lock: aumentar velocidad también sube el tono. Doble clic vuelve a velocidad original.',
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
    'El retardo es fijo, de 375 ms; no está sincronizado al BPM. Doble clic elimina el efecto.',
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

/** Transport/routing facts only; a playing source may itself contain silence. */
export function deckStatus(audio: Audio | null, index: number): string {
  const slot = audio?.slots[index];
  if (!audio || !slot?.id) return 'Sin pista';
  if (audio.streamError) return 'Error de salida';
  if (!slot.playing)
    return slot.duration > 0 && slot.position >= slot.duration ? 'Fin de pista' : 'En pausa';
  if (audio.master === 0) return 'Master a cero';
  if (slot.gain === 0) return 'Canal a cero';
  if ((index === 0 && audio.crossfader === 1) || (index === 1 && audio.crossfader === 0))
    return 'Fuera del crossfader';
  return 'Reproduciendo';
}
const sections: Record<string, [string, string]> = {
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
  const text = (action && descriptions[action.kind]) || (target.help && sections[target.help]);
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
  let status = target.disabled ? 'Control no disponible en este momento.' : undefined;
  if (action && !allowedInMode(action, snapshot.mode)) status = 'Disponible en modo Mezcla.';
  else if (!target.disabled && action && snapshot.audio) {
    const slot = snapshot.audio.slots[action.slot];
    if (slot && !global) {
      status = deckStatus(snapshot.audio, action.slot);
    }
  }
  return {
    label,
    summary: text ? text[0] : target.title || 'Control de la interfaz de curado.',
    detail: text
      ? text[1]
      : 'Esta es la ayuda básica de este control. Los textos y consejos se ampliarán aquí.',
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
