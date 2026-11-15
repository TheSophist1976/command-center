import type { Task } from '../types';
import { countForFilter, type DueFilter, type FilterItem } from '../filters';

interface FilterPillsProps {
  items: FilterItem[];
  active: DueFilter;
  tasks: Task[];
  today: Date;
  onPick: (filter: DueFilter) => void;
  /** Phone layout: taller pills that scroll sideways. */
  mobile?: boolean;
}

/** The All tasks view's filters (due windows, no due date, recurring) as one row of pills. */
export function FilterPills({ items, active, tasks, today, onPick, mobile }: FilterPillsProps) {
  return (
    <div
      className="no-scrollbar"
      role="group"
      aria-label="Filter tasks"
      style={{ display: 'flex', gap: 8, overflowX: 'auto', flex: 'none', alignItems: 'center' }}
    >
      {items.map((item) => {
        const on = active === item.value;
        return (
          <button
            key={item.value}
            aria-pressed={on}
            onClick={() => onPick(item.value)}
            style={{
              flex: 'none', display: 'flex', alignItems: 'center', gap: 6, height: mobile ? 34 : 28, padding: '0 12px',
              borderRadius: 999, cursor: 'pointer', whiteSpace: 'nowrap',
              border: `1px solid ${on ? 'var(--accent-ring)' : 'var(--hairline)'}`,
              background: on ? 'var(--accent-soft)' : 'transparent',
            }}
          >
            <span style={{ fontSize: mobile ? 13.5 : 12.5, fontWeight: on ? 600 : 400, color: on ? 'var(--fg-1)' : 'var(--fg-3)' }}>
              {item.value === 'all-tasks' ? 'All' : item.label}
            </span>
            <span style={{ fontFamily: 'var(--font-mono)', fontSize: 11.5, color: on ? 'var(--accent)' : 'var(--fg-5)' }}>
              {countForFilter(tasks, today, item.value)}
            </span>
          </button>
        );
      })}
    </div>
  );
}
