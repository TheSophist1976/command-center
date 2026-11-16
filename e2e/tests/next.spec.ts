// End-to-end: the Next view is the home screen. The `task` CLI plays the agent, the human
// works the queue in the browser, and every move is checked in the stored state too.
import { section, docked, expect, isoDate, openNext, row, swipeRow, test, type CommandCenter } from './fixtures';

const BOT = 'Bot';

/** The agent finishes a task and hands it back for review, with a message explaining the work. */
function awaitReview(cc: CommandCenter, title: string, dueInDays: number, message: string): number {
  const id = cc.add(title, '--agent', BOT, '--due', isoDate(dueInDays));
  cc.agent(BOT, 'edit', String(id), '--work-status', 'in-progress');
  cc.reply(BOT, id, message);
  cc.submit(BOT, id);
  return id;
}

/** The agent asks a question and blocks on the answer. */
function askQuestion(cc: CommandCenter, title: string, dueInDays: number, question: string): number {
  const id = cc.add(title, '--agent', BOT, '--due', isoDate(dueInDays));
  cc.agent(BOT, 'edit', String(id), '--work-status', 'in-progress');
  cc.ask(BOT, id, question);
  return id;
}

test('opens on the queue, with the first task docked and its conversation showing', async ({ page, cc }) => {
  awaitReview(cc, 'Write the report', 1, 'The report is ready.');
  askQuestion(cc, 'Pick a colour', 2, 'Blue or green?');
  cc.add('Unowned idea');
  const working = cc.add('Refactor the parser', '--agent', BOT, '--due', isoDate(3));
  cc.agent(BOT, 'edit', String(working), '--work-status', 'in-progress');

  await openNext(page, cc);

  // Everything waiting on you is one list; each row says why it is there.
  for (const title of ['Write the report', 'Pick a colour', 'Unowned idea']) {
    await expect(section(page, 'Needs you')).toContainText(title);
  }
  await expect(row(page, 'Write the report')).toContainText('Ready for your review');
  await expect(row(page, 'Pick a colour')).toContainText('Waiting for your answer');
  await expect(row(page, 'Unowned idea')).toContainText('Needs an agent');
  // Agents working is awareness only: counted, but folded away until opened.
  await expect(section(page, 'Agents working')).toContainText('1');
  await expect(row(page, 'Refactor the parser')).toHaveCount(0);
  await section(page, 'Agents working').getByRole('button').click();
  await expect(row(page, 'Refactor the parser')).toBeVisible();

  // The first task is docked without a click. A task with no agent or date leads the queue.
  await expect(docked(page).locator('#field-title')).toContainText('Unowned idea');
  await row(page, 'Write the report').click();
  await expect(docked(page).locator('#field-title')).toContainText('Write the report');
  await expect(docked(page)).toContainText('The report is ready.');
  await expect(docked(page).getByRole('button', { name: 'Approve' })).toBeVisible();
  // Everything else about the task is one fold away rather than competing with the conversation.
  await expect(docked(page).getByText('Task details')).toBeVisible();
});

test('the queue is ordered by the old rules: needs-triage first, then urgency, deferred last', async ({ page, cc }) => {
  awaitReview(cc, 'Later review', 9, 'Later.');
  askQuestion(cc, 'Soon question', 1, 'Soon?');
  cc.add('Unowned idea');
  awaitReview(cc, 'Today review', 0, 'Today.');
  await openNext(page, cc);

  const titles = () => section(page, 'Needs you').locator('[data-task-row]').evaluateAll((els) => els.map((e) => e.textContent ?? ''));
  const order = (all: string[]) => ['Unowned idea', 'Today review', 'Soon question', 'Later review'].map((t) => all.findIndex((x) => x.includes(t)));
  expect(order(await titles())).toEqual([0, 1, 2, 3]);

  // Deferring the task at the top sinks it below everything else, whatever kind of task it is.
  await expect(docked(page).locator('#field-title')).toContainText('Unowned idea');
  await page.keyboard.press('z');
  await expect(row(page, 'Unowned idea')).toContainText('deferred');
  expect(order(await titles())).toEqual([3, 0, 1, 2]);
});

