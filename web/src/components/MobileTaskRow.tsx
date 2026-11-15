import { AlertTriangle, CalendarOff, Repeat, UserX } from 'lucide-react';
import type { Task } from '../types';
import { isOverdue } from '../dueWindow';
import { dueColor, dueLabel } from '../taskFormat';
import { DoneCheck, PriorityBadge, StatusPill } from './Badges';
import { SwipeableRow } from './SwipeableRow';

interface MobileTaskRowProps {
  task: Task;
  today: Date;
  /** Show the agent name in the meta line (hidden when the list is grouped by agent). */
  showAgent: boolean;
  /** Whether this row's quick-due buttons are revealed. Only one row is open at a time. */
  swipeOpen: boolean;
  onSwipeOpenChange: (open: boolean) => void;
  onSelect: () => void;
  onToggleDone: () => void;
  onQuickDue: (days: number) => void;
}

/** Phone list row. Swipe right to mark done (or reopen), left to reveal Today / Tomorrow / +1 week. */
export function MobileTaskRow({ task: t, today, showAgent, swipeOpen, onSwipeOpenChange, onSelect, onToggleDone, onQuickDue }: MobileTaskRowProps) {
  const done = t.status === 'done';
  const open = t.status === 'open';
  const overdue = isOverdue(t, today);

  return (
    <SwipeableRow
      taskId={t.id}
      today={today}
      done={done}
      swipeOpen={swipeOpen}
      onSwipeOpenChange={onSwipeOpenChange}
      onSelect={onSelect}
      onToggleDone={onToggleDone}
      onQuickDue={onQuickDue}
    >
    <button
      onClick={(e) => { e.stopPropagation(); onToggleDone(); }}
      onPointerDown={(e) => e.stopPropagation()}
      aria-label={done ? 'Reopen' : 'Mark done'}
      style={{ width: 48, height: 44, flex: 'none', display: 'flex', alignItems: 'center', justifyContent: 'center', background: 'transparent', border: 'none', cursor: 'pointer' }}
    >
      <DoneCheck done={done} priority={t.priority} size={20} />
    </button>
    <div style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column', gap: 6, paddingTop: 6 }}>
      <div
        style={{
          fontSize: 15, lineHeight: 1.35, textDecoration: done ? 'line-through' : 'none', color: done ? 'var(--fg-4)' : 'var(--fg-1)',
          display: '-webkit-box', WebkitLineClamp: 2, WebkitBoxOrient: 'vertical', overflow: 'hidden', textWrap: 'pretty',
        }}
      >
        {t.title}
      </div>
      <div style={{ display: 'flex', alignItems: 'center', flexWrap: 'wrap', gap: 8, fontSize: 12, color: 'var(--fg-4)' }}>
        <span style={{ fontFamily: 'var(--font-mono)', color: 'var(--fg-5)' }}>#{t.id}</span>
        <PriorityBadge priority={t.priority} size={10.5} />
        {t.work_status && <StatusPill status={t.work_status} />}
        {showAgent && t.agent && <span style={{ fontFamily: 'var(--font-mono)', color: 'var(--fg-3)' }}>{t.agent}</span>}
        {open && !t.agent && <UserX size={14} color="var(--citrine)" aria-label="Needs an agent" />}
        <span style={{ flex: 1 }} />
        <span style={{ display: 'flex', alignItems: 'center', gap: 4, fontFamily: 'var(--font-mono)', color: dueColor(t, today), fontWeight: overdue ? 700 : 400 }}>
          {overdue && <AlertTriangle size={12} color="var(--danger)" />}
          {open && !t.due_date && <CalendarOff size={13} color="var(--citrine)" aria-label="Needs a due date" />}
          {dueLabel(t, today)}
          {t.recurrence && <Repeat size={12} color="var(--fg-4)" aria-label={t.recurrence} />}
        </span>
      </div>
    </div>
    </SwipeableRow>
  );
}
