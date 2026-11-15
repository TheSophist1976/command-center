import type { AgentProfile, Service, Task } from './types';

export interface AgentActivity {
  name: string;
  state: 'running' | 'idle';
  /** Open tasks this agent is working right now. */
  running: Task[];
}

/**
 * An agent is running while it has an open task in progress. This is derived
 * from the tasks themselves, so it needs no separate status source.
 */
export function agentActivity(tasks: Task[], agents: AgentProfile[]): AgentActivity[] {
  return agents.map((a) => {
    const running = tasks
      .filter((t) => t.status === 'open' && t.work_status === 'in-progress' && t.agent?.toLowerCase() === a.name.toLowerCase())
      .sort((x, y) => x.id - y.id);
    return { name: a.name, state: running.length > 0 ? 'running' : 'idle', running };
  });
}

export interface ServiceSummary {
  failing: Service[];
  ok: number;
  paused: number;
}

/** Failing services are the ones to surface; paused ones are deliberate and never count as failing. */
export function serviceSummary(services: Service[]): ServiceSummary {
  const failing = services.filter((s) => s.work_status === 'failing' && !s.paused);
  const paused = services.filter((s) => s.paused).length;
  return { failing, paused, ok: services.length - failing.length - paused };
}

export interface StatusSummary {
  running: number;
  idle: number;
  failing: number;
  ok: number;
}

export function statusSummary(activity: AgentActivity[], services: ServiceSummary): StatusSummary {
  const running = activity.filter((a) => a.state === 'running').length;
  return { running, idle: activity.length - running, failing: services.failing.length, ok: services.ok };
}