test('there is no sidebar and no Inbox: Next and All tasks are the only views', async ({ page, cc }) => {
  cc.add('Anything');
  await openNext(page, cc);
  await expect(page.getByRole('tab')).toHaveText([/Next/, /All tasks/]);
  await expect(page.getByText('Inbox', { exact: true })).toHaveCount(0);
  await expect(page.getByText('Due window')).toHaveCount(0);
});

test('approving finishes the task and moves on to the next one', async ({ page, cc }) => {
  const first = awaitReview(cc, 'First review', 1, 'First is ready.');
  awaitReview(cc, 'Second review', 2, 'Second is ready.');
  await openNext(page, cc);
  await expect(docked(page).locator('#field-title')).toContainText('First review');

  await docked(page).getByRole('button', { name: 'Approve' }).click();

  await expect(docked(page).locator('#field-title')).toContainText('Second review');
  await expect(docked(page)).toContainText('Second is ready.');
  await expect(row(page, 'First review')).toHaveCount(0);
  expect((await cc.task(first)).state).toBe('done');
  expect(cc.human('thread', 'show', String(first))).toContain('## Approval —');
});

test('requesting changes sends the task back to the agent and moves on', async ({ page, cc }) => {
  const first = awaitReview(cc, 'First review', 1, 'First is ready.');
  awaitReview(cc, 'Second review', 2, 'Second is ready.');
  await openNext(page, cc);

  const composer = docked(page).getByPlaceholder('Request changes…', { exact: false });
  await composer.fill('Tighten the intro');
  await composer.press('Enter');

  await expect(docked(page).locator('#field-title')).toContainText('Second review');
  expect((await cc.task(first)).state).toBe('changes-requested');
  // It is the agent's turn now, so it left the queue for the folded Agents working band.
  await expect(row(page, 'First review')).toHaveCount(0);
  await expect(section(page, 'Agents working')).toContainText('1');
});

test('answering an agent hands the task back and moves on', async ({ page, cc }) => {
  const asked = askQuestion(cc, 'Pick a colour', 1, 'Blue or green?');
  cc.add('Unowned idea');
  await openNext(page, cc);
  await row(page, 'Pick a colour').click();
  await expect(docked(page)).toContainText('Blue or green?');

  const answer = docked(page).getByPlaceholder('Answer the agent…', { exact: false });
  await answer.fill('Blue');
  await answer.press('Enter');

  await expect(docked(page).locator('#field-title')).toContainText('Unowned idea');
  expect((await cc.task(asked)).state).toBe('changes-requested');
  await expect(row(page, 'Pick a colour')).toHaveCount(0);
});

test('keyboard: j and k move through the queue, Enter marks done and moves on', async ({ page, cc }) => {
  awaitReview(cc, 'Alpha review', 1, 'Alpha.');
  const b = awaitReview(cc, 'Beta review', 2, 'Beta.');
  cc.add('Gamma idea');
  await openNext(page, cc);
  // Gamma has no agent or date, so it leads; then Alpha and Beta by due date.
  await expect(docked(page).locator('#field-title')).toContainText('Gamma idea');

  await page.keyboard.press('j');
  await expect(docked(page).locator('#field-title')).toContainText('Alpha review');
  await page.keyboard.press('j');
  await expect(docked(page).locator('#field-title')).toContainText('Beta review');
  await page.keyboard.press('k');
  await expect(docked(page).locator('#field-title')).toContainText('Alpha review');
  await page.keyboard.press('j');
  await page.keyboard.press('Enter');

  await expect.poll(async () => (await cc.task(b)).status).toBe('done');
  await expect(docked(page).locator('#field-title')).toContainText('Alpha review');
});

