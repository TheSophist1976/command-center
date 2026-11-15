export type View = 'next' | 'all';

interface ViewToggleProps {
  view: View;
  onChange: (view: View) => void;
  /** Tasks that need the human, shown next to "Next". */
  nextCount: number;
  /** Every task, shown next to "All tasks". */
  allCount: number;
  /** Phone layout: full width, bigger touch targets. */
  mobile?: boolean;
}

const OPTIONS: { value: View; label: string }[] = [
  { value: 'next', label: 'Next' },
  { value: 'all', label: 'All tasks' },
];

/** The one navigation control: the focused Next queue, or the full task list. */
export function ViewToggle({ view, onChange, nextCount, allCount, mobile }: ViewToggleProps) {
  const counts: Record<View, number> = { next: nextCount, all: allCount };
  return (
    <div
      role="tablist"
      aria-label="View"
      style={{
        display: 'flex', flex: mobile ? 1 : 'none', padding: 3, gap: 2, background: 'var(--ink-2)',
        border: '1px solid var(--hairline)', borderRadius: mobile ? 8 : 6,
      }}
    >
      {OPTIONS.map((o) => {
        const on = view === o.value;
        return (
          <button
            key={o.value}
            role="tab"
            aria-selected={on}
            onClick={() => onChange(o.value)}
            style={{
              flex: mobile ? 1 : 'none', height: mobile ? 38 : 28, padding: '0 14px', border: 'none', borderRadius: mobile ? 6 : 4,
              cursor: 'pointer', fontSize: mobile ? 15 : 13.5, fontWeight: 600, whiteSpace: 'nowrap',
              background: on ? 'var(--accent-soft)' : 'transparent',
              color: on ? 'var(--accent)' : 'var(--fg-4)',
              boxShadow: on ? '0 0 0 1px var(--accent-ring)' : 'none',
            }}
          >
            {o.label}
            <span aria-label={`${counts[o.value]} tasks`} style={{ marginLeft: 8, fontFamily: 'var(--font-mono)', fontSize: mobile ? 12.5 : 12, opacity: 0.85 }}>
              {counts[o.value]}
            </span>
          </button>
        );
      })}
    </div>
  );
}
