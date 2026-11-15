import { beforeEach, describe, expect, it } from 'vitest';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import App from './App';
import { installFakeApi, type FakeApi } from './test/fakeApi';
import { setViewport } from './test/setup';
import { makeTask } from './test/factories';

const agents = ['CommandCenter', 'Vault', 'HabitWise', 'Home'].map((name) => ({ name, dir: `/${name}` }));

const tasks = [
  makeTask({ id: 1, title: 'Ship release', agent: 'CommandCenter', work_status: 'waiting-for-review', due_date: '2030-01-01' }),
  makeTask({ id: 2, title: 'Reorganise notes', agent: 'Vault', work_status: 'waiting-for-review', due_date: '2030-01-02' }),
  makeTask({ id: 3, title: 'Pick a calendar', agent: 'HabitWise', work_status: 'needs-input', due_date: '2030-01-03' }),
  makeTask({ id: 4, title: 'Unowned idea', agent: undefined, due_date: undefined }),
  makeTask({ id: 5, title: 'Fix drawer', agent: 'Vault', work_status: 'in-progress', due_date: '2030-01-05' }),
  makeTask({ id: 6, title: 'Water plants', agent: 'Home', work_status: 'todo', due_date: '2030-01-06', recurrence: 'weekly:sun' }),
];

let api: FakeApi;

async function renderApp() {
  render(<App />);
  await screen.findByRole('region', { name: 'Review' });
}

const docked = () => screen.getByRole('region', { name: 'Selected task' });
const queueRow = (title: string) => screen.getByRole('button', { name: new RegExp(title) });
/** Titles of the rows in the All tasks table (the inspector may also show the selected task). */
const tableTitles = () => Array.from(document.querySelectorAll('[data-task-row]')).map((el) => el.textContent ?? '');
const hasRow = (title: string) => tableTitles().some((t) => t.includes(title));

beforeEach(() => {
  setViewport(false);
  api = installFakeApi({ tasks, agents });
});

