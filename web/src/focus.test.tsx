import { beforeEach, describe, expect, it } from 'vitest';
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import App from './App';
import { installFakeApi } from './test/fakeApi';
import { setViewport } from './test/setup';
import { makeTask } from './test/factories';
import { addDays, formatLocalDate } from './taskFormat';

const day = (n: number) => formatLocalDate(addDays(new Date(), n));
const agents = ['CommandCenter', 'Vault'].map((name) => ({ name, dir: `/${name}` }));

const tasks = [
  makeTask({ id: 1, title: 'Review due today', agent: 'Vault', work_status: 'waiting-for-review', due_date: day(0) }),
  makeTask({ id: 2, title: 'Overdue question', agent: 'Vault', work_status: 'needs-input', due_date: day(-3) }),
  makeTask({ id: 3, title: 'Undated idea', agent: undefined, due_date: undefined }),
  makeTask({ id: 4, title: 'Review due later', agent: 'Vault', work_status: 'waiting-for-review', due_date: day(5) }),
  makeTask({ id: 5, title: 'Agent busy', agent: 'Vault', work_status: 'in-progress', due_date: day(0) }),
  makeTask({ id: 6, title: 'Snoozed review', agent: 'Vault', work_status: 'waiting-for-review', due_date: day(0) }),
];

const queueRow = (title: string) => screen.getByRole('button', { name: new RegExp(title) });
const hasRow = (title: string) => screen.queryByRole('button', { name: new RegExp(title) }) !== null;
const focusButton = () => screen.getByRole('button', { name: /^Focus/ });
const docked = () => screen.getByRole('region', { name: 'Selected task' });

async function renderNext(opts: { snoozed?: boolean; focus?: boolean } = {}) {
  if (opts.snoozed !== false) localStorage.setItem('ccDeferrals', JSON.stringify({ 6: Date.now() + 3_600_000 }));
  if (opts.focus) localStorage.setItem('ccFocus', 'on');
  render(<App />);
  // Loaded once either the queue or, with Focus emptying it, the empty message is on screen.
  await waitFor(() => expect(screen.queryByRole('region', { name: 'Needs you' }) ?? screen.queryByText('Nothing to work on right now.')).toBeTruthy());
}

beforeEach(() => {
  setViewport(false);
  installFakeApi({ tasks, agents });
});

