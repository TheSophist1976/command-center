import { vi } from 'vitest';
import type { AgentProfile, Service, Task, Thread } from '../types';

interface FakeApiOptions {
  tasks: Task[];
  agents?: AgentProfile[];
  services?: Service[];
}

export interface FakeApi {
  tasks: Task[];
  /** Every request made, as "METHOD /path", in order. */
  calls: string[];
  /** Bodies posted to a task's thread, in order. */
  posts: { id: number; kind: string; text: string }[];
}

const json = (body: unknown, status = 200) =>
  new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });

/** What the real server derives from the lifecycle: which composer and buttons a task's conversation offers. */
function threadFor(task: Task): Thread {
  const review = task.work_status === 'waiting-for-review';
  const input = task.work_status === 'needs-input';
  return {
    note: review || input
      ? { slug: `task-${task.id}-thread`, title: 'Thread', body: `# Thread\n\n## Agent — today\n\nAgent message for ${task.title}.` }
      : null,
    allowed_action: { send: review ? 'feedback' : input ? 'answer' : null, approve: review },
    unread: false,
  };
}

/**
 * Stands in for the Command Center server: `fetch` answers the /api routes the app uses, and
 * actions mutate the in-memory tasks the way the real server would. Also stubs EventSource,
 * which jsdom lacks.
 */
export function installFakeApi({ tasks, agents = [], services = [] }: FakeApiOptions): FakeApi {
  const api: FakeApi = { tasks: structuredClone(tasks), calls: [], posts: [] };
  const find = (id: number) => api.tasks.find((t) => t.id === id)!;

  vi.stubGlobal('EventSource', class {
    onopen: (() => void) | null = null;
    onmessage: (() => void) | null = null;
    close() {}
  });

  vi.stubGlobal('fetch', vi.fn(async (input: string, init?: RequestInit) => {
    const method = init?.method ?? 'GET';
    api.calls.push(`${method} ${input}`);
    let m: RegExpMatchArray | null;

    if (input === '/api/tasks' && method === 'GET') return json(api.tasks);
    if (input === '/api/agents') return json(agents);
    if (input === '/api/services') return json(services);
    if (input === '/api/projects' || input === '/api/tags' || input.startsWith('/api/projects?')) return json([]);
    if (input === '/api/version') return json({ current: '5.4.2', latest: null, update_available: false, update_supported: false });
    if ((m = input.match(/^\/api\/tasks\/(\d+)\/notes$/))) return json([]);
    if ((m = input.match(/^\/api\/tasks\/(\d+)\/thread$/))) {
      const task = find(Number(m[1]));
      if (method === 'POST') {
        const { kind, text } = JSON.parse(String(init?.body));
        api.posts.push({ id: task.id, kind, text });
        if (kind === 'approve') { task.status = 'done'; task.work_status = 'complete'; }
        else if (kind === 'answer') task.work_status = 'in-progress';
        else if (kind === 'feedback') task.work_status = 'changes-requested';
      }
      return json(threadFor(task));
    }
    if ((m = input.match(/^\/api\/tasks\/(\d+)\/done$/)) && method === 'POST') {
      const task = find(Number(m[1]));
      task.status = 'done';
      task.work_status = 'complete';
      return json({ completed: task, spawned: null });
    }
    if ((m = input.match(/^\/api\/tasks\/(\d+)$/))) {
      const task = find(Number(m[1]));
      if (method === 'PATCH') Object.assign(task, JSON.parse(String(init?.body)));
      return json(task);
    }
    return json({ error: `unhandled ${method} ${input}` }, 404);
  }));

  return api;
}
