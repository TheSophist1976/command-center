import type { Task } from './types';

export function countAllOpen(tasks: Task[]): number {
  return tasks.filter((t) => t.status !== 'done').length;
}