describe('Focus', () => {
  it('is off by default: the queue shows everything that needs you', async () => {
    await renderNext();
    expect(focusButton()).toHaveAttribute('aria-pressed', 'false');
    for (const t of ['Review due today', 'Overdue question', 'Undated idea', 'Review due later', 'Snoozed review']) expect(hasRow(t)).toBe(true);
  });

  it('keeps what can be worked now: overdue, due today and undated', async () => {
    await renderNext();
    await userEvent.click(focusButton());
    expect(focusButton()).toHaveAttribute('aria-pressed', 'true');
    for (const t of ['Review due today', 'Overdue question', 'Undated idea']) expect(hasRow(t)).toBe(true);
  });

  it('hides anything due later, anything deferred, and the agents’ work', async () => {
    await renderNext();
    await userEvent.click(focusButton());
    expect(hasRow('Review due later')).toBe(false);
    expect(hasRow('Snoozed review')).toBe(false);
    expect(screen.queryByRole('region', { name: 'Agents working' })).not.toBeInTheDocument();
    expect(hasRow('Agent busy')).toBe(false);
  });

  it('says how many tasks it is hiding', async () => {
    await renderNext();
    await userEvent.click(focusButton());
    expect(focusButton()).toHaveTextContent('3 hidden');
  });

  it('makes the Next count match what is listed', async () => {
    await renderNext();
    expect(within(screen.getByRole('tab', { name: /Next/ })).getByLabelText('5 tasks')).toBeInTheDocument();
    await userEvent.click(focusButton());
    expect(within(screen.getByRole('tab', { name: /Next/ })).getByLabelText('3 tasks')).toBeInTheDocument();
  });

  it('brings everything back when turned off', async () => {
    await renderNext();
    await userEvent.click(focusButton());
    await userEvent.click(focusButton());
    expect(hasRow('Review due later')).toBe(true);
    expect(hasRow('Snoozed review')).toBe(true);
  });

  it('is toggled by the f key, and by the command palette', async () => {
    await renderNext();
    await userEvent.keyboard('f');
    expect(focusButton()).toHaveAttribute('aria-pressed', 'true');
    await userEvent.keyboard('f');
    expect(focusButton()).toHaveAttribute('aria-pressed', 'false');

    await userEvent.keyboard('{Control>}k{/Control}');
    await userEvent.type(await screen.findByPlaceholderText(/Search tasks or type a command/), 'Focus');
    await userEvent.click(await screen.findByText('Turn Focus on'));
    await waitFor(() => expect(focusButton()).toHaveAttribute('aria-pressed', 'true'));
  });

  it('is remembered across reloads', async () => {
    await renderNext();
    await userEvent.click(focusButton());
    document.body.innerHTML = '';
    render(<App />);
    await waitFor(() => expect(focusButton()).toHaveAttribute('aria-pressed', 'true'));
    expect(hasRow('Review due later')).toBe(false);
  });

  it('belongs to Next only: no button and no shortcut in All tasks', async () => {
    await renderNext();
    await userEvent.click(screen.getByRole('tab', { name: /All tasks/ }));
    expect(screen.queryByRole('button', { name: /^Focus/ })).not.toBeInTheDocument();
    await userEvent.keyboard('f');
    await userEvent.click(screen.getByRole('tab', { name: /Next/ }));
    expect(focusButton()).toHaveAttribute('aria-pressed', 'false');
  });

  it('leaves All tasks showing everything while Focus is on', async () => {
    await renderNext({ focus: true });
    await userEvent.click(screen.getByRole('tab', { name: /All tasks/ }));
    const titles = Array.from(document.querySelectorAll('[data-task-row]')).map((el) => el.textContent ?? '');
    for (const t of ['Review due later', 'Agent busy', 'Snoozed review']) expect(titles.some((x) => x.includes(t))).toBe(true);
  });

  it('moves the docked pane off a task that Focus hides', async () => {
    await renderNext();
    await userEvent.click(queueRow('Review due later'));
    await waitFor(() => expect(within(docked()).getByText('Review due later')).toBeInTheDocument());
    await userEvent.click(focusButton());
    // It moves to the nearest task that is still listed, here the review due today above it.
    await waitFor(() => expect(within(docked()).getByText('Review due today')).toBeInTheDocument());
    expect(queueRow('Review due today')).toHaveAttribute('aria-current', 'true');
    expect(within(docked()).queryByText('Review due later')).not.toBeInTheDocument();
  });

  it('drops a task from the queue when it is deferred, and moves on', async () => {
    await renderNext({ focus: true, snoozed: false });
    // Undated and unassigned, so it is first in the queue.
    await waitFor(() => expect(queueRow('Undated idea')).toHaveAttribute('aria-current', 'true'));
    await userEvent.keyboard('z');
    await waitFor(() => expect(hasRow('Undated idea')).toBe(false));
    expect(queueRow('Overdue question')).toHaveAttribute('aria-current', 'true');
  });

  it('turns a deferral into a hidden task, and Show everything undoes the hiding', async () => {
    installFakeApi({
      tasks: [
        makeTask({ id: 1, title: 'Later review', agent: 'Vault', work_status: 'waiting-for-review', due_date: day(4) }),
        makeTask({ id: 2, title: 'Agent busy', agent: 'Vault', work_status: 'in-progress' }),
      ],
      agents,
    });
    await renderNext({ focus: true, snoozed: false });
    expect(await screen.findByText('Nothing to work on right now.')).toBeInTheDocument();
    expect(screen.getByText(/2 tasks are hidden/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Show everything' }));
    expect(focusButton()).toHaveAttribute('aria-pressed', 'false');
    expect(await screen.findByRole('button', { name: /Later review/ })).toBeInTheDocument();
  });

  it('starts on when it was left on, on a phone', async () => {
    setViewport(true);
    await renderNext({ focus: true });
    expect(hasRow('Review due today')).toBe(true);
    expect(hasRow('Review due later')).toBe(false);
  });
});

describe('Focus on a phone', () => {
  beforeEach(() => setViewport(true));

  it('has its own toggle under the view tabs', async () => {
    await renderNext();
    expect(focusButton()).toHaveAttribute('aria-pressed', 'false');
    await userEvent.click(focusButton());
    expect(focusButton()).toHaveAttribute('aria-pressed', 'true');
    expect(hasRow('Review due later')).toBe(false);
    expect(hasRow('Overdue question')).toBe(true);
  });

  it('is hidden in All tasks', async () => {
    await renderNext();
    await userEvent.click(screen.getByRole('tab', { name: /All tasks/ }));
    expect(screen.queryByRole('button', { name: /^Focus/ })).not.toBeInTheDocument();
  });
});
