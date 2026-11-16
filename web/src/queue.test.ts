import { describe, expect, it } from 'vitest';
import { bandOf, buildQueue, isFocusable, needsYouCount, nextSelection, queueOrder, queueSize } from './queue';
import { byDeferredThenDue } from './filters';
import { needsHumanAction } from './filters';
import { makeTask } from './test/factories';

describe('bandOf', () => {
  it('puts a task waiting for review in Review, even when it is unassigned', () => {
    expect(bandOf(makeTask({ work_status: 'waiting-for-review' }))).toBe('review');
    expect(bandOf(makeTask({ work_status: 'waiting-for-review', agent: undefined }))).toBe('review');
  });

  it('puts a task blocked on input in Needs your input', () => {
    expect(bandOf(makeTask({ work_status: 'needs-input' }))).toBe('input');
  });

  it('puts unassigned and human-assigned tasks in Triage', () => {
    expect(bandOf(makeTask({ agent: undefined }))).toBe('triage');
    expect(bandOf(makeTask({ agent: 'human' }))).toBe('triage');
    expect(bandOf(makeTask({ agent: 'Human' }))).toBe('triage');
  });

  it('puts agent-owned work in progress under Agents working', () => {
    expect(bandOf(makeTask({ work_status: 'in-progress' }))).toBe('working');
    expect(bandOf(makeTask({ work_status: 'changes-requested' }))).toBe('working');
  });

  it('leaves queued, finished and done tasks out of Next', () => {
    expect(bandOf(makeTask({ work_status: 'todo' }))).toBeNull();
    expect(bandOf(makeTask({ work_status: 'complete' }))).toBeNull();
    expect(bandOf(makeTask({ status: 'done', work_status: 'waiting-for-review' }))).toBeNull();
  });

  it('keeps review + input + triage identical to needsHumanAction', () => {
    const statuses = [undefined, 'todo', 'in-progress', 'waiting-for-review', 'changes-requested', 'needs-input', 'complete'] as const;
    for (const agent of [undefined, 'human', 'Vault']) {
      for (const work_status of statuses) {
        for (const status of ['open', 'done'] as const) {
          const t = makeTask({ agent, work_status, status });
          const band = bandOf(t);
          expect(band !== null && band !== 'working').toBe(needsHumanAction(t));
        }
      }
    }
  });
});

describe('buildQueue', () => {
  it('returns the Needs you list and the Agents working list, in that order', () => {
    expect(buildQueue([]).map((s) => s.section)).toEqual(['needs', 'working']);
  });

  it('puts review, input and triage tasks together in Needs you, and agent work in Agents working', () => {
    const q = buildQueue([
      makeTask({ id: 1, work_status: 'waiting-for-review' }),
      makeTask({ id: 2, work_status: 'needs-input' }),
      makeTask({ id: 3, agent: undefined }),
      makeTask({ id: 4, work_status: 'in-progress' }),
      makeTask({ id: 5, work_status: 'changes-requested' }),
      makeTask({ id: 6, work_status: 'todo' }),
    ]);
    expect(q[0].tasks.map((t) => t.id).sort()).toEqual([1, 2, 3]);
    expect(q[1].tasks.map((t) => t.id).sort()).toEqual([4, 5]);
  });
});

