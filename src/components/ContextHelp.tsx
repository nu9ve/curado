import { tr } from '../lib/language';
import { useEffect, useRef, useState } from 'react';
import { ChevronDown, ChevronUp, HelpCircle } from 'lucide-react';
import { useSnapshot } from '../lib/bridge';
import { describeHint, readHintTarget } from '../lib/hints';
import type { Hint, HintTarget } from '../lib/hints';
import type { KeyBinding } from '../lib/types';

/** One delegated listener per input type; no listeners or renders per pointer move. */
export function ContextHelp({
  keys,
  expanded,
  onExpand,
}: {
  keys: KeyBinding[];
  expanded: boolean;
  onExpand: (expanded: boolean) => void;
}) {
  const [target, setTarget] = useState<HintTarget | null>(null);
  const snapshot = useSnapshot();
  useEffect(() => {
    const update = (event: Event) => {
      const next = readHintTarget(event.target);
      // Keep the last control while entering the help itself, so it can be expanded/read.
      if (next) setTarget((prev) => (JSON.stringify(prev) === JSON.stringify(next) ? prev : next));
    };
    document.addEventListener('pointerover', update);
    document.addEventListener('focusin', update);
    return () => {
      document.removeEventListener('pointerover', update);
      document.removeEventListener('focusin', update);
    };
  }, []);
  const hint = target ? describeHint(target, keys, snapshot) : null;
  return <ContextHelpPanel hint={hint} expanded={expanded} onExpand={onExpand} />;
}

export function ContextHelpPanel({
  hint,
  expanded,
  onExpand,
}: {
  hint: Hint | null;
  expanded: boolean;
  onExpand: (expanded: boolean) => void;
}) {
  const button = useRef<HTMLButtonElement>(null);
  return (
    <section
      className={`context-help ${expanded ? 'is-expanded' : ''} tone-${hint?.tone ?? 'master'}`}
      data-context-help
      aria-label={tr('Ayuda contextual', 'Contextual help')}
      onKeyDown={(e) => {
        if (e.key !== 'Escape' || !expanded) return;
        e.preventDefault();
        e.stopPropagation();
        onExpand(false);
        button.current?.focus();
      }}
    >
      <div className="context-summary">
        <HelpCircle size={14} aria-hidden="true" />
        <span className="context-copy" role="status" aria-live="polite" aria-atomic="true">
          <strong>{hint?.label ?? tr('explora curado', 'explore curado')}</strong>
          <span className="context-excerpt">
            {hint?.summary ??
              tr(
                'Pasa el cursor o enfoca un control para conocerlo.',
                'Hover or focus a control to learn about it.',
              )}
          </span>
        </span>
        <span
          className="context-shortcuts"
          aria-label={tr('Atajos asignados', 'Assigned shortcuts')}
        >
          {hint?.shortcuts.map((key) => (
            <kbd key={key}>{key}</kbd>
          ))}
        </span>
        <button
          ref={button}
          type="button"
          className="context-toggle"
          aria-expanded={expanded}
          aria-controls="context-help-details"
          onClick={() => onExpand(!expanded)}
        >
          {expanded ? tr('menos', 'less') : tr('más info', 'more info')}
          {expanded ? <ChevronDown size={14} /> : <ChevronUp size={14} />}
        </button>
      </div>
      <div id="context-help-details" className="context-details" hidden={!expanded}>
        <p>
          {hint?.summary ??
            tr(
              'La ayuda sigue al control que señalas con el cursor o recorres con Tab.',
              'Help follows the control you point to or focus with Tab.',
            )}
        </p>
        <p>
          {hint?.detail ??
            tr(
              'Aquí aparecerán detalles de uso, atajos y consejos. Puedes cerrar esta sección con Escape.',
              'Usage details, shortcuts and tips appear here. Press Escape to close this section.',
            )}
        </p>
        {hint?.status && <p className="context-status">{hint.status}</p>}
      </div>
    </section>
  );
}
