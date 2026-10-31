import { tr } from '../lib/language';

/** A focusable reference control, never an audio action. */
export function UnavailableControl({ label, dial = false }: { label: string; dial?: boolean }) {
  return (
    <button
      type="button"
      className={`rx-soon ${dial ? 'rx-soon-dial' : ''}`}
      aria-disabled="true"
      aria-label={`${label} · ${tr('próximamente, sin función', 'coming soon, unavailable')}`}
      title={tr('Próximamente · todavía no tiene función', 'Coming soon · not yet functional')}
    >
      {dial && <i aria-hidden="true" />}
      <span>{label}</span>
    </button>
  );
}