test('clicking a row and pressing Enter completes that task, once', async ({ page, cc }) => {
  const a = awaitReview(cc, 'Alpha review', 1, 'Alpha.');
  const b = awaitReview(cc, 'Beta review', 2, 'Beta.');
  await openNext(page, cc);

  await row(page, 'Beta review').click();
  await expect(docked(page).locator('#field-title')).toContainText('Beta review');
  await page.keyboard.press('Enter');

  await expect.poll(async () => (await cc.task(b)).status).toBe('done');
  expect((await cc.task(a)).status).toBe('open');
  await expect(docked(page).locator('#field-title')).toContainText('Alpha review');
});

test('Space on the Task details fold opens it and does not complete the task', async ({ page, cc }) => {
  const id = awaitReview(cc, 'Write the report', 1, 'Ready.');
  await openNext(page, cc);
  await expect(docked(page).locator('#field-title')).toContainText('Write the report');

  await docked(page).getByText('Task details').click();
  await expect(docked(page).locator('details')).toHaveAttribute('open', '');
  await page.keyboard.press('Space');

  await expect(docked(page).locator('details')).not.toHaveAttribute('open', '');
  expect((await cc.task(id)).status).toBe('open');
});

test('pressing e edits the title without typing an e into it', async ({ page, cc }) => {
  const id = awaitReview(cc, 'Write the report', 1, 'Ready.');
  await openNext(page, cc);
  await expect(docked(page).locator('#field-title')).toContainText('Write the report');

  await page.keyboard.press('e');
  const editor = docked(page).locator('input').first();
  await expect(editor).toHaveValue('Write the report');
  await page.keyboard.press('Enter');
  expect((await cc.task(id)).title).toBe('Write the report');
});

test('the command palette reaches filters and grouping from the Next view', async ({ page, cc }) => {
  cc.add('Anything');
  await openNext(page, cc);
  await page.keyboard.press('Control+k');
  await page.getByPlaceholder('Search tasks or type a command…').fill('Recurring');
  await page.getByText('Filter: Recurring').click();
  await expect(page.getByRole('tab', { name: /All tasks/ })).toHaveAttribute('aria-selected', 'true');
  await expect(page.getByRole('button', { name: /^Recurring/ })).toHaveAttribute('aria-pressed', 'true');
});

test('Focus shows only what can be worked right now, and f toggles it', async ({ page, cc }) => {
  // (The CLI refuses past due dates, so overdue tasks are covered by the unit tests instead.)
  awaitReview(cc, 'Review today', 0, 'Ready.');
  awaitReview(cc, 'Review tomorrow', 1, 'Ready.');
  awaitReview(cc, 'Review later', 5, 'Ready.');
  cc.add('Undated idea');
  const busy = cc.add('Agent busy', '--agent', BOT, '--due', isoDate(0));
  cc.agent(BOT, 'edit', String(busy), '--work-status', 'in-progress');
  await openNext(page, cc);
  const focus = page.getByRole('button', { name: /^Focus/ });
  await expect(focus).toHaveAttribute('aria-pressed', 'false');
  await expect(row(page, 'Review later')).toBeVisible();

  await focus.click();

  await expect(focus).toHaveAttribute('aria-pressed', 'true');
  await expect(row(page, 'Review today')).toBeVisible();
  await expect(row(page, 'Undated idea')).toBeVisible();
  await expect(row(page, 'Review tomorrow')).toHaveCount(0);
  await expect(row(page, 'Review later')).toHaveCount(0);
  await expect(section(page, 'Agents working')).toHaveCount(0);
  await expect(focus).toContainText('3 hidden');
  await expect(page.getByRole('tab', { name: /Next/ })).toContainText('2');

  await page.keyboard.press('f');
  await expect(focus).toHaveAttribute('aria-pressed', 'false');
  await expect(row(page, 'Review later')).toBeVisible();
});

