import type { CSSProperties } from 'react';
import { PRIORITY_COLOR, WORK_STATUS_COLOR, WORK_STATUS_LABEL } from '../taskFormat';
import { COL } from '../styles';

/** Critical is a solid badge so it never reads as the same signal as overdue (red text). */
export function PriorityBadge({ priority, size = 11 }: { priority: string; size?: number }) {
  const critical = priority === 'critical';
  return (
    <span
      style={{
        fontFamily: 'var(--font-display)', fontSize: size, fontWeight: 700, textTransform: 'uppercase',
        color: critical ? 'var(--ink)' : PRIORITY_COLOR[priority],
        background: critical ? 'var(--danger)' : 'transparent',
        padding: critical ? '1px 6px' : 0, borderRadius: 3,
      }}
    >
      {priority}
    </span>
  );
}

/** Every work-status pill is the same width so the column lines up. */
export function StatusPill({ status, style }: { status: string; style?: CSSProperties }) {
  return (
    <span
      style={{
        display: 'block', width: COL.status, flex: 'none', textAlign: 'center', whiteSpace: 'nowrap',
        fontSize: 10, fontWeight: 700, textTransform: 'uppercase', letterSpacing: '0.04em',
        padding: '2px 0', borderRadius: 999, color: 'var(--ink)', background: WORK_STATUS_COLOR[status],
        ...style,
      }}
    >
      {WORK_STATUS_LABEL[status]}
    </span>
  );
}

export function DoneCheck({ done, priority, size }: { done: boolean; priority: string; size: number }) {
  return (
    <span
      style={{
        width: size, height: size, flex: 'none', borderRadius: 999,
        border: `2px solid ${done ? 'var(--teal)' : PRIORITY_COLOR[priority]}`,
        background: done ? 'var(--teal)' : 'transparent',
        display: 'flex', alignItems: 'center', justifyContent: 'center',
      }}
    >
      {done && (
        <svg width={size - 8} height={size - 8} viewBox="0 0 24 24" fill="none" stroke="var(--ink)" strokeWidth={size > 18 ? 3.5 : 4} strokeLinecap="round" strokeLinejoin="round">
          <path d="M20 6L9 17l-5-5" />
        </svg>
      )}
    </span>
  );
}
