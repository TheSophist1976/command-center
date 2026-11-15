import { CalendarDays, HelpCircle, Repeat, Sun, type LucideIcon } from 'lucide-react';
import type { Task } from './types';
import { dueMatches, type DueWindow } from './dueWindow';

export type DueFilter = DueWindow | 'no-due-date' | 'recurring' | 'all-tasks';

export interface FilterItem {
  value: DueFilter;
  label: string;
  icon: LucideIcon;
  /** Empty-state title and body. */
  empty: [string, string];
}

export const DUE_WINDOW_ITEMS: FilterItem[] = [
  { value: 'day', label: 'Today', icon: Sun, empty: ['Nothing due today.', 'Pull something forward from This week, or add a task.'] },
  { value: 'week', label: 'This week', icon: CalendarDays, empty: ['Nothing due this week.', 'Tasks with a due date this week show up here.'] },
  { value: 'month', label: 'This month', icon: CalendarDays, empty: ['Nothing due this month.', ''] },
  { value: 'year', label: 'This year', icon: CalendarDays, empty: ['Nothing due this year.', ''] },
  { value: 'no-due-date', label: 'No due date', icon: HelpCircle, empty: ['Every task has a due date.', ''] },
  { value: 'recurring', label: 'Recurring', icon: Repeat, empty: ['No recurring tasks.', 'Set Recurrence on a task (e.g. weekly:fri) to repeat it.'] },
  { value: 'all-tasks', label: 'All tasks', icon: CalendarDays, empty: ['No tasks yet.', 'Add your first task.'] },
];

/** The filter pills of the All tasks view: everything first, then the narrowing filters. */
export const ALL_TASKS_FILTERS: FilterItem[] = [
  ...DUE_WINDOW_ITEMS.filter((i) => i.value === 'all-tasks'),
  ...DUE_WINDOW_ITEMS.filter((i) => i.value !== 'all-tasks'),
];

export function hasNoDueDate(t: Task): boolean {
  return t.status === 'open' && !t.due_date;
}

export function isRecurring(t: Task): boolean {
  return t.status === 'open' && !!t.recurrence;
}

/** Open work only a human can move: unassigned, human-assigned, awaiting review, or blocked on input. */
export function needsHumanAction(t: Task): boolean {
  return t.status === 'open' && (!t.agent || t.agent.toLowerCase() === 'human' || t.work_status === 'waiting-for-review' || t.work_status === 'needs-input');
}

export function matchesFilter(t: Task, today: Date, filter: DueFilter): boolean {
  switch (filter) {
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

/** Oldest created first; tasks without a created timestamp go last. */
function byOldest(a: Task, b: Task): number {
  const ca = Date.parse(a.created);
  const cb = Date.parse(b.created);
  const na = Number.isNaN(ca);
  const nb = Number.isNaN(cb);
  if (na || nb) return na === nb ? 0 : na ? 1 : -1;
  return ca - cb;
}

/** Next-queue order: deferred tasks sink below everything else; in each half, tasks missing an agent or due date come first (by priority), then the rest by due date then priority. Remaining ties go to the oldest task first. */
export function byDeferredThenDue(deferred: ReadonlySet<number>): (a: Task, b: Task) => number {
  return (a, b) => {
    const da = deferred.has(a.id);
    const db = deferred.has(b.id);
    if (da !== db) return da ? 1 : -1;
    const ta = !a.agent || !a.due_date;
    const tb = !b.agent || !b.due_date;
    if (ta !== tb) return ta ? -1 : 1;
    return (ta ? byPriority(a, b) : byDueThenPriority(a, b)) || byOldest(a, b);
  };
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