describe('desktop: Next', () => {
  it('opens on the queue, in bands, with no sidebar or Inbox', async () => {
    await renderApp();
    expect(screen.getByRole('tab', { name: /Next/ })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getAllByRole('region').map((r) => r.getAttribute('aria-label')).filter((l) => l !== 'Selected task'))
      .toEqual(['Review', 'Needs your input', 'Triage', 'Agents working']);
    expect(screen.queryByText('Inbox')).not.toBeInTheDocument();
    expect(screen.queryByText('Due window')).not.toBeInTheDocument();
  });

  it('counts only the tasks that need you on the Next tab', async () => {
    await renderApp();
    expect(within(screen.getByRole('tab', { name: /Next/ })).getByLabelText('4 tasks')).toBeInTheDocument();
    expect(within(screen.getByRole('tab', { name: /All tasks/ })).getByLabelText('6 tasks')).toBeInTheDocument();
  });

  it('docks the first task with its conversation, no click needed', async () => {
    await renderApp();
    await waitFor(() => expect(within(docked()).getByText('Ship release')).toBeInTheDocument());
    expect(await within(docked()).findByText('Agent message for Ship release.')).toBeInTheDocument();
    expect(within(docked()).getByRole('button', { name: /Request changes/ })).toBeInTheDocument();
    expect(within(docked()).getByRole('button', { name: /^Approve/ })).toBeInTheDocument();
  });

  it('folds the task details away for review items', async () => {
    await renderApp();
    await within(docked()).findByText('Agent message for Ship release.');
    expect(docked().querySelector('details')).not.toHaveAttribute('open');
  });

  it('unfolds the task details for triage items so they can be fixed in place', async () => {
    await renderApp();
    await userEvent.click(queueRow('Unowned idea'));
    await waitFor(() => expect(within(docked()).getByText('Unowned idea')).toBeInTheDocument());
    expect(docked().querySelector('details')).toHaveAttribute('open');
  });

  it('shows the selected row in the queue and follows j / k', async () => {
    await renderApp();
    await waitFor(() => expect(queueRow('Ship release')).toHaveAttribute('aria-current', 'true'));
    await userEvent.keyboard('j');
    await waitFor(() => expect(queueRow('Reorganise notes')).toHaveAttribute('aria-current', 'true'));
    await userEvent.keyboard('k');
    await waitFor(() => expect(queueRow('Ship release')).toHaveAttribute('aria-current', 'true'));
  });

  it('approving posts the approval and moves on to the next item', async () => {
    await renderApp();
    await userEvent.click(await within(docked()).findByRole('button', { name: /^Approve/ }));
    await waitFor(() => expect(api.posts).toEqual([{ id: 1, kind: 'approve', text: '' }]));
    await waitFor(() => expect(within(docked()).getByText('Reorganise notes')).toBeInTheDocument());
    expect(screen.queryByRole('button', { name: /Ship release/ })).not.toBeInTheDocument();
    expect(queueRow('Reorganise notes')).toHaveAttribute('aria-current', 'true');
  });

  it('requesting changes sends it back to the agent and moves on', async () => {
    await renderApp();
    const box = await within(docked()).findByPlaceholderText(/Request changes/);
    await userEvent.type(box, 'Please add a test{Enter}');
    await waitFor(() => expect(api.posts).toEqual([{ id: 1, kind: 'feedback', text: 'Please add a test' }]));
    await waitFor(() => expect(within(docked()).getByText('Reorganise notes')).toBeInTheDocument());
    // The task is with the agent now, so it sits in the folded Agents working band rather than the queue.
    expect(screen.queryByRole('button', { name: /Ship release/ })).not.toBeInTheDocument();
  });

  it('answering an agent hands the task back and moves on', async () => {
    await renderApp();
    await userEvent.click(queueRow('Pick a calendar'));
    const box = await within(docked()).findByPlaceholderText(/Answer the agent/);
    await userEvent.type(box, 'Personal{Enter}');
    await waitFor(() => expect(api.posts).toEqual([{ id: 3, kind: 'answer', text: 'Personal' }]));
    await waitFor(() => expect(within(docked()).getByText('Unowned idea')).toBeInTheDocument());
  });

  it('Enter marks the selected task done and moves on', async () => {
    await renderApp();
    await waitFor(() => expect(within(docked()).getByText('Ship release')).toBeInTheDocument());
    await userEvent.keyboard('{Enter}');
    await waitFor(() => expect(api.calls).toContain('POST /api/tasks/1/done'));
    await waitFor(() => expect(within(docked()).getByText('Reorganise notes')).toBeInTheDocument());
  });

  it('z defers the selected task to the bottom of its band', async () => {
    await renderApp();
    await waitFor(() => expect(within(docked()).getByText('Ship release')).toBeInTheDocument());
    await userEvent.keyboard('z');
    await waitFor(() => expect(within(docked()).getByText('Reorganise notes')).toBeInTheDocument());
    const review = within(screen.getByRole('region', { name: 'Review' })).getAllByRole('button').map((b) => b.textContent);
    expect(review[0]).toContain('Reorganise notes');
    expect(review[1]).toContain('Ship release');
    expect(review[1]).toContain('deferred');
  });

  it('keeps Agents working folded, and unfolds it on request', async () => {
    await renderApp();
    expect(screen.queryByText('Fix drawer')).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: /Agents working/ }));
    expect(screen.getByText('Fix drawer')).toBeInTheDocument();
  });

  it('shows who is running from the tasks themselves', async () => {
    await renderApp();
    expect(await screen.findByText('1 running · 3 idle')).toBeInTheDocument();
  });

  it('opens an agent for editing from the status strip', async () => {
    await renderApp();
    await userEvent.click(screen.getByRole('button', { name: 'Agent and service status' }));
    expect(screen.getByText('#5 Fix drawer')).toBeInTheDocument();
  });
});

describe('desktop: All tasks', () => {
  it('is one toggle away, with filters and the full table', async () => {
    await renderApp();
    await userEvent.click(screen.getByRole('tab', { name: /All tasks/ }));
    expect(screen.getByRole('tab', { name: /All tasks/ })).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('group', { name: 'Filter tasks' })).toBeInTheDocument();
    for (const title of ['Ship release', 'Unowned idea', 'Water plants']) expect(hasRow(title)).toBe(true);
    expect(screen.getAllByText('Effort').length).toBeGreaterThan(0);
    expect(screen.queryByRole('region', { name: 'Review' })).not.toBeInTheDocument();
  });

  it('keeps recurring tasks reachable through a Recurring filter', async () => {
    await renderApp();
    await userEvent.click(screen.getByRole('tab', { name: /All tasks/ }));
    await userEvent.click(screen.getByRole('button', { name: /^Recurring/ }));
    expect(tableTitles()).toHaveLength(1);
    expect(hasRow('Water plants')).toBe(true);
  });

  it('groups by agent by default and can regroup', async () => {
    await renderApp();
    await userEvent.click(screen.getByRole('tab', { name: /All tasks/ }));
    expect(screen.getByText('unassigned')).toBeInTheDocument();
    await userEvent.selectOptions(screen.getByLabelText('Group'), 'none');
    expect(screen.queryByText('unassigned')).not.toBeInTheDocument();
  });

  it('remembers the chosen view across reloads', async () => {
    await renderApp();
    await userEvent.click(screen.getByRole('tab', { name: /All tasks/ }));
    document.body.innerHTML = '';
    render(<App />);
    await waitFor(() => expect(screen.getByRole('tab', { name: /All tasks/ })).toHaveAttribute('aria-selected', 'true'));
  });

  it('finds a task outside the queue from ⌘K by switching to All tasks', async () => {
    await renderApp();
    await userEvent.keyboard('{Control>}k{/Control}');
    await userEvent.type(await screen.findByPlaceholderText(/Search tasks or type a command/), 'Water');
    await userEvent.click(await screen.findByText('Water plants'));
    await waitFor(() => expect(screen.getByRole('tab', { name: /All tasks/ })).toHaveAttribute('aria-selected', 'true'));
  });
});

