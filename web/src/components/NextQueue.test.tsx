import { describe, expect, it, vi } from 'vitest';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { NextQueue } from './NextQueue';
import { buildQueue, reasonFor } from '../queue';
import { makeTask } from '../test/factories';
import { indexProjects } from '../projects';

const today = new Date('2026-11-15T00:00:00');

const tasks = [
  makeTask({ id: 1, title: 'Ship release', work_status: 'waiting-for-review', agent: 'CommandCenter', project: 'command-center' }),
  makeTask({ id: 2, title: 'Pick a calendar', work_status: 'needs-input', agent: 'HabitWise' }),
  makeTask({ id: 3, title: 'Unowned idea', agent: undefined, due_date: undefined }),
  makeTask({ id: 4, title: 'Fix drawer', work_status: 'in-progress', agent: 'Vault' }),
];

function setup(over: Partial<Parameters<typeof NextQueue>[0]> = {}) {
  const props = {
    queue: buildQueue(tasks),
    selectedId: null,
    onSelect: vi.fn(),
    today,
    deferredIds: new Set<number>(),
    workingExpanded: false,
    onToggleWorking: vi.fn(),
    ...over,
  };
  render(<NextQueue {...props} />);
  return props;
}

describe('NextQueue', () => {
  it('groups tasks into labelled bands in priority order', () => {
    setup();
    const labels = screen.getAllByRole('region').map((r) => r.getAttribute('aria-label'));
    expect(labels).toEqual(['Review', 'Needs your input', 'Triage', 'Agents working']);
    expect(within(screen.getByRole('region', { name: 'Review' })).getByText('Ship release')).toBeInTheDocument();
    expect(within(screen.getByRole('region', { name: 'Needs your input' })).getByText('Pick a calendar')).toBeInTheDocument();
    expect(within(screen.getByRole('region', { name: 'Triage' })).getByText('Unowned idea')).toBeInTheDocument();
  });

  it('shows why each task is queued', () => {
    setup();
    expect(screen.getByText('Ready for your review')).toBeInTheDocument();
    expect(screen.getByText('Waiting for your answer')).toBeInTheDocument();
    expect(screen.getByText('Needs an agent · no due date')).toBeInTheDocument();
  });

  it('keeps the Agents working band folded until it is opened', async () => {
    const props = setup();
    expect(screen.queryByText('Fix drawer')).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: /Agents working/ }));
    expect(props.onToggleWorking).toHaveBeenCalled();
  });

  it('lists the working tasks once expanded', () => {
    setup({ workingExpanded: true });
    expect(screen.getByText('Fix drawer')).toBeInTheDocument();
    expect(screen.getByText('Vault is working on it')).toBeInTheDocument();
  });

  it('marks the selected row and reports clicks', async () => {
    const props = setup({ selectedId: 2 });
    expect(screen.getByRole('button', { name: /Pick a calendar/ })).toHaveAttribute('aria-current', 'true');
    expect(screen.getByRole('button', { name: /Ship release/ })).not.toHaveAttribute('aria-current');
    await userEvent.click(screen.getByRole('button', { name: /Ship release/ }));
    expect(props.onSelect).toHaveBeenCalledWith(tasks[0]);
  });

  it('flags deferred tasks', () => {
    setup({ deferredIds: new Set([2]) });
    expect(within(screen.getByRole('button', { name: /Pick a calendar/ })).getByText('deferred')).toBeInTheDocument();
  });

  it('says so when nothing needs the human, even if agents are busy', () => {
    setup({ queue: buildQueue([tasks[3]]) });
    expect(screen.getByText('Nothing needs you.')).toBeInTheDocument();
  });
});

describe('reasonFor', () => {
  it('distinguishes unassigned from human-assigned triage', () => {
    expect(reasonFor(makeTask({ agent: 'human' }), 'triage')).toBe('Assigned to you');
    expect(reasonFor(makeTask({ agent: undefined }), 'triage')).toBe('Needs an agent');
  });

  it('describes agents revising after a change request', () => {
    expect(reasonFor(makeTask({ agent: 'Vault', work_status: 'changes-requested' }), 'working')).toBe('Vault is revising');
  });

  it('shows a project color dot on the row chip when the project has a color', () => {
    setup({ projectMeta: indexProjects([{ name: 'Command-Center', status: 'active', color: 'blue' }]) });
    const row = screen.getByText('Ship release').closest('button')!;
    expect(within(row).getByTestId('project-dot')).toBeInTheDocument();
    expect(within(screen.getByText('Unowned idea').closest('button')!).queryByTestId('project-dot')).toBeNull();
  });
});
