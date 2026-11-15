import type { Task } from '../types';

let nextId = 1;

/** A minimal open task; pass only the fields a test cares about. */
export function makeTask(overrides: Partial<Task> = {}): Task {
  const id = overrides.id ?? nextId++;
  return {
    id,
    title: `Task ${id}`,
    status: 'open',
    priority: 'medium',
    tags: [],
    created: '2026-11-01T09:00:00Z',
    notes: [],
    agent: 'Vault',
    due_date: '2026-11-20',
    work_status: 'todo',
    ...overrides,
  };
}
