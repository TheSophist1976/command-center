// End-to-end: the Next view is the home screen. The `task` CLI plays the agent, the human
// works the queue in the browser, and every move is checked in the stored state too.
import { band, docked, expect, isoDate, openNext, row, test, type CommandCenter } from './fixtures';

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

test('opens on the queue in bands, with the first task docked and its conversation showing', async ({ page, cc }) => {
  awaitReview(cc, 'Write the report', 1, 'The report is ready.');
  askQuestion(cc, 'Pick a colour', 2, 'Blue or green?');
  cc.add('Unowned idea');
  const working = cc.add('Refactor the parser', '--agent', BOT, '--due', isoDate(3));
  cc.agent(BOT, 'edit', String(working), '--work-status', 'in-progress');

  await openNext(page, cc);

  await expect(band(page, 'Review')).toContainText('Write the report');
  await expect(band(page, 'Needs your input')).toContainText('Pick a colour');
  await expect(band(page, 'Triage')).toContainText('Unowned idea');
  // Agents working is awareness only: counted, but folded away until opened.
  await expect(band(page, 'Agents working')).toContainText('1');
  await expect(row(page, 'Refactor the parser')).toHaveCount(0);
  await band(page, 'Agents working').getByRole('button').click();
  await expect(row(page, 'Refactor the parser')).toBeVisible();

  await expect(docked(page).locator('#field-title')).toContainText('Write the report');
  await expect(docked(page)).toContainText('The report is ready.');
  await expect(docked(page).getByRole('button', { name: 'Approve' })).toBeVisible();
  // Everything else about the task is one fold away rather than competing with the conversation.
  await expect(docked(page).getByText('Task details')).toBeVisible();
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
  await expect(band(page, 'Review')).not.toContainText('First review');
  await expect(band(page, 'Agents working')).toContainText('1');
});

test('answering an agent hands the task back and moves on', async ({ page, cc }) => {
  const asked = askQuestion(cc, 'Pick a colour', 1, 'Blue or green?');
  cc.add('Unowned idea');
  await openNext(page, cc);
  await expect(docked(page)).toContainText('Blue or green?');

  const answer = docked(page).getByPlaceholder('Answer the agent…', { exact: false });
  await answer.fill('Blue');
  await answer.press('Enter');

  await expect(docked(page).locator('#field-title')).toContainText('Unowned idea');
  expect((await cc.task(asked)).state).toBe('changes-requested');
  await expect(band(page, 'Needs your input')).toHaveCount(0);
});

test('keyboard: j and k move through the queue, Enter marks done and moves on', async ({ page, cc }) => {
  const a = awaitReview(cc, 'Alpha review', 1, 'Alpha.');
  awaitReview(cc, 'Beta review', 2, 'Beta.');
  cc.add('Gamma idea');
  await openNext(page, cc);
  await expect(docked(page).locator('#field-title')).toContainText('Alpha review');

  await page.keyboard.press('j');
  await expect(docked(page).locator('#field-title')).toContainText('Beta review');
  await page.keyboard.press('j');
  await expect(docked(page).locator('#field-title')).toContainText('Gamma idea');
  await page.keyboard.press('k');
  await expect(docked(page).locator('#field-title')).toContainText('Beta review');
  await page.keyboard.press('k');
  await page.keyboard.press('Enter');

  await expect(docked(page).locator('#field-title')).toContainText('Beta review');
  expect((await cc.task(a)).status).toBe('done');
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