test('Focus is remembered, hides a task you defer, and Show everything leaves it', async ({ page, cc }) => {
  awaitReview(cc, 'Only review', 0, 'Ready.');
  await openNext(page, cc);
  await page.getByRole('button', { name: /^Focus/ }).click();
  await page.reload();
  await expect(page.getByRole('button', { name: /^Focus/ })).toHaveAttribute('aria-pressed', 'true');
  await expect(row(page, 'Only review')).toBeVisible();

  await page.keyboard.press('z');

  await expect(row(page, 'Only review')).toHaveCount(0);
  await expect(page.getByText('Nothing to work on right now.')).toBeVisible();
  await expect(page.getByText(/1 task is hidden/)).toBeVisible();
  await page.getByRole('button', { name: 'Show everything' }).click();
  await expect(page.getByRole('button', { name: /^Focus/ })).toHaveAttribute('aria-pressed', 'false');
  await expect(row(page, 'Only review')).toBeVisible();
});

test('a task that still needs an agent opens with its details unfolded, so it can be assigned in place', async ({ page, cc }) => {
  const id = cc.add('Unowned idea');
  await openNext(page, cc);
  await expect(docked(page).locator('#field-title')).toContainText('Unowned idea');
  await expect(row(page, 'Unowned idea')).toContainText('Needs an agent');
  await expect(docked(page).locator('#field-agent')).toBeVisible();

  // This environment has no agent profiles, so the picker offers Unassigned and human.
  await docked(page).locator('#field-agent').click();
  await docked(page).getByText('human', { exact: true }).click();

  await expect.poll(async () => (await cc.task(id)).agent).toBe('human');
  await expect(row(page, 'Unowned idea')).toContainText('Assigned to you');
});

test('All tasks is one toggle away and keeps recurring tasks reachable', async ({ page, cc }) => {
  const id = cc.add('Water the plants', '--agent', BOT, '--due', isoDate(1));
  cc.human('edit', String(id), '--recur', 'weekly:fri');
  cc.add('Plain task', '--agent', BOT, '--due', isoDate(2));
  await openNext(page, cc);
  // Agent-owned, not started: nothing for the human to do, so neither is in the Next queue.
  await expect(row(page, 'Water the plants')).toHaveCount(0);

  await page.getByRole('tab', { name: /All tasks/ }).click();
  await expect(row(page, 'Water the plants')).toBeVisible();
  await expect(row(page, 'Plain task')).toBeVisible();

  await page.getByRole('button', { name: /^Recurring/ }).click();
  await expect(row(page, 'Water the plants')).toBeVisible();
  await expect(row(page, 'Plain task')).toHaveCount(0);
});

test('remembers the chosen view across reloads', async ({ page, cc }) => {
  cc.add('Anything');
  await openNext(page, cc);
  await page.getByRole('tab', { name: /All tasks/ }).click();
  await page.reload();
  await expect(page.getByRole('tab', { name: /All tasks/ })).toHaveAttribute('aria-selected', 'true');
});

test('the status strip follows what the agents are doing', async ({ page, cc }) => {
  await openNext(page, cc);
  await expect(page.getByRole('status')).toContainText('services ok');
  await page.getByRole('button', { name: 'Agent and service status' }).click();
  await expect(page.getByRole('button', { name: 'Open Services…' })).toBeVisible();
});