// The order of the Next queue is a product rule, not an implementation detail: it was the single list
// the web UI always showed, and moving to sections once silently reordered it. These pin it.
describe('Next queue order', () => {
  const ids = (tasks: ReturnType<typeof makeTask>[], deferred: number[] = []) =>
    buildQueue(tasks, new Set(deferred))[0].tasks.map((t) => t.id);

  it('puts tasks missing an agent or a due date before everything else', () => {
    const order = ids([
      makeTask({ id: 1, work_status: 'waiting-for-review', due_date: '2026-11-01' }),
      makeTask({ id: 2, work_status: 'needs-input', due_date: undefined }),
      makeTask({ id: 3, agent: undefined, due_date: '2026-11-02' }),
    ]);
    expect(order.slice(0, 2).sort()).toEqual([2, 3]);
    expect(order[2]).toBe(1);
  });

  it('orders those by priority, then the oldest first', () => {
    const order = ids([
      makeTask({ id: 1, agent: undefined, due_date: undefined, priority: 'low' }),
      makeTask({ id: 2, agent: undefined, due_date: undefined, priority: 'critical' }),
      makeTask({ id: 3, agent: undefined, due_date: undefined, priority: 'low', created: '2026-10-01T09:00:00Z' }),
    ]);
    expect(order).toEqual([2, 3, 1]);
  });

  it('orders the rest by due date, then priority, whatever kind of task each is', () => {
    const order = ids([
      makeTask({ id: 1, work_status: 'waiting-for-review', due_date: '2026-11-25', priority: 'low' }),
      makeTask({ id: 2, work_status: 'needs-input', due_date: '2026-11-11', priority: 'low' }),
      makeTask({ id: 3, agent: 'human', due_date: '2026-11-16', priority: 'low' }),
      makeTask({ id: 4, work_status: 'waiting-for-review', due_date: '2026-11-16', priority: 'critical' }),
    ]);
    // 3 is assigned to the human, so it has an agent and a date like the others.
    expect(order).toEqual([2, 4, 3, 1]);
  });

  it('lets urgency decide across kinds: an overdue question beats a later review', () => {
    const order = ids([
      makeTask({ id: 1, work_status: 'waiting-for-review', due_date: '2026-11-25' }),
      makeTask({ id: 2, work_status: 'needs-input', due_date: '2026-11-11' }),
    ]);
    expect(order).toEqual([2, 1]);
  });

  it('sinks deferred tasks below everything else, even across kinds', () => {
    const order = ids(
      [
        makeTask({ id: 1, work_status: 'waiting-for-review', due_date: '2026-11-10' }),
        makeTask({ id: 2, work_status: 'needs-input', due_date: '2026-11-20' }),
        makeTask({ id: 3, agent: undefined, due_date: undefined }),
      ],
      [1, 3],
    );
    expect(order[0]).toBe(2);
    expect(order.slice(1).sort()).toEqual([1, 3]);
  });

  it('orders the deferred ones by the same rules among themselves', () => {
    const order = ids(
      [
        makeTask({ id: 1, work_status: 'waiting-for-review', due_date: '2026-11-25' }),
        makeTask({ id: 2, agent: undefined, due_date: undefined }),
        makeTask({ id: 3, work_status: 'needs-input', due_date: '2026-11-12' }),
      ],
      [1, 2, 3],
    );
    expect(order).toEqual([2, 3, 1]);
  });

  it('breaks remaining ties with the oldest task first', () => {
    const order = ids([
      makeTask({ id: 1, work_status: 'waiting-for-review', due_date: '2026-11-20', created: '2026-11-05T09:00:00Z' }),
      makeTask({ id: 2, work_status: 'needs-input', due_date: '2026-11-20', created: '2026-11-01T09:00:00Z' }),
    ]);
    expect(order).toEqual([2, 1]);
  });

  it('orders Agents working by the same rules', () => {
    const q = buildQueue([
      makeTask({ id: 1, work_status: 'in-progress', due_date: '2026-11-25' }),
      makeTask({ id: 2, work_status: 'in-progress', due_date: '2026-11-12' }),
    ]);
    expect(q[1].tasks.map((t) => t.id)).toEqual([2, 1]);
  });

  it('is the order the old single list produced, task for task', () => {
    const tasks = [
      makeTask({ id: 1, work_status: 'waiting-for-review', due_date: '2026-11-25', priority: 'low' }),
      makeTask({ id: 2, work_status: 'needs-input', due_date: '2026-11-11', priority: 'critical' }),
      makeTask({ id: 3, agent: undefined, due_date: undefined, priority: 'medium' }),
      makeTask({ id: 4, work_status: 'waiting-for-review', due_date: '2026-11-16', priority: 'high' }),
      makeTask({ id: 5, work_status: 'waiting-for-review', due_date: undefined, priority: 'medium' }),
      makeTask({ id: 6, work_status: 'needs-input', due_date: '2026-11-17', priority: 'medium' }),
      makeTask({ id: 7, agent: 'human', due_date: '2026-11-16', priority: 'high' }),
    ];
    const deferred = new Set([4]);
    const old = [...tasks].sort(byDeferredThenDue(deferred)).map((t) => t.id);
    expect(buildQueue(tasks, deferred)[0].tasks.map((t) => t.id)).toEqual(old);
    expect(old).toEqual([3, 5, 2, 7, 6, 1, 4]);
  });
});

describe('queueOrder / needsYouCount', () => {
  const tasks = [
    makeTask({ id: 1, work_status: 'in-progress' }),
    makeTask({ id: 2, work_status: 'waiting-for-review', due_date: '2026-11-18' }),
    makeTask({ id: 3, agent: undefined, due_date: undefined }),
    makeTask({ id: 4, work_status: 'needs-input', due_date: '2026-11-20' }),
  ];

  it('lists Needs you in order and skips a collapsed Agents working', () => {
    const q = buildQueue(tasks);
    expect(queueOrder(q, { workingExpanded: false }).map((t) => t.id)).toEqual([3, 2, 4]);
    expect(queueOrder(q, { workingExpanded: true }).map((t) => t.id)).toEqual([3, 2, 4, 1]);
  });

  it('counts only tasks that need the human', () => {
    expect(needsYouCount(tasks)).toBe(3);
  });
});

