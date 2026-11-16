import { Focus } from 'lucide-react';

interface FocusToggleProps {
  on: boolean;
  onChange: (on: boolean) => void;
  /** How many tasks Focus is hiding; shown while it is on so the queue never looks emptier than it is. */
  hidden: number;
  /** Phone layout: a taller touch target. */
  mobile?: boolean;
}

/** Narrows the Next queue to what can be worked right now. */
export function FocusToggle({ on, onChange, hidden, mobile }: FocusToggleProps) {
  return (
    <button
      type="button"
      aria-pressed={on}
      onClick={() => onChange(!on)}
      title="Show only what can be worked right now: hides deferred tasks, agents' work and anything due after today (f)"
      style={{
        display: 'flex', alignItems: 'center', gap: 7, flex: 'none', height: mobile ? 34 : 28, padding: '0 12px', borderRadius: 999,
        cursor: 'pointer', whiteSpace: 'nowrap', fontSize: mobile ? 13.5 : 12.5, fontWeight: on ? 600 : 400,
        border: `1px solid ${on ? 'var(--accent-ring)' : 'var(--hairline)'}`,
        background: on ? 'var(--accent-soft)' : 'transparent',
        color: on ? 'var(--accent)' : 'var(--fg-3)',
      }}
    >
      <Focus size={mobile ? 15 : 14} aria-hidden />
      Focus
      {on && hidden > 0 && (
        <span aria-label={`${hidden} hidden`} style={{ fontFamily: 'var(--font-mono)', fontSize: 11.5, color: 'var(--fg-4)' }}>
          {hidden} hidden
        </span>
      )}
    </button>
  );
}
