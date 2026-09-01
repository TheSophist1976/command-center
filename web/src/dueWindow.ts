import type { Task } from './types';

export type DueWindow = 'day' | 'week' | 'month' | 'year';

export function countAllOpen(tasks: Task[]): number {
  return tasks.filter((t) => t.status !== 'done').length;
}

export function isIncomplete(t: Task): boolean {
  return t.status === 'open' && (!t.due_date || !t.agent);
}

export function isOverdue(t: Task, today: Date): boolean {
  return t.status === 'open' && !!t.due_date && parseDate(t.due_date) < today;
}

function parseDate(s: string): Date {
  const d = new Date(s + 'T00:00:00');
  d.setHours(0, 0, 0, 0);
  return d;
}

function startOfWeekMonday(d: Date): Date {
  const dayIndex = (d.getDay() + 6) % 7; // 0 = Monday
  const monday = new Date(d);
  monday.setDate(d.getDate() - dayIndex);
  monday.setHours(0, 0, 0, 0);
  return monday;
}

/**
 * Mirrors `due_matches` in src/tui.rs exactly: done tasks never match, overdue
 * open tasks match every window, Day also surfaces no-due-date/no-agent
 * ("incomplete") tasks and tasks with no due date at all.
 */
export function dueMatches(task: Task, today: Date, window: DueWindow): boolean {
  if (task.status === 'done') return false;

  if (task.due_date) {
    const d = parseDate(task.due_date);
    if (d < today) return true; // overdue always shown, in every window
  }

  switch (window) {
    case 'day': {
      if (isIncomplete(task)) return true;
      if (!task.due_date) return true;
      return parseDate(task.due_date).getTime() === today.getTime();
    }
    case 'week': {
      if (!task.due_date) return false;
      const d = parseDate(task.due_date);
      const monday = startOfWeekMonday(today);
      const sunday = new Date(monday);
      sunday.setDate(monday.getDate() + 6);
      return d >= monday && d <= sunday;
    }
    case 'month': {
      if (!task.due_date) return false;
      const d = parseDate(task.due_date);
      return d.getFullYear() === today.getFullYear() && d.getMonth() === today.getMonth();
    }
    case 'year': {
      if (!task.due_date) return false;
      const d = parseDate(task.due_date);
      return d.getFullYear() === today.getFullYear();
    }
  }
}

export function countDueWindow(tasks: Task[], today: Date, window: DueWindow): number {
  return tasks.filter((t) => dueMatches(t, today, window)).length;
}

export function startOfToday(): Date {
  const d = new Date();
  d.setHours(0, 0, 0, 0);
  return d;
}
