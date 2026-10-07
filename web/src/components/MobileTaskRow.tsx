import { useRef, useState } from 'react';
import { AlertTriangle, CalendarOff, Check, Repeat, UserX } from 'lucide-react';
import type { Task } from '../types';
import { isOverdue } from '../dueWindow';
import { addDays, dueColor, dueLabel, monthDay } from '../taskFormat';
import { DoneCheck, PriorityBadge, StatusPill } from './Badges';

/** Width of the quick-due buttons revealed by swiping left. */
const SWIPE_W = 216;
/** How far a drag has to travel before it commits on release. */
const SWIPE_COMMIT = 90;
const SWIPE_RIGHT_MAX = 140;

const QUICK_DUE: { label: string; days: number; bg: string; fg: string }[] = [
  { label: 'Today', days: 0, bg: 'var(--accent)', fg: 'var(--ink)' },
  { label: 'Tmrw', days: 1, bg: 'var(--cyan)', fg: 'var(--ink)' },
  { label: '+1 wk', days: 7, bg: 'var(--nord10)', fg: 'var(--fg-1)' },
];

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
  const [dragDx, setDragDx] = useState<number | null>(null);
  const drag = useRef<{ x0: number; y0: number; base: number; moved: boolean; vertical: boolean; pointerId: number } | null>(null);
  const justSwiped = useRef(false);

  const done = t.status === 'done';
  const open = t.status === 'open';
  const overdue = isOverdue(t, today);
  const dx = dragDx ?? (swipeOpen ? -SWIPE_W : 0);

  function handlePointerDown(e: React.PointerEvent<HTMLDivElement>) {
    drag.current = { x0: e.clientX, y0: e.clientY, base: swipeOpen ? -SWIPE_W : 0, moved: false, vertical: false, pointerId: e.pointerId };
  }

  function handlePointerMove(e: React.PointerEvent<HTMLDivElement>) {
    const g = drag.current;
    if (!g || g.vertical) return;
    const ddx = e.clientX - g.x0;
    const ddy = e.clientY - g.y0;
    if (!g.moved) {
      if (Math.abs(ddy) > 8 && Math.abs(ddy) > Math.abs(ddx)) { g.vertical = true; return; }
      if (Math.abs(ddx) <= 6) return;
      g.moved = true;
      try { e.currentTarget.setPointerCapture(g.pointerId); } catch { /* pointer already released */ }
    }
    setDragDx(Math.max(-SWIPE_W, Math.min(SWIPE_RIGHT_MAX, g.base + ddx)));
  }

  function handlePointerUp() {
    const g = drag.current;
    drag.current = null;
    if (!g || !g.moved) return;
    justSwiped.current = true;
    const final = dragDx ?? g.base;
    setDragDx(null);
    if (final > SWIPE_COMMIT) {
      onSwipeOpenChange(false);
      onToggleDone();
    } else {
      onSwipeOpenChange(final < -SWIPE_COMMIT);
    }
  }

  function handleClick() {
    if (justSwiped.current) { justSwiped.current = false; return; }
    if (swipeOpen) { onSwipeOpenChange(false); return; }
    onSelect();
  }

  return (
    <div data-task-row={t.id} style={{ position: 'relative', overflow: 'hidden', borderBottom: '1px solid var(--hairline-soft)' }}>
      <div
        aria-hidden
        style={{
          position: 'absolute', top: 0, bottom: 0, left: 0, right: SWIPE_W, background: 'var(--teal)',
          display: 'flex', alignItems: 'center', gap: 8, paddingLeft: 20, color: 'var(--ink)',
          fontSize: 13, fontWeight: 700, opacity: dx > 0 ? 1 : 0,
        }}
      >
        <Check size={18} strokeWidth={3} />
        {done ? 'Reopen' : 'Done'}
      </div>
      <div style={{ position: 'absolute', top: 0, bottom: 0, right: 0, width: SWIPE_W, display: 'flex' }}>
        {QUICK_DUE.map((q) => (
          <button
            key={q.label}
            tabIndex={swipeOpen ? 0 : -1}
            onClick={() => { onQuickDue(q.days); onSwipeOpenChange(false); }}
            style={{
              flex: 1, border: 'none', background: q.bg, color: q.fg, fontSize: 13, fontWeight: 700, cursor: 'pointer',
              display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 2,
            }}
          >
            {q.label}
            <span style={{ fontSize: 10.5, fontWeight: 600, fontFamily: 'var(--font-mono)' }}>{monthDay(addDays(today, q.days), today)}</span>
          </button>
        ))}
      </div>
      <div
        onClick={handleClick}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerCancel={handlePointerUp}
        style={{
          position: 'relative', zIndex: 1, display: 'flex', alignItems: 'flex-start', padding: '6px 16px 10px 0',
          borderLeft: '3px solid transparent', background: 'var(--ink)', cursor: 'pointer',
          touchAction: 'pan-y', userSelect: 'none', WebkitUserSelect: 'none',
          transform: `translateX(${dx}px)`, transition: dragDx === null ? 'transform 200ms ease-out' : 'none',
        }}
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
      </div>
    </div>
  );
}
