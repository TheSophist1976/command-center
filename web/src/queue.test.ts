import { describe, expect, it } from 'vitest';
import { bandOf, buildQueue, needsYouCount, nextSelection, queueOrder } from './queue';
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
  it('returns every band in order, empty ones included', () => {
    expect(buildQueue([]).map((b) => b.band)).toEqual(['review', 'input', 'triage', 'working']);
  });

  it('sorts a band by due date and sinks deferred tasks to its bottom', () => {
    const a = makeTask({ id: 1, work_status: 'waiting-for-review', due_date: '2026-11-25' });
    const b = makeTask({ id: 2, work_status: 'waiting-for-review', due_date: '2026-11-18' });
    const c = makeTask({ id: 3, work_status: 'waiting-for-review', due_date: '2026-11-10' });
    const review = (deferred: number[]) => buildQueue([a, b, c], new Set(deferred))[0].tasks.map((t) => t.id);
    expect(review([])).toEqual([3, 2, 1]);
    expect(review([3])).toEqual([2, 1, 3]);
  });

  it('keeps a deferred task in its own band rather than moving it', () => {
    const t = makeTask({ id: 9, work_status: 'needs-input' });
    const q = buildQueue([t], new Set([9]));
    expect(q.find((b) => b.band === 'input')?.tasks).toEqual([t]);
  });
});

describe('queueOrder / needsYouCount', () => {
  const tasks = [
    makeTask({ id: 1, work_status: 'in-progress' }),
    makeTask({ id: 2, work_status: 'waiting-for-review' }),
    makeTask({ id: 3, agent: undefined }),
    makeTask({ id: 4, work_status: 'needs-input' }),
  ];

  it('walks the bands top to bottom and skips a collapsed working band', () => {
    const q = buildQueue(tasks);
    expect(queueOrder(q, { workingExpanded: false }).map((t) => t.id)).toEqual([2, 4, 3]);
    expect(queueOrder(q, { workingExpanded: true }).map((t) => t.id)).toEqual([2, 4, 3, 1]);
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