describe('nextSelection', () => {
  it('keeps the selection while it is still queued', () => {
    expect(nextSelection([1, 2, 3], [1, 2, 3], 2, true)).toBe(2);
  });

  it('moves to the task that followed when the selection leaves the queue', () => {
    expect(nextSelection([1, 2, 3], [1, 3], 2, true)).toBe(3);
  });

  it('moves to the previous task when the last one leaves', () => {
    expect(nextSelection([1, 2, 3], [1, 2], 3, true)).toBe(2);
  });

  it('skips over other tasks that left at the same time', () => {
    expect(nextSelection([1, 2, 3, 4], [1, 4], 2, true)).toBe(4);
  });

  it('selects nothing once the queue is empty', () => {
    expect(nextSelection([1], [], 1, true)).toBeNull();
  });

  it('selects the first task when nothing was selected, if asked to', () => {
    expect(nextSelection([], [5, 6], null, true)).toBe(5);
    expect(nextSelection([], [5, 6], null, false)).toBeNull();
  });

  it('replaces a selection that was never in the queue with the first task, if asked to', () => {
    expect(nextSelection([], [5, 6], 99, true)).toBe(5);
    expect(nextSelection([], [5, 6], 99, false)).toBe(99);
  });
});

describe('Focus mode', () => {
  const today = new Date('2026-11-16T09:30:00');
  const none = new Set<number>();

  it('keeps what is waiting on the human', () => {
    expect(isFocusable(makeTask({ work_status: 'waiting-for-review', due_date: '2026-11-16' }), none, today)).toBe(true);
    expect(isFocusable(makeTask({ work_status: 'needs-input', due_date: '2026-11-16' }), none, today)).toBe(true);
    expect(isFocusable(makeTask({ agent: undefined, due_date: '2026-11-16' }), none, today)).toBe(true);
  });

  it('hides the agents’ own work', () => {
    expect(isFocusable(makeTask({ work_status: 'in-progress', due_date: '2026-11-16' }), none, today)).toBe(false);
    expect(isFocusable(makeTask({ work_status: 'changes-requested' }), none, today)).toBe(false);
  });

  it('hides deferred tasks', () => {
    const t = makeTask({ id: 9, work_status: 'needs-input', due_date: undefined });
    expect(isFocusable(t, new Set([9]), today)).toBe(false);
    expect(isFocusable(t, none, today)).toBe(true);
  });

  it('keeps overdue, due-today and undated tasks and hides later ones', () => {
    const review = (due_date?: string) => makeTask({ work_status: 'waiting-for-review', due_date });
    expect(isFocusable(review('2026-11-10'), none, today)).toBe(true);
    expect(isFocusable(review('2026-11-16'), none, today)).toBe(true);
    expect(isFocusable(review(undefined), none, today)).toBe(true);
    expect(isFocusable(review('2026-11-17'), none, today)).toBe(false);
    expect(isFocusable(review('2027-01-01'), none, today)).toBe(false);
  });

  it('treats the whole of today as today, whatever the time', () => {
    const t = makeTask({ work_status: 'needs-input', due_date: '2026-11-16' });
    expect(isFocusable(t, none, new Date('2026-11-16T23:59:00'))).toBe(true);
    expect(isFocusable(t, none, new Date('2026-11-16T00:00:00'))).toBe(true);
  });

  it('ignores done tasks', () => {
    expect(isFocusable(makeTask({ status: 'done', work_status: 'waiting-for-review' }), none, today)).toBe(false);
  });

  const tasks = [
    makeTask({ id: 1, work_status: 'waiting-for-review', due_date: '2026-11-16' }),
    makeTask({ id: 2, work_status: 'needs-input', due_date: '2026-12-01' }),
    makeTask({ id: 3, agent: undefined, due_date: undefined }),
    makeTask({ id: 4, work_status: 'in-progress' }),
    makeTask({ id: 5, work_status: 'waiting-for-review', due_date: '2026-11-10' }),
  ];

  it('builds a queue of only the focusable tasks, still in order', () => {
    const q = buildQueue(tasks, new Set([5]), { focus: true, today });
    expect(q.map((s) => [s.section, s.tasks.map((t) => t.id)])).toEqual([
      ['needs', [3, 1]], ['working', []],
    ]);
  });

  it('leaves the queue untouched when Focus is off', () => {
    const q = buildQueue(tasks, new Set([5]), { today });
    expect(queueSize(q)).toBe(5);
  });

  it('counts what a queue lists', () => {
    expect(queueSize(buildQueue(tasks, none, { focus: true, today }))).toBe(3);
  });
});
