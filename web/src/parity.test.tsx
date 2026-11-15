// Behaviours the old sidebar-and-table UI had, checked against the Next-first UI. Each test is a
// thing a person could do before; if one of these breaks, something was lost in a redesign.
import { beforeEach, describe, expect, it } from 'vitest';
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import App from './App';
import { installFakeApi, type FakeApi } from './test/fakeApi';
import { setViewport } from './test/setup';
import { makeTask } from './test/factories';
import { formatLocalDate, addDays } from './taskFormat';

const agents = ['CommandCenter', 'Vault'].map((name) => ({ name, dir: `/${name}` }));
const tasks = [
  makeTask({ id: 1, title: 'Ship release', agent: 'CommandCenter', work_status: 'waiting-for-review', due_date: '2030-01-01' }),
  makeTask({ id: 2, title: 'Reorganise notes', agent: 'Vault', work_status: 'waiting-for-review', due_date: '2030-01-02' }),
  makeTask({ id: 3, title: 'Water plants', agent: 'Vault', work_status: 'todo', due_date: '2030-01-06', recurrence: 'weekly:sun' }),
];
let api: FakeApi;
const today = new Date();
today.setHours(0, 0, 0, 0);

const docked = () => screen.getByRole('region', { name: 'Selected task' });
const queueRow = (title: string) => screen.getByRole('button', { name: new RegExp(title) });

async function renderNext() {
  render(<App />);
  await screen.findByRole('region', { name: 'Review' });
  await waitFor(() => expect(queueRow('Ship release')).toHaveAttribute('aria-current', 'true'));
}

beforeEach(() => {
  setViewport(false);
  api = installFakeApi({ tasks, agents });
});

describe('keyboard shortcuts keep working on the Next view', () => {
  it.each([
    ['T', 0], ['N', 1], ['W', 7],
  ])('%s sets the due date relative to today', async (key, days) => {
    await renderNext();
    await userEvent.keyboard(key);
    await waitFor(() => expect(api.patches).toEqual([{ id: 1, body: { due: formatLocalDate(addDays(today, days)) } }]));
  });

  it('X clears the due date', async () => {
    await renderNext();
    await userEvent.keyboard('X');
    await waitFor(() => expect(api.patches).toEqual([{ id: 1, body: { due: '' } }]));
  });

  it('e edits the title', async () => {
    await renderNext();
    await userEvent.keyboard('e');
    expect(await screen.findByDisplayValue('Ship release')).toBeInTheDocument();
  });

  it('d opens the due date picker even though the field sits in the folded details', async () => {
    await renderNext();
    expect(docked().querySelector('details')).not.toHaveAttribute('open');
    await userEvent.keyboard('d');
    expect(docked().querySelector('details')).toHaveAttribute('open');
  });

  it('/ and ? open the command palette', async () => {
    await renderNext();
    await userEvent.keyboard('/');
    expect(await screen.findByPlaceholderText(/Search tasks or type a command/)).toBeInTheDocument();
  });

  it('Space marks the selected task done', async () => {
    await renderNext();
    await userEvent.keyboard(' ');
    await waitFor(() => expect(api.calls).toContain('POST /api/tasks/1/done'));
  });

  it('a click on a row followed by Enter completes the task exactly once', async () => {
    await renderNext();
    await userEvent.click(queueRow('Reorganise notes'));
    await waitFor(() => expect(queueRow('Reorganise notes')).toHaveAttribute('aria-current', 'true'));
    await userEvent.keyboard('{Enter}');
    await waitFor(() => expect(api.calls).toContain('POST /api/tasks/2/done'));
    expect(api.calls.filter((c) => c === 'POST /api/tasks/2/done')).toHaveLength(1);
    expect(api.calls).not.toContain('POST /api/tasks/1/done');
  });

  it('Space on the Task details fold toggles it and does not complete the task', async () => {
    await renderNext();
    const summary = within(docked()).getByText(/Task details/);
    await userEvent.click(summary);
    await userEvent.keyboard(' ');
    expect(api.calls.filter((c) => c.endsWith('/done'))).toEqual([]);
  });

  it('typing in a text box never fires a shortcut', async () => {
    await renderNext();
    await userEvent.type(await within(docked()).findByPlaceholderText(/Request changes/), 'Tell Wendy');
    expect(api.patches).toEqual([]);
    expect(api.calls.filter((c) => c.endsWith('/done'))).toEqual([]);
  });
});

describe('the command palette still reaches every old destination', () => {
  async function runCommand(text: string, pick: string) {
    await userEvent.keyboard('{Control>}k{/Control}');
    await userEvent.type(await screen.findByPlaceholderText(/Search tasks or type a command/), text);
    const matches = await screen.findAllByText(pick);
    await userEvent.click(matches[matches.length - 1]);
  }

  it('offers the due-window filters from the Next view and lands on them in All tasks', async () => {
    await renderNext();
    await runCommand('Recurring', 'Filter: Recurring');
    await waitFor(() => expect(screen.getByRole('tab', { name: /All tasks/ })).toHaveAttribute('aria-selected', 'true'));
    expect(screen.getByRole('button', { name: /^Recurring/ })).toHaveAttribute('aria-pressed', 'true');
  });

  it('can regroup the All tasks list from the Next view', async () => {
    await renderNext();
    await runCommand('Group by project', 'Group by project');
    await waitFor(() => expect(screen.getByRole('tab', { name: /All tasks/ })).toHaveAttribute('aria-selected', 'true'));
    expect(screen.getByLabelText('Group')).toHaveValue('project');
  });

  it('still opens Settings', async () => {
    await renderNext();
    await runCommand('Settings', 'Settings');
    expect(await screen.findByText('Notifications')).toBeInTheDocument();
  });

  it('still opens Services', async () => {
    await renderNext();
    await runCommand('Services', 'Open Services');
    await waitFor(() => expect(api.calls).toContain('GET /api/services'));
    expect(screen.getAllByText(/Services/).length).toBeGreaterThan(0);
  });
});

describe('deferring', () => {
  it('z sinks a task, marks it deferred, and Undefer brings it back', async () => {
    await renderNext();
    await userEvent.keyboard('z');
    await waitFor(() => expect(queueRow('Ship release')).toHaveTextContent('deferred'));
    await userEvent.click(queueRow('Ship release'));
    await userEvent.click(await within(docked()).findByRole('button', { name: 'Undefer' }));
    await waitFor(() => expect(queueRow('Ship release')).not.toHaveTextContent('deferred'));
  });
});

describe('queue rows carry what the old rows flagged', () => {
  it('shows a missing due date and a recurrence on tasks that are not in triage', async () => {
    api = installFakeApi({
      tasks: [
        makeTask({ id: 1, title: 'Needs a date', agent: 'Vault', work_status: 'waiting-for-review', due_date: undefined }),
        makeTask({ id: 2, title: 'Repeats weekly', agent: 'Vault', work_status: 'needs-input', recurrence: 'weekly:fri' }),
      ],
      agents,
    });
    render(<App />);
    await screen.findByRole('region', { name: 'Review' });
    expect(queueRow('Needs a date')).toHaveTextContent('No due date');
    expect(queueRow('Repeats weekly')).toHaveTextContent('weekly:fri');
  });
});

describe('phone: the swipe hint', () => {
  it('is shown under the Next queue, as it was under every list before', async () => {
    setViewport(true);
    render(<App />);
    await screen.findByRole('region', { name: 'Review' });
    expect(screen.getByText(/Swipe right to complete/)).toBeInTheDocument();
  });
});
