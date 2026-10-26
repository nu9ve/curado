import { memo } from 'react';
import type { KeyBinding } from '../lib/types';
import { actionLabel, allowedInMode, keyLabel } from '../lib/keyboard';
import { useHud } from '../lib/feedback';
import { actionKey } from '../lib/feedback';

/** Physical ANSI/ISO rows; codes are layout independent (`KeyboardEvent.code`). */
const ROWS: string[][] = [
  [
    'Escape',
    'Digit1',
    'Digit2',
    'Digit3',
    'Digit4',
    'Digit5',
    'Digit6',
    'Digit7',
    'Digit8',
    'Digit9',
    'Digit0',
  ],
  ['KeyQ', 'KeyW', 'KeyE', 'KeyR', 'KeyT', 'KeyY', 'KeyU', 'KeyI', 'KeyO', 'KeyP'],
  ['KeyA', 'KeyS', 'KeyD', 'KeyF', 'KeyG', 'KeyH', 'KeyJ', 'KeyK', 'KeyL'],
  [
    'KeyZ',
    'KeyX',
    'KeyC',
    'KeyV',
    'KeyB',
    'KeyN',
    'KeyM',
    'Space',
    'ArrowLeft',
    'ArrowDown',
    'ArrowRight',
  ],
];

function zone(bindings: KeyBinding[]) {
  const plain = bindings.find((b) => !b.shift) ?? bindings[0];
  if (!plain) return 'free';
  const { slot, kind } = plain.action;
  if (kind === 'crossfader' || kind === 'panic' || kind === 'master') return 'mixer';
  if (slot >= 2) return 'sample';
  return slot ? 'b' : 'a';
}

/**
 * The left hand plays deck A, the right hand deck B, the bottom row fires samples.
 * Keycaps light up through `data-key` when pressed (see `feedback.ts`) and can be clicked.
 */
export const KeyboardMap = memo(function KeyboardMap({
  keys,
  mode,
  onTrigger,
}: {
  keys: KeyBinding[];
  mode: 'player' | 'mix';
  onTrigger: (binding: KeyBinding) => void;
}) {
  return (
    <section className="keyboard-map" aria-label="Mapa del teclado">
      <div className="keyboard-legend">
        <span className="eyebrow">TECLADO</span>
        <span className="z-a">A · izquierda</span>
        <span className="z-b">B · derecha</span>
        <span className="z-sample">samples</span>
        <span className="z-mixer">mixer</span>
      </div>
      {ROWS.map((row, r) => (
        <div className="key-row" key={r} style={{ paddingLeft: `${r * 0.9}em` }}>
          {row.map((code) => {
            const bound = keys.filter((k) => k.code === code);
            const z = zone(bound);
            const blocked = bound.every((b) => !allowedInMode(b.action, mode));
            const plain = bound.find((b) => !b.shift);
            const title = bound
              .map(
                (b) =>
                  `${b.shift ? '⇧ ' : ''}${b.action.slot < 2 ? (b.action.slot ? 'B' : 'A') : `S${b.action.slot - 1}`} · ${actionLabel(b.action.kind)}`,
              )
              .join('\n');
            return (
              <button
                key={code}
                className={`keycap z-${z} ${code === 'Space' ? 'wide' : ''} ${blocked && bound.length ? 'blocked' : ''}`}
                data-key={code}
                data-action={bound.map((b) => actionKey(b.action)).join(' ') || undefined}
                title={title || 'Libre'}
                disabled={!plain || (blocked && bound.length > 0)}
                onClick={() => plain && onTrigger(plain)}
                tabIndex={-1}
              >
                {keyLabel(code)}
              </button>
            );
          })}
        </div>
      ))}
    </section>
  );
});

/** On-screen display of the last shortcuts: what was just triggered, in big pixel type. */
export function KeyHud() {
  const entries = useHud();
  return (
    <div className="key-hud" aria-live="polite">
      {entries.map((e) => (
        <div key={e.id} className={`hud-entry slot-${Math.min(e.slot, 2)}`}>
          <kbd>{e.key}</kbd>
          <span>{e.label}</span>
        </div>
      ))}
    </div>
  );
}
