import type { Task, AgentProfile, Note, VersionInfo, UpdateStatus } from './types';

async function jsonOrThrow<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const body = await response.json().catch(() => ({ error: response.statusText }));
    throw new Error(body.error ?? `Request failed: ${response.status}`);
  }
  return response.json();
}

// The backend omits `notes` entirely when it's empty; normalize so callers always get a real array.
function normalizeTask(task: Task): Task {
  return { ...task, notes: task.notes ?? [] };
}

export async function fetchTasks(): Promise<Task[]> {
  const tasks = await jsonOrThrow<Task[]>(await fetch('/api/tasks'));
  return tasks.map(normalizeTask);
}

export async function fetchAgents(): Promise<AgentProfile[]> {
  return jsonOrThrow(await fetch('/api/agents'));
}

export async function fetchAgentInstructions(name: string): Promise<Note | null> {
  return jsonOrThrow(await fetch(`/api/agents/${encodeURIComponent(name)}/instructions`));
}

export async function saveAgentInstructions(name: string, changes: { title?: string; body?: string }): Promise<Note> {
  return jsonOrThrow(
    await fetch(`/api/agents/${encodeURIComponent(name)}/instructions`, {
      method: 'PUT',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(changes),
    }),
  );
}

export async function fetchAgentMemory(name: string): Promise<Note | null> {
  return jsonOrThrow(await fetch(`/api/agents/${encodeURIComponent(name)}/memory`));
}

export async function saveAgentMemory(name: string, changes: { title?: string; body?: string }): Promise<Note> {
  return jsonOrThrow(
    await fetch(`/api/agents/${encodeURIComponent(name)}/memory`, {
      method: 'PUT',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(changes),
    }),
  );
}

export async function addTask(input: {
  title: string;
  priority?: string;
  due?: string;
  project?: string;
  tags?: string;
  agent?: string;
  description?: string;
  instructions?: string;
}): Promise<Task> {
  const task = await jsonOrThrow<Task>(
    await fetch('/api/tasks', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(input),
    }),
  );
  return normalizeTask(task);
}

export async function editTask(id: number, changes: Partial<{
  title: string; priority: string; due: string; project: string; tags: string; agent: string; description: string; instructions: string; effort: string; work_status: string; recurrence: string;
}>): Promise<Task> {
  const task = await jsonOrThrow<Task>(
    await fetch(`/api/tasks/${id}`, {
      method: 'PATCH',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(changes),
    }),
  );
  return normalizeTask(task);
}

export async function markDone(id: number): Promise<{ completed: Task; spawned: Task | null }> {
  const result = await jsonOrThrow<{ completed: Task; spawned: Task | null }>(
    await fetch(`/api/tasks/${id}/done`, { method: 'POST' }),
  );
  return {
    completed: normalizeTask(result.completed),
    spawned: result.spawned ? normalizeTask(result.spawned) : null,
  };
}

export async function reopenTask(id: number): Promise<Task> {
  const task = await jsonOrThrow<Task>(await fetch(`/api/tasks/${id}/reopen`, { method: 'POST' }));
  return normalizeTask(task);
}

export async function deleteTask(id: number): Promise<void> {
  const response = await fetch(`/api/tasks/${id}`, { method: 'DELETE' });
  if (!response.ok && response.status !== 204) {
    throw new Error(`Failed to delete task ${id}`);
  }
}

export async function fetchTaskNotes(taskId: number): Promise<Note[]> {
  return jsonOrThrow(await fetch(`/api/tasks/${taskId}/notes`));
}

export async function createTaskNote(taskId: number, title: string): Promise<Note> {
  return jsonOrThrow(
    await fetch(`/api/tasks/${taskId}/notes`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ title }),
    }),
  );
}

export async function openNote(slug: string): Promise<void> {
  const response = await fetch(`/api/notes/${slug}/open`, { method: 'POST' });
  if (!response.ok && response.status !== 204) {
    const body = await response.json().catch(() => ({ error: response.statusText }));
    throw new Error(body.error ?? `Failed to open note ${slug}`);
  }
}

export async function unlinkTaskNote(taskId: number, slug: string): Promise<void> {
  const response = await fetch(`/api/tasks/${taskId}/notes/${slug}`, { method: 'DELETE' });
  if (!response.ok && response.status !== 204) {
    throw new Error(`Failed to unlink note ${slug}`);
  }
}

export async function fetchTaskReview(taskId: number): Promise<Note | null> {
  return jsonOrThrow(await fetch(`/api/tasks/${taskId}/review`));
}

export async function postTaskFeedback(taskId: number, text: string): Promise<Note> {
  return jsonOrThrow(
    await fetch(`/api/tasks/${taskId}/review`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ text }),
    }),
  );
}

export async function postTaskAnswer(taskId: number, text: string): Promise<Note> {
  return jsonOrThrow(
    await fetch(`/api/tasks/${taskId}/question`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ text }),
    }),
  );
}

export async function fetchTaskQuestion(taskId: number): Promise<Note | null> {
  return jsonOrThrow(await fetch(`/api/tasks/${taskId}/question`));
}

export async function fetchVersion(): Promise<VersionInfo> {
  return jsonOrThrow(await fetch('/api/version'));
}

export async function startUpdate(): Promise<void> {
  const response = await fetch('/api/update', {
    method: 'POST',
    headers: { 'X-Command-Center': 'update' },
  });
  if (!response.ok) {
    const body = await response.json().catch(() => ({}));
    throw new Error(body.error ?? `Request failed: ${response.status}`);
  }
}

export async function fetchUpdateStatus(): Promise<UpdateStatus> {
  return jsonOrThrow(await fetch('/api/update/status'));
}
