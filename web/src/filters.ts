import { CalendarDays, HelpCircle, Inbox, ListTodo, Repeat, Sun, type LucideIcon } from 'lucide-react';
import type { Task } from './types';
import { dueMatches, isOverdue, type DueWindow } from './dueWindow';

export type DueFilter = DueWindow | 'no-due-date' | 'recurring' | 'all-tasks' | 'inbox' | 'next';

export interface FilterItem {
  value: DueFilter;
  label: string;
  icon: LucideIcon;
  /** Empty-state title and body. */
  empty: [string, string];
}

export const NEXT_ITEM: FilterItem = {
  value: 'next', label: 'Next', icon: ListTodo,
  empty: ['Nothing needs you.', 'No open tasks are assigned to you, awaiting review, or blocked on your input.'],
};

export const INBOX_ITEM: FilterItem = {
  value: 'inbox', label: 'Inbox', icon: Inbox,
  empty: ['Inbox zero.', 'Every open task has a due date and an agent, and nothing is overdue.'],
};

export const DUE_WINDOW_ITEMS: FilterItem[] = [
  { value: 'day', label: 'Today', icon: Sun, empty: ['Nothing due today.', 'Pull something forward from This week, or add a task.'] },
  { value: 'week', label: 'This week', icon: CalendarDays, empty: ['Nothing due this week.', 'Tasks with a due date this week show up here.'] },
  { value: 'month', label: 'This month', icon: CalendarDays, empty: ['Nothing due this month.', ''] },
  { value: 'year', label: 'This year', icon: CalendarDays, empty: ['Nothing due this year.', ''] },
  { value: 'no-due-date', label: 'No due date', icon: HelpCircle, empty: ['Every task has a due date.', ''] },
  { value: 'recurring', label: 'Recurring', icon: Repeat, empty: ['No recurring tasks.', 'Set Recurrence on a task (e.g. weekly:fri) to repeat it.'] },
  { value: 'all-tasks', label: 'All tasks', icon: CalendarDays, empty: ['No tasks yet.', 'Add your first task.'] },
];

export const ALL_FILTER_ITEMS: FilterItem[] = [NEXT_ITEM, INBOX_ITEM, ...DUE_WINDOW_ITEMS];

export function hasNoDueDate(t: Task): boolean {
  return t.status === 'open' && !t.due_date;
}

export function isRecurring(t: Task): boolean {
  return t.status === 'open' && !!t.recurrence;
}

export function isInboxTask(t: Task, today: Date): boolean {
  return t.status === 'open' && (!t.agent || !t.due_date || isOverdue(t, today));
}

/** Open work only a human can move: human-assigned, awaiting review, or blocked on input. */
export function needsHumanAction(t: Task): boolean {
  return t.status === 'open' && (t.agent?.toLowerCase() === 'human' || t.work_status === 'waiting-for-review' || t.work_status === 'needs-input');
}

export function matchesFilter(t: Task, today: Date, filter: DueFilter): boolean {
  switch (filter) {
    case 'next':
      return needsHumanAction(t);
    case 'inbox':
      return isInboxTask(t, today);
    case 'all-tasks':
      return true;
    case 'no-due-date':
      return hasNoDueDate(t);
    case 'recurring':
      return isRecurring(t);
    default:
      return dueMatches(t, today, filter);
  }
}

export function countForFilter(tasks: Task[], today: Date, filter: DueFilter): number {
  return tasks.filter((t) => matchesFilter(t, today, filter)).length;
}

export type GroupBy = 'agent' | 'project' | 'priority' | 'work_status' | 'none';

export const GROUP_BY_OPTIONS: { value: GroupBy; label: string }[] = [
  { value: 'agent', label: 'Agent' },
  { value: 'project', label: 'Project' },
  { value: 'priority', label: 'Priority' },
  { value: 'work_status', label: 'Work status' },
  { value: 'none', label: 'None' },
];

const PRIORITY_ORDER: Record<string, number> = { critical: 0, high: 1, medium: 2, low: 3 };

function groupKey(task: Task, groupBy: GroupBy): string {
  switch (groupBy) {
    case 'agent':
      return task.agent ?? 'unassigned';
    case 'project':
      return task.project ?? 'no project';
    case 'priority':
      return task.priority;
    case 'work_status':
      return task.work_status ?? 'no status';
    case 'none':
      return '';
  }
}

function byPriority(a: Task, b: Task): number {
  return (PRIORITY_ORDER[a.priority] ?? 99) - (PRIORITY_ORDER[b.priority] ?? 99);
}

/** Earliest due date first (undated last), then highest priority. */
export function byDueThenPriority(a: Task, b: Task): number {
  if (a.due_date !== b.due_date) {
    if (!a.due_date) return 1;
    if (!b.due_date) return -1;
    return a.due_date < b.due_date ? -1 : 1;
  }
  return byPriority(a, b);
}

export function groupTasks(tasks: Task[], groupBy: GroupBy, compare: (a: Task, b: Task) => number = byPriority): Map<string, Task[]> {
  const groups = new Map<string, Task[]>();
  for (const t of tasks) {
    const key = groupKey(t, groupBy);
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key)!.push(t);
  }
  for (const group of groups.values()) {
    group.sort(compare);
  }
  if (groupBy === 'priority') {
    return new Map([...groups.entries()].sort((a, b) => (PRIORITY_ORDER[a[0]] ?? 99) - (PRIORITY_ORDER[b[0]] ?? 99)));
  }
  return groups;
}
