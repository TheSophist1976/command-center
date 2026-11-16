import type { Task } from './types';
import { byDeferredThenDue, needsHumanAction } from './filters';
import { formatLocalDate } from './taskFormat';

/**
 * What a task in the Next view is waiting on: `review` (an agent finished and wants approval),
 * `input` (an agent asked a question), `triage` (unassigned or assigned to the human), or
 * `working` (an agent is on it). It drives the row's icon and reason line, not its position:
 * the first three share one list ordered by urgency.
 */
export type Band = 'review' | 'input' | 'triage' | 'working';

/** The Next view lists `needs` (everything waiting on the human, one ordered list) above `working` (awareness only). */
export type Section = 'needs' | 'working';

export const SECTION_LABEL: Record<Section, string> = {
  needs: 'Needs you',
  working: 'Agents working',
};

export interface QueueSection {
  section: Section;
  tasks: Task[];
}

/**
 * Which band a task belongs to, or null when it is not in Next at all.
 * Review, input and triage together are exactly `needsHumanAction`, so the
 * "needs you" count is unchanged from before the bands existed.
 */
export function bandOf(t: Task): Band | null {
  if (t.status !== 'open') return null;
  if (t.work_status === 'waiting-for-review') return 'review';
  if (t.work_status === 'needs-input') return 'input';
  if (needsHumanAction(t)) return 'triage';
  if (t.work_status === 'in-progress' || t.work_status === 'changes-requested') return 'working';
  return null;
}

/**
 * Focus mode keeps only what can be worked right now: waiting on the human (not the agents' work),
 * not snoozed with Defer, and not due after today. Overdue, due-today and undated tasks stay.
 */
export function isFocusable(t: Task, deferred: ReadonlySet<number>, today: Date): boolean {
  const band = bandOf(t);
  if (band === null || band === 'working') return false;
  if (deferred.has(t.id)) return false;
  return !t.due_date || t.due_date <= formatLocalDate(today);
}

export interface QueueOptions {
  /** Show only what can be worked right now (see `isFocusable`). Needs `today`. */
  focus?: boolean;
  today?: Date;
}

/**
 * The queue: "Needs you" as one list in the order the Next view has always used (see
 * `byDeferredThenDue`: tasks missing an agent or due date first, then due date and priority, oldest
 * last, deferred tasks last of all), then "Agents working" in the same order.
 */
export function buildQueue(tasks: Task[], deferred: ReadonlySet<number> = new Set(), opts: QueueOptions = {}): QueueSection[] {
  const compare = byDeferredThenDue(deferred);
  const pool = opts.focus && opts.today ? tasks.filter((t) => isFocusable(t, deferred, opts.today!)) : tasks;
  const needs = pool.filter((t) => { const b = bandOf(t); return b !== null && b !== 'working'; }).sort(compare);
  const working = pool.filter((t) => bandOf(t) === 'working').sort(compare);
  return [{ section: 'needs', tasks: needs }, { section: 'working', tasks: working }];
}

/** How many tasks a queue lists, across all its sections. */
export function queueSize(queue: QueueSection[]): number {
  return queue.reduce((n, s) => n + s.tasks.length, 0);
}

/** The tasks in keyboard order. A collapsed working section is skipped because its rows are not on screen. */
export function queueOrder(queue: QueueSection[], opts: { workingExpanded: boolean }): Task[] {
  return queue.filter((s) => s.section !== 'working' || opts.workingExpanded).flatMap((s) => s.tasks);
}

/** Tasks that need the human: the badge on the Next toggle. */
export function needsYouCount(tasks: Task[]): number {
  return tasks.filter((t) => {
    const band = bandOf(t);
    return band !== null && band !== 'working';
  }).length;
}

/**
 * Which task should be selected after the queue changed.
 * - the selected task is still queued: keep it
 * - it just left the queue (approved, answered, completed): the nearest task that is still there,
 *   preferring the one that followed it, so you can keep working down the list
 * - nothing was selected, or the selection was never in the queue: the first task when
 *   `autoSelectFirst` (the docked desktop pane is never empty), otherwise leave it alone
 */
export function nextSelection(
  prevIds: readonly number[],
  currIds: readonly number[],
  selectedId: number | null,
  autoSelectFirst: boolean,
): number | null {
  if (selectedId !== null && currIds.includes(selectedId)) return selectedId;
  if (selectedId !== null && prevIds.includes(selectedId)) {
    const at = prevIds.indexOf(selectedId);
    const after = prevIds.slice(at + 1).find((id) => currIds.includes(id));
    if (after !== undefined) return after;
    const before = prevIds.slice(0, at).reverse().find((id) => currIds.includes(id));
    if (before !== undefined) return before;
    return autoSelectFirst ? currIds[0] ?? null : null;
  }
  return autoSelectFirst ? currIds[0] ?? null : selectedId;
}

/** One line saying why a task is in the queue, so a row never needs opening to be understood. */
export function reasonFor(task: Task, band: Band): string {
  switch (band) {
    case 'review':
      return 'Ready for your review';
    case 'input':
      return 'Waiting for your answer';
    case 'triage': {
      const reasons = [!task.agent ? 'Needs an agent' : 'Assigned to you'];
      if (!task.due_date) reasons.push('no due date');
      return reasons.join(' · ');
    }
    case 'working':
      return task.work_status === 'changes-requested' ? `${task.agent} is revising` : `${task.agent} is working on it`;
  }
}
