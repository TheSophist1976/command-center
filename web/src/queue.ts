import type { Task } from './types';
import { byDeferredThenDue, needsHumanAction } from './filters';

/**
 * The Next view is one queue split into bands, most pressing first:
 * `review` (an agent finished and wants approval), `input` (an agent asked a
 * question), `triage` (unassigned or assigned to the human), and `working`
 * (an agent is on it; shown for awareness only).
 */
export type Band = 'review' | 'input' | 'triage' | 'working';

export const BAND_ORDER: Band[] = ['review', 'input', 'triage', 'working'];

export const BAND_LABEL: Record<Band, string> = {
  review: 'Review',
  input: 'Needs your input',
  triage: 'Triage',
  working: 'Agents working',
};

export interface QueueBand {
  band: Band;
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

/** Every band in order (empty ones included), each sorted with deferred tasks sinking to its bottom. */
export function buildQueue(tasks: Task[], deferred: ReadonlySet<number> = new Set()): QueueBand[] {
  const compare = byDeferredThenDue(deferred);
  return BAND_ORDER.map((band) => ({
    band,
    tasks: tasks.filter((t) => bandOf(t) === band).sort(compare),
  }));
}

/** The tasks in keyboard order. A collapsed working band is skipped because its rows are not on screen. */
export function queueOrder(queue: QueueBand[], opts: { workingExpanded: boolean }): Task[] {
  return queue.filter((b) => b.band !== 'working' || opts.workingExpanded).flatMap((b) => b.tasks);
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
