import type { Task, AgentProfile } from './types';

async function jsonOrThrow<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const body = await response.json().catch(() => ({ error: response.statusText }));
    throw new Error(body.error ?? `Request failed: ${response.status}`);
  }
  return response.json();
}

export async function fetchTasks(): Promise<Task[]> {
  return jsonOrThrow(await fetch('/api/tasks'));
}

export async function fetchAgents(): Promise<AgentProfile[]> {
  return jsonOrThrow(await fetch('/api/agents'));
}

export async function addTask(input: {
  title: string;
  priority?: string;
  due?: string;
  project?: string;
  tags?: string;
  agent?: string;
  description?: string;
}): Promise<Task> {
  return jsonOrThrow(
    await fetch('/api/tasks', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(input),
    }),
  );
}

export async function editTask(id: number, changes: Partial<{
  title: string; priority: string; due: string; project: string; tags: string; agent: string; description: string; effort: string;
}>): Promise<Task> {
  return jsonOrThrow(
    await fetch(`/api/tasks/${id}`, {
      method: 'PATCH',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(changes),
    }),
  );
}

export async function markDone(id: number): Promise<{ completed: Task; spawned: Task | null }> {
  return jsonOrThrow(await fetch(`/api/tasks/${id}/done`, { method: 'POST' }));
}

export async function reopenTask(id: number): Promise<Task> {
  return jsonOrThrow(await fetch(`/api/tasks/${id}/reopen`, { method: 'POST' }));
}

export async function deleteTask(id: number): Promise<void> {
  const response = await fetch(`/api/tasks/${id}`, { method: 'DELETE' });
  if (!response.ok && response.status !== 204) {
    throw new Error(`Failed to delete task ${id}`);
  }
}
