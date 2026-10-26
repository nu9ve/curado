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
      aria-label="Ayuda contextual"
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
          <strong>{hint?.label ?? 'explora curado'}</strong>
          <span className="context-excerpt">
            {hint?.summary ?? 'Pasa el cursor o enfoca un control para conocerlo.'}
          </span>
        </span>
        <span className="context-shortcuts" aria-label="Atajos asignados">
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
          {expanded ? 'menos' : 'más info'}
          {expanded ? <ChevronDown size={14} /> : <ChevronUp size={14} />}
        </button>
      </div>
      <div id="context-help-details" className="context-details" hidden={!expanded}>
        <p>
          {hint?.summary ??
            'La ayuda sigue al control que señalas con el cursor o recorres con Tab.'}
        </p>
        <p>
          {hint?.detail ??
            'Aquí aparecerán detalles de uso, atajos y consejos. Puedes cerrar esta sección con Escape.'}
        </p>
        {hint?.status && <p className="context-status">{hint.status}</p>}
      </div>
    </section>
  );
}
