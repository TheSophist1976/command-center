import { useRef, useState, type CSSProperties, type ReactNode } from 'react';
import { Check } from 'lucide-react';
import { addDays, monthDay } from '../taskFormat';

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

interface SwipeableRowProps {
  taskId: number;
  today: Date;
  /** The task is done, so swiping right reopens it instead of completing it. */
  done: boolean;
  /** Whether this row's quick-due buttons are revealed. Only one row is open at a time. */
  swipeOpen: boolean;
  onSwipeOpenChange: (open: boolean) => void;
  onSelect: () => void;
  onToggleDone: () => void;
  onQuickDue: (days: number) => void;
  /** The row is itself the control (no nested buttons), so expose it to assistive tech and the keyboard as one. */
  asButton?: boolean;
  selected?: boolean;
  /** Merged over the default layout of the sliding surface. */
  contentStyle?: CSSProperties;
  children: ReactNode;
}

/**
 * The phone swipe gestures shared by every task row: swipe right to mark done (or reopen),
 * left to reveal Today / Tomorrow / +1 week. The row's own content goes in `children`.
 */
export function SwipeableRow({
  taskId, today, done, swipeOpen, onSwipeOpenChange, onSelect, onToggleDone, onQuickDue, asButton, selected, contentStyle, children,
}: SwipeableRowProps) {
  const [dragDx, setDragDx] = useState<number | null>(null);
  const drag = useRef<{ x0: number; y0: number; base: number; moved: boolean; vertical: boolean; pointerId: number } | null>(null);
  const justSwiped = useRef(false);
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
    <div data-task-row={taskId} style={{ position: 'relative', overflow: 'hidden', borderBottom: '1px solid var(--hairline-soft)' }}>
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
        {...(asButton
          ? {
              role: 'button',
              tabIndex: 0,
              'aria-current': selected ? ('true' as const) : undefined,
              onKeyDown: (e: React.KeyboardEvent) => {
                if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); handleClick(); }
              },
            }
          : {})}
        onClick={handleClick}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerCancel={handlePointerUp}
        style={{
          position: 'relative', zIndex: 1, display: 'flex', alignItems: 'flex-start', padding: '6px 16px 10px 0',
          borderLeft: '3px solid transparent', background: 'var(--ink)', cursor: 'pointer',
          touchAction: 'pan-y', userSelect: 'none', WebkitUserSelect: 'none',
          ...contentStyle,
          transform: `translateX(${dx}px)`, transition: dragDx === null ? 'transform 200ms ease-out' : 'none',
        }}
      >
        {children}
      </div>
    </div>
  );
}