test.describe('on a phone', () => {
  test.use({ viewport: { width: 390, height: 800 }, hasTouch: true });

  test('the queue stacks, a task opens full screen, and approving steps to the next one', async ({ page, cc }) => {
    awaitReview(cc, 'First review', 1, 'First is ready.');
    awaitReview(cc, 'Second review', 2, 'Second is ready.');
    await openNext(page, cc);
    await expect(page.getByRole('region', { name: 'Selected task' })).toHaveCount(0);
    await expect(page.getByRole('button', { name: 'Agent and service status' })).toBeVisible();

    await row(page, 'First review').click();
    const detail = page.locator('.mobile-detail');
    await expect(detail).toContainText('1 of 2');
    await expect(detail).toContainText('First is ready.');

    await detail.getByRole('button', { name: 'Next task' }).click();
    await expect(detail).toContainText('2 of 2');
    await expect(detail).toContainText('Second review');
    await detail.getByRole('button', { name: 'Previous task' }).click();

    await detail.getByRole('button', { name: 'Approve' }).click();
    await expect(detail).toContainText('Second review');
    await expect(detail).toContainText('1 of 1');

    await detail.getByRole('button', { name: /Next$/ }).click();
    await expect(detail).toHaveCount(0);
    await expect(row(page, 'First review')).toHaveCount(0);
  });

  test('swiping a queue row right completes the task without opening it', async ({ page, cc }) => {
    const id = cc.add('Swipe me done');
    cc.add('Stay put');
    await openNext(page, cc);

    await swipeRow(page, row(page, 'Swipe me done'), 140);

    await expect.poll(async () => (await cc.task(id)).status).toBe('done');
    await expect(row(page, 'Swipe me done')).toHaveCount(0);
    await expect(row(page, 'Stay put')).toBeVisible();
    await expect(page.locator('.mobile-detail')).toHaveCount(0);
  });

  test('swiping a queue row left reveals due-date buttons that set the date', async ({ page, cc }) => {
    const id = cc.add('Swipe me later');
    await openNext(page, cc);

    await swipeRow(page, row(page, 'Swipe me later'), -160);
    await row(page, 'Swipe me later').getByRole('button', { name: /Tmrw/ }).click();

    await expect.poll(async () => (await cc.task(id)).due_date).toBe(isoDate(1));
    await expect(page.locator('.mobile-detail')).toHaveCount(0);
  });

  test('a short swipe does nothing, and a tap still opens the task', async ({ page, cc }) => {
    const id = cc.add('Just a tap');
    await openNext(page, cc);

    await swipeRow(page, row(page, 'Just a tap'), 30);
    expect((await cc.task(id)).status).toBe('open');
    await row(page, 'Just a tap').click();
    await expect(page.locator('.mobile-detail')).toContainText('Just a tap');
  });

  test('swiping works on the All tasks list too', async ({ page, cc }) => {
    const id = cc.add('Swipe in all tasks');
    await openNext(page, cc);
    await page.getByRole('tab', { name: /All tasks/ }).click();

    await swipeRow(page, row(page, 'Swipe in all tasks'), 140);

    await expect.poll(async () => (await cc.task(id)).status).toBe('done');
    await expect(page.locator('.mobile-detail')).toHaveCount(0);
  });

  test('Focus has its own toggle and narrows the queue', async ({ page, cc }) => {
    awaitReview(cc, 'Review today', 0, 'Ready.');
    awaitReview(cc, 'Review later', 5, 'Later.');
    await openNext(page, cc);

    await page.getByRole('button', { name: /^Focus/ }).click();

    await expect(row(page, 'Review today')).toBeVisible();
    await expect(row(page, 'Review later')).toHaveCount(0);
    await page.getByRole('tab', { name: /All tasks/ }).click();
    await expect(page.getByRole('button', { name: /^Focus/ })).toHaveCount(0);
  });

  test('All tasks is a card list with filter pills', async ({ page, cc }) => {
    const id = cc.add('Water the plants', '--agent', BOT, '--due', isoDate(1));
    cc.human('edit', String(id), '--recur', 'weekly:fri');
    await openNext(page, cc);
    await page.getByRole('tab', { name: /All tasks/ }).click();
    await expect(row(page, 'Water the plants')).toBeVisible();
    await expect(page.getByRole('group', { name: 'Filter tasks' })).toBeVisible();
    await page.getByRole('button', { name: /^Recurring/ }).click();
    await expect(row(page, 'Water the plants')).toBeVisible();
  });
});
