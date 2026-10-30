import type { Task, AgentProfile, Note, VersionInfo, UpdateStatus, Service, ServiceRun, Thread, ThreadSendKind } from './types';

async function jsonOrThrow<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const body = await response.json().catch(() => ({ error: response.statusText }));
    throw new Error(body.error ?? `Request failed: ${response.status}`);
  }
  return response.json();
}

// fetch() rejects with a bare TypeError when the request never gets a response
// (server down or restarting, network change, page suspended in the background).
// Safari words it "TypeError: Load failed", which says nothing useful, so name the cause.
async function apiFetch(input: string, init?: RequestInit): Promise<Response> {
  try {
    return await fetch(input, init);
  } catch (e) {
    if (e instanceof TypeError) {
      throw new Error(`Can't reach the Command Center server (${e.message})`);
    }
    throw e;
  }
}

// The backend omits `notes` entirely when it's empty; normalize so callers always get a real array.
function normalizeTask(task: Task): Task {
  return { ...task, notes: task.notes ?? [] };
}

export async function fetchTasks(): Promise<Task[]> {
  const tasks = await jsonOrThrow<Task[]>(await apiFetch('/api/tasks'));
  return tasks.map(normalizeTask);
}

export async function fetchTask(id: number): Promise<Task> {
  return normalizeTask(await jsonOrThrow<Task>(await apiFetch(`/api/tasks/${id}`)));
}

export async function fetchServices(): Promise<Service[]> {
  const services = await jsonOrThrow<Service[]>(await apiFetch('/api/services'));
  return services.map((s) => ({ ...s, notes: s.notes ?? [] }));
}

export async function addService(service: {
  title: string; recurrence: string; due?: string; agent?: string; instructions?: string;
}): Promise<{ message: string }> {
  return jsonOrThrow(
    await apiFetch('/api/services', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(service),
    }),
  );
}

export async function setServicePaused(id: number, paused: boolean): Promise<{ message: string }> {
  return jsonOrThrow(await apiFetch(`/api/services/${id}/${paused ? 'pause' : 'resume'}`, { method: 'POST' }));
}

export async function fetchServiceRuns(id: number, limit = 50): Promise<ServiceRun[]> {
  return jsonOrThrow(await apiFetch(`/api/services/${id}/runs?limit=${limit}`));
}

export async function fetchAgents(): Promise<AgentProfile[]> {
  return jsonOrThrow(await apiFetch('/api/agents'));
}

export async function fetchAgentInstructions(name: string): Promise<Note | null> {
  return jsonOrThrow(await apiFetch(`/api/agents/${encodeURIComponent(name)}/instructions`));
}

export async function saveAgentInstructions(name: string, changes: { title?: string; body?: string }): Promise<Note> {
  return jsonOrThrow(
    await apiFetch(`/api/agents/${encodeURIComponent(name)}/instructions`, {
      method: 'PUT',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(changes),
    }),
  );
}

export async function fetchAgentMemory(name: string): Promise<Note | null> {
  return jsonOrThrow(await apiFetch(`/api/agents/${encodeURIComponent(name)}/memory`));
}

export async function saveAgentMemory(name: string, changes: { title?: string; body?: string }): Promise<Note> {
  return jsonOrThrow(
    await apiFetch(`/api/agents/${encodeURIComponent(name)}/memory`, {
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
    await apiFetch('/api/tasks', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(input),
    }),
  );
  return normalizeTask(task);
}

export async function editTask(id: number, changes: Partial<{
  title: string; priority: string; due: string; project: string; tags: string; agent: string; description: string; instructions: string; effort: string; work_status: string; recurrence: string; kind: 'task' | 'service';
}>): Promise<Task> {
  const task = await jsonOrThrow<Task>(
    await apiFetch(`/api/tasks/${id}`, {
      method: 'PATCH',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(changes),
    }),
  );
  return normalizeTask(task);
}

export async function markDone(id: number): Promise<{ completed: Task; spawned: Task | null }> {
  const result = await jsonOrThrow<{ completed: Task; spawned: Task | null }>(
    await apiFetch(`/api/tasks/${id}/done`, { method: 'POST' }),
  );
  return {
    completed: normalizeTask(result.completed),
    spawned: result.spawned ? normalizeTask(result.spawned) : null,
  };
}

export async function reopenTask(id: number): Promise<Task> {
  const task = await jsonOrThrow<Task>(await apiFetch(`/api/tasks/${id}/reopen`, { method: 'POST' }));
  return normalizeTask(task);
}

export async function deleteTask(id: number): Promise<void> {
  const response = await apiFetch(`/api/tasks/${id}`, { method: 'DELETE' });
  if (!response.ok && response.status !== 204) {
    throw new Error(`Failed to delete task ${id}`);
  }
}

export async function fetchTaskNotes(taskId: number): Promise<Note[]> {
  return jsonOrThrow(await apiFetch(`/api/tasks/${taskId}/notes`));
}

export async function createTaskNote(taskId: number, title: string): Promise<Note> {
  return jsonOrThrow(
    await apiFetch(`/api/tasks/${taskId}/notes`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ title }),
    }),
  );
}

export async function saveNote(slug: string, changes: { title?: string; body?: string }): Promise<Note> {
  return jsonOrThrow(
    await apiFetch(`/api/notes/${encodeURIComponent(slug)}`, {
      method: 'PUT',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(changes),
    }),
  );
}

export async function openNote(slug: string): Promise<void> {
  const response = await apiFetch(`/api/notes/${slug}/open`, { method: 'POST' });
  if (!response.ok && response.status !== 204) {
    const body = await response.json().catch(() => ({ error: response.statusText }));
    throw new Error(body.error ?? `Failed to open note ${slug}`);
  }
}

export async function unlinkTaskNote(taskId: number, slug: string): Promise<void> {
  const response = await apiFetch(`/api/tasks/${taskId}/notes/${slug}`, { method: 'DELETE' });
  if (!response.ok && response.status !== 204) {
    throw new Error(`Failed to unlink note ${slug}`);
  }
}

export async function fetchTaskThread(taskId: number): Promise<Thread> {
  return jsonOrThrow(await apiFetch(`/api/tasks/${taskId}/thread`));
}

export async function postTaskThread(
  taskId: number,
  kind: ThreadSendKind | 'approve',
  text: string,
): Promise<Thread> {
  return jsonOrThrow(
    await apiFetch(`/api/tasks/${taskId}/thread`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ kind, text }),
    }),
  );
}

export async function fetchVersion(): Promise<VersionInfo> {
  return jsonOrThrow(await apiFetch('/api/version'));
}

export async function startUpdate(): Promise<void> {
  const response = await apiFetch('/api/update', {
    method: 'POST',
    headers: { 'X-Command-Center': 'update' },
  });
  if (!response.ok) {
    const body = await response.json().catch(() => ({}));
    throw new Error(body.error ?? `Request failed: ${response.status}`);
  }
}

export async function fetchUpdateStatus(): Promise<UpdateStatus> {
  return jsonOrThrow(await apiFetch('/api/update/status'));
}
