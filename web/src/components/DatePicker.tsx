import { useEffect, useRef, useState, type CSSProperties } from 'react';
import { ChevronLeft, ChevronRight } from 'lucide-react';

interface DatePickerProps {
  id?: string;
  value: string; // 'YYYY-MM-DD' or ''
  onSave: (value: string) => void;
}

function parseLocalDate(s: string): Date {
  const [y, m, d] = s.split('-').map(Number);
  return new Date(y, m - 1, d);
}

function formatLocalDate(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

function startOfToday(): Date {
  const d = new Date();
  d.setHours(0, 0, 0, 0);
  return d;
}

function addDays(d: Date, days: number): Date {
  const copy = new Date(d);
  copy.setDate(copy.getDate() + days);
  return copy;
}

function sameDay(a: Date, b: Date): boolean {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
}

const WEEKDAY_LABELS = ['Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa', 'Su'];

const iconButtonStyle: CSSProperties = {
  background: 'transparent', border: 'none', cursor: 'pointer', display: 'flex',
  alignItems: 'center', justifyContent: 'center', padding: 4, color: 'var(--fg-4)',
};

const smallButtonStyle: CSSProperties = {
  flex: 1, background: 'var(--ink-3)', border: '1px solid var(--hairline)', borderRadius: 5,
  color: 'var(--fg-2)', fontSize: 12, padding: '5px 0', cursor: 'pointer',
};

export function DatePicker({ id, value, onSave }: DatePickerProps) {
  const [open, setOpen] = useState(false);
  const today = startOfToday();
  const selectedDate = value ? parseLocalDate(value) : null;
  const [highlighted, setHighlighted] = useState(selectedDate ?? today);
  const [viewYear, setViewYear] = useState(highlighted.getFullYear());
  const [viewMonth, setViewMonth] = useState(highlighted.getMonth());
  const containerRef = useRef<HTMLDivElement>(null);
  const popoverRef = useRef<HTMLDivElement>(null);

  function openPicker() {
    const base = selectedDate ?? today;
    setHighlighted(base);
    setViewYear(base.getFullYear());
    setViewMonth(base.getMonth());
    setOpen(true);
  }

  function choose(d: Date) {
    onSave(formatLocalDate(d));
    setOpen(false);
  }

  function moveHighlight(next: Date) {
    setHighlighted(next);
    setViewYear(next.getFullYear());
    setViewMonth(next.getMonth());
  }

  useEffect(() => {
    if (!open) return;
    popoverRef.current?.focus();
    function handleClickOutside(e: MouseEvent) {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    }
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [open]);

  function handleKeyDown(e: React.KeyboardEvent) {
    // Every branch stops propagation so the app's global keyboard shortcuts
    // can't fire underneath this popover while it's open.
    if (e.key === 'ArrowLeft') {
      e.preventDefault();
      e.stopPropagation();
      moveHighlight(addDays(highlighted, -1));
    } else if (e.key === 'ArrowRight') {
      e.preventDefault();
      e.stopPropagation();
      moveHighlight(addDays(highlighted, 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      e.stopPropagation();
      moveHighlight(addDays(highlighted, -7));
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      e.stopPropagation();
      moveHighlight(addDays(highlighted, 7));
    } else if (e.key === 'Enter') {
      e.preventDefault();
      e.stopPropagation();
      choose(highlighted);
    } else if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      setOpen(false);
    } else {
      e.stopPropagation();
    }
  }

  function shiftMonth(delta: number) {
    let m = viewMonth + delta;
    let y = viewYear;
    if (m < 0) { m = 11; y -= 1; }
    if (m > 11) { m = 0; y += 1; }
    setViewMonth(m);
    setViewYear(y);
  }

  const firstOfMonth = new Date(viewYear, viewMonth, 1);
  const daysInMonth = new Date(viewYear, viewMonth + 1, 0).getDate();
  const leadingBlanks = (firstOfMonth.getDay() + 6) % 7; // Monday-first grid
  const cells: (Date | null)[] = [
    ...Array.from({ length: leadingBlanks }, () => null),
    ...Array.from({ length: daysInMonth }, (_, i) => new Date(viewYear, viewMonth, i + 1)),
  ];
  const monthLabel = firstOfMonth.toLocaleDateString(undefined, { month: 'long', year: 'numeric' });

  return (
    <div ref={containerRef} style={{ position: 'relative' }}>
      <div id={id} onClick={openPicker} style={{ cursor: 'pointer', minHeight: 20 }} title="Click to pick a date">
        {value || <span style={{ color: 'var(--fg-5)' }}>No due date</span>}
      </div>
      {open && (
        <div
          ref={popoverRef}
          tabIndex={-1}
          onKeyDown={handleKeyDown}
          style={{
            position: 'absolute', top: '100%', left: 0, zIndex: 50, marginTop: 4,
            background: 'var(--ink-2)', border: '1px solid var(--hairline)', borderRadius: 8,
            padding: 12, width: 240, boxShadow: '0 4px 16px rgba(0,0,0,0.4)', outline: 'none',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', marginBottom: 8 }}>
            <button onClick={() => shiftMonth(-1)} style={iconButtonStyle} title="Previous month">
              <ChevronLeft size={14} />
            </button>
            <span style={{ flex: 1, textAlign: 'center', fontSize: 13, fontWeight: 600, color: 'var(--fg-1)' }}>
              {monthLabel}
            </span>
            <button onClick={() => shiftMonth(1)} style={iconButtonStyle} title="Next month">
              <ChevronRight size={14} />
            </button>
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(7, 1fr)', gap: 2, marginBottom: 4 }}>
            {WEEKDAY_LABELS.map((w) => (
              <span key={w} style={{ fontSize: 10, textAlign: 'center', color: 'var(--fg-5)' }}>{w}</span>
            ))}
          </div>
          <div style={{ display: 'grid', gridTemplateColumns: 'repeat(7, 1fr)', gap: 2 }}>
            {cells.map((d, i) => {
              if (!d) return <span key={i} />;
              const isToday = sameDay(d, today);
              const isSelected = !!selectedDate && sameDay(d, selectedDate);
              const isHighlighted = sameDay(d, highlighted);
              return (
                <button
                  key={i}
                  onClick={() => choose(d)}
                  onMouseEnter={() => setHighlighted(d)}
                  style={{
                    height: 26, borderRadius: 4, border: 'none', cursor: 'pointer', fontSize: 12,
                    boxShadow: isHighlighted ? '0 0 0 1px var(--magenta)' : 'none',
                    background: isSelected ? 'var(--magenta)' : isToday ? 'rgba(255,0,149,0.15)' : 'transparent',
                    color: isSelected ? 'var(--ink)' : 'var(--fg-2)',
                  }}
                >
                  {d.getDate()}
                </button>
              );
            })}
          </div>
          <div style={{ display: 'flex', gap: 6, marginTop: 10, borderTop: '1px solid var(--hairline-soft)', paddingTop: 10 }}>
            <button onClick={() => choose(today)} style={smallButtonStyle}>Today</button>
            <button onClick={() => { onSave(''); setOpen(false); }} style={smallButtonStyle}>Clear</button>
          </div>
        </div>
      )}
    </div>
  );
}
