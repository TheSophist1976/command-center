import { describe, expect, it } from 'vitest';
import { agentActivity, serviceSummary, statusSummary } from './status';
import { makeTask } from './test/factories';
import type { Service } from './types';

const agents = [{ name: 'CommandCenter', dir: '/a' }, { name: 'Vault', dir: '/b' }, { name: 'Home', dir: '/c' }];

describe('agentActivity', () => {
  it('marks an agent running when it has an in-progress open task', () => {
    const tasks = [
      makeTask({ id: 1, agent: 'CommandCenter', work_status: 'in-progress' }),
      makeTask({ id: 2, agent: 'Vault', work_status: 'todo' }),
    ];
    const result = agentActivity(tasks, agents);
    expect(result.map((a) => [a.name, a.state])).toEqual([['CommandCenter', 'running'], ['Vault', 'idle'], ['Home', 'idle']]);
    expect(result[0].running.map((t) => t.id)).toEqual([1]);
  });

  it('matches agent names case-insensitively', () => {
    const tasks = [makeTask({ agent: 'vault', work_status: 'in-progress' })];
    expect(agentActivity(tasks, agents)[1].state).toBe('running');
  });

  it('ignores done tasks and other statuses', () => {
    const tasks = [
      makeTask({ agent: 'Home', status: 'done', work_status: 'in-progress' }),
      makeTask({ agent: 'Home', work_status: 'waiting-for-review' }),
    ];
    expect(agentActivity(tasks, agents)[2].state).toBe('idle');
  });
});

describe('serviceSummary', () => {
  const svc = (o: Partial<Service>) => ({ ...makeTask(), kind: 'service', run_count: 0, ...o }) as Service;

  it('counts failing, paused and healthy services', () => {
    const s = serviceSummary([
      svc({ id: 1, work_status: 'failing' }),
      svc({ id: 2, work_status: 'failing', paused: true }),
      svc({ id: 3, work_status: 'todo' }),
      svc({ id: 4, work_status: 'todo', paused: true }),
    ]);
    expect(s.failing.map((x) => x.id)).toEqual([1]);
    expect(s.paused).toBe(2);
    expect(s.ok).toBe(1);
  });
});

describe('statusSummary', () => {
  it('combines agents and services for the status strip', () => {
    const activity = agentActivity([makeTask({ agent: 'Vault', work_status: 'in-progress' })], agents);
    const services = serviceSummary([{ ...makeTask(), kind: 'service', run_count: 0, work_status: 'failing' } as Service]);
    expect(statusSummary(activity, services)).toEqual({ running: 1, idle: 2, failing: 1, ok: 0 });
  });
});