describe('mobile', () => {
  beforeEach(() => setViewport(true));

  async function renderMobile() {
    render(<App />);
    await screen.findByRole('region', { name: 'Review' });
  }
  const detail = () => document.querySelector('.mobile-detail') as HTMLElement;

  it('shows the status bar and the Next | All tasks toggle over a stacked queue', async () => {
    await renderMobile();
    expect(screen.getByRole('button', { name: 'Agent and service status' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /Next/ })).toHaveAttribute('aria-selected', 'true');
    expect(screen.queryByRole('region', { name: 'Selected task' })).not.toBeInTheDocument();
    expect(detail()).toBeNull();
  });

  it('opens a task full screen and steps through the queue', async () => {
    await renderMobile();
    await userEvent.click(queueRow('Ship release'));
    expect(within(detail()).getByText('1 of 4')).toBeInTheDocument();
    expect(within(detail()).getByText('Ship release')).toBeInTheDocument();
    await userEvent.click(within(detail()).getByRole('button', { name: 'Next task' }));
    expect(within(detail()).getByText('2 of 4')).toBeInTheDocument();
    expect(within(detail()).getByText('Reorganise notes')).toBeInTheDocument();
    await userEvent.click(within(detail()).getByRole('button', { name: 'Previous task' }));
    expect(within(detail()).getByText('1 of 4')).toBeInTheDocument();
  });

  it('approving moves to the next item, then back to the list when none are left', async () => {
    await renderMobile();
    await userEvent.click(queueRow('Ship release'));
    await userEvent.click(await within(detail()).findByRole('button', { name: /^Approve/ }));
    await waitFor(() => expect(within(detail()).getByText('Reorganise notes')).toBeInTheDocument());
    expect(within(detail()).getByText('1 of 3')).toBeInTheDocument();
    expect(api.posts).toEqual([{ id: 1, kind: 'approve', text: '' }]);
  });

  /** Drag a queue row the way a finger would; returns the row so its revealed buttons can be reached. */
  function swipe(title: string, dx: number) {
    const rowEl = screen.getByText(title).closest('[data-task-row]') as HTMLElement;
    const surface = within(rowEl).getByRole('button', { name: new RegExp(title) });
    fireEvent.pointerDown(surface, { clientX: 200, clientY: 100, pointerId: 1 });
    fireEvent.pointerMove(surface, { clientX: 200 + dx / 2, clientY: 100, pointerId: 1 });
    fireEvent.pointerMove(surface, { clientX: 200 + dx, clientY: 100, pointerId: 1 });
    fireEvent.pointerUp(surface, { clientX: 200 + dx, clientY: 100, pointerId: 1 });
    return rowEl;
  }

  it('swipes a queue row right to mark the task done, without opening it', async () => {
    await renderMobile();
    swipe('Unowned idea', 130);
    await waitFor(() => expect(api.calls).toContain('POST /api/tasks/4/done'));
    expect(detail()).toBeNull();
    await waitFor(() => expect(screen.queryByRole('button', { name: /Unowned idea/ })).not.toBeInTheDocument());
  });

  it('swipes a queue row left to set its due date', async () => {
    await renderMobile();
    const rowEl = swipe('Unowned idea', -150);
    await userEvent.click(within(rowEl).getByRole('button', { name: /Tmrw/ }));
    await waitFor(() => expect(api.calls).toContain('PATCH /api/tasks/4'));
    expect(detail()).toBeNull();
  });

  it('returns to the queue from the back button', async () => {
    await renderMobile();
    await userEvent.click(queueRow('Ship release'));
    await userEvent.click(within(detail()).getByRole('button', { name: /Next$/ }));
    expect(detail()).toBeNull();
  });

  it('has an All tasks list with filter pills and a grouping control', async () => {
    await renderMobile();
    await userEvent.click(screen.getByRole('tab', { name: /All tasks/ }));
    expect(screen.getByRole('group', { name: 'Filter tasks' })).toBeInTheDocument();
    expect(screen.getByLabelText('Group by')).toBeInTheDocument();
    expect(screen.getByText('Water plants')).toBeInTheDocument();
  });
});
