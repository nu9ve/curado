import { tr } from '../lib/language';
import type { SortColumn, TrackSort } from '../lib/sorting';

export const ariaSort = (sort: TrackSort | null, ...columns: SortColumn[]) =>
  sort && columns.includes(sort.column)
    ? sort.descending
      ? ('descending' as const)
      : ('ascending' as const)
    : undefined;

/** A column's name, which sorts by it. */
export function SortButton({
  column,
  label,
  sort,
  onSort,
}: {
  column: SortColumn;
  label: string;
  sort: TrackSort | null;
  onSort: (column: SortColumn | null) => void;
}) {
  const active = sort?.column === column;
  return (
    <button
      type="button"
      className="sort-head"
      data-active={active}
      title={tr(`Ordenar por ${label}`, `Sort by ${label}`)}
      onClick={() => onSort(column)}
    >
      {label}
      {active && <span aria-hidden="true">{sort.descending ? '▼' : '▲'}</span>}
    </button>
  );
}

/** The number column: back to the list's own order. */
export function SortReset({ onSort }: { onSort: (column: SortColumn | null) => void }) {
  return (
    <button
      type="button"
      className="sort-head"
      title={tr('Volver al orden de la lista', 'Back to the list order')}
      onClick={() => onSort(null)}
    >
      #
    </button>
  );
}
