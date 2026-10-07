import type { Task } from './types';
import { isOverdue } from './dueWindow';

export const WORK_STATUS_LABEL: Record<string, string> = {
  'todo': 'To Do',
  'in-progress': 'In Progress',
  'waiting-for-review': 'Waiting for Review',
  'changes-requested': 'Changes Requested',
  'needs-input': 'Needs Input',
  'complete': 'Complete',
};

export const WORK_STATUS_COLOR: Record<string, string> = {
  'todo': 'var(--fg-4)',
  'in-progress': 'var(--citrine)',
  'waiting-for-review': 'var(--cyan)',
  'changes-requested': 'var(--danger)',
  'needs-input': 'var(--danger)',
  'complete': 'var(--teal)',
};

export const PRIORITY_COLOR: Record<string, string> = {
  critical: 'var(--danger)',
  high: 'var(--citrine)',
  medium: 'var(--fg-3)',
  low: 'var(--fg-5)',
};

export function parseLocalDate(s: string): Date {
  const [y, m, d] = s.split('-').map(Number);
  return new Date(y, m - 1, d);
}

export function formatLocalDate(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
}

export function addDays(d: Date, days: number): Date {
  const copy = new Date(d);
  copy.setDate(copy.getDate() + days);
  return copy;
}

export function addMonths(d: Date, months: number): Date {
  const copy = new Date(d);
  copy.setMonth(copy.getMonth() + months);
  return copy;
}

/** "Oct 14", with the year added when it isn't the current one. */
export function monthDay(d: Date, today: Date): string {
  return d.toLocaleDateString('en-US', {
    month: 'short',
    day: 'numeric',
    ...(d.getFullYear() !== today.getFullYear() ? { year: 'numeric' } : {}),
  });
}

/**
 * Human-readable due date: "4d overdue", "Today", "Tomorrow", a weekday for
 * the rest of the coming week, otherwise "Oct 14". Empty when there's no date.
 */
export function dueLabel(t: Task, today: Date): string {
  if (!t.due_date) return '';
  const d = parseLocalDate(t.due_date);
  const diff = Math.round((d.getTime() - today.getTime()) / 86_400_000);
  if (diff < 0 && t.status === 'open') return `${-diff}d overdue`;
  if (diff === 0) return 'Today';
  if (diff === 1) return 'Tomorrow';
  if (diff > 1 && diff < 7) return d.toLocaleDateString('en-US', { weekday: 'short' });
  return monthDay(d, today);
}

/** Inspector form: "Tomorrow · Oct 8". */
export function dueLabelLong(t: Task, today: Date): string {
  if (!t.due_date) return '';
  return `${dueLabel(t, today)} · ${monthDay(parseLocalDate(t.due_date), today)}`;
}

export function dueColor(t: Task, today: Date): string {
  if (isOverdue(t, today)) return 'var(--danger)';
  return t.due_date ? 'var(--fg-3)' : 'var(--fg-5)';
}
