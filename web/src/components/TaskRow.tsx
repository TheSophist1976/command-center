import { AlertTriangle, CalendarOff, Repeat, UserX } from 'lucide-react';
import type { Task } from '../types';
import { isOverdue } from '../dueWindow';
import { dueColor, dueLabel } from '../taskFormat';
import { DoneCheck, PriorityBadge, StatusPill } from './Badges';
import { COL } from '../styles';

interface TaskRowProps {
  task: Task;
  today: Date;
  selected: boolean;
  onSelect: () => void;
  onToggleDone: () => void;
}

export function TaskRow({ task: t, today, selected, onSelect, onToggleDone }: TaskRowProps) {
  const open = t.status === 'open';
  const done = t.status === 'done';
  const overdue = isOverdue(t, today);
  return (
    <div
      data-task-row={t.id}
      className="task-row"
      onClick={onSelect}
      style={{
        display: 'flex', alignItems: 'center', gap: 16, height: 46, padding: '0 24px 0 21px',
        borderBottom: '1px solid var(--hairline-soft)', cursor: 'pointer',
        background: selected ? 'var(--accent-soft)' : undefined,
        borderLeft: `3px solid ${selected ? 'var(--accent)' : 'transparent'}`,
      }}
    >
      <button
        onClick={(e) => { e.stopPropagation(); onToggleDone(); }}
        title={done ? 'Reopen' : 'Mark done'}
        aria-label={done ? 'Reopen' : 'Mark done'}
        style={{ width: COL.check, height: COL.check, flex: 'none', padding: 0, border: 'none', background: 'transparent', cursor: 'pointer', display: 'flex' }}
      >
        <DoneCheck done={done} priority={t.priority} size={18} />
      </button>
      <span style={{ width: COL.id, flex: 'none', fontFamily: 'var(--font-mono)', fontSize: 12, color: 'var(--fg-5)' }}>{t.id}</span>
      <span style={{ width: COL.priority, flex: 'none', display: 'flex' }}>
        <PriorityBadge priority={t.priority} />
      </span>
      <span style={{ width: COL.flags, flex: 'none', display: 'flex', alignItems: 'center', gap: 6 }}>
        {open && !t.agent && (
          <span title="Needs an agent" style={{ display: 'flex' }}><UserX size={14} color="var(--citrine)" /></span>
        )}
        {open && !t.due_date && (
          <span title="Needs a due date" style={{ display: 'flex' }}><CalendarOff size={14} color="var(--citrine)" /></span>
        )}
      </span>
      <span style={{ width: COL.status, flex: 'none' }}>
        {t.work_status && <StatusPill status={t.work_status} />}
      </span>
      <span style={{
        width: COL.agent, flex: 'none', fontSize: 12.5, color: t.agent ? 'var(--fg-3)' : 'var(--fg-5)',
        whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis',
      }}>
        {t.agent ?? '—'}
      </span>
      <span style={{
        flex: 1, minWidth: 0, fontSize: 14.5, whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: 'ellipsis',
        textDecoration: done ? 'line-through' : 'none', color: done ? 'var(--fg-4)' : 'var(--fg-1)',
      }}>
        {t.title}
      </span>
      <span
        title={t.due_date ?? 'No due date'}
        style={{
          width: COL.due, flex: 'none', display: 'flex', alignItems: 'center', gap: 6, fontFamily: 'var(--font-mono)', fontSize: 12.5,
          color: dueColor(t, today), fontWeight: overdue ? 700 : 400, whiteSpace: 'nowrap',
        }}
      >
        {overdue && <AlertTriangle size={13} color="var(--danger)" style={{ flex: 'none' }} />}
        {t.due_date ? dueLabel(t, today) : '—'}
        {t.recurrence && (
          <span title={t.recurrence} style={{ display: 'flex' }}><Repeat size={12} color="var(--fg-4)" /></span>
        )}
      </span>
      <span style={{ width: COL.effort, flex: 'none', fontFamily: 'var(--font-mono)', fontSize: 12, color: 'var(--fg-3)' }}>
        {t.effort ?? '—'}
      </span>
    </div>
  );
}
