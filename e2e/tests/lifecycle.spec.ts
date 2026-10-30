// End-to-end: the human drives the web UI while the `task` CLI plays the
// agent, and every step is checked both on screen and in the stored state.
import { expect, inspectorButton, isoDate, openTask, statusControl, statusOptions, test } from './fixtures';

const BOT = 'Bot';

test('review loop: submit, feedback, resubmit, done, reopen', async ({ page, cc }) => {
  const id = cc.add('Write the report', '--agent', BOT);
  cc.agent(BOT, 'edit', String(id), '--work-status', 'in-progress');
  cc.submit(BOT, id);

  await openTask(page, cc, 'Write the report');
  await expect(statusControl(page)).toContainText('Waiting for Review');
  // The human may only reset from here: no agent states on offer.
  expect(await statusOptions(page)).toEqual(['Waiting for Review', 'To Do']);
  await page.keyboard.press('Escape');
  await page.locator('#field-title').click({ position: { x: 1, y: 1 } });

  const composer = page.getByPlaceholder('Request changes…', { exact: false });
  await composer.fill('Tighten the intro');
  await composer.press('Enter');
  await expect(statusControl(page)).toContainText('Changes Requested');
  expect((await cc.task(id)).state).toBe('changes-requested');

  // The agent picks it up again; the UI follows live and closes the composer.
  cc.agent(BOT, 'edit', String(id), '--work-status', 'in-progress');
  await expect(statusControl(page)).toContainText('In Progress');
  await expect(composer).toBeHidden();
  cc.submit(BOT, id);
  await expect(statusControl(page)).toContainText('Waiting for Review');

  await inspectorButton(page, 'Mark done').click();
  await expect(inspectorButton(page, 'Reopen')).toBeVisible();
  expect((await cc.task(id)).state).toBe('done');

  await inspectorButton(page, 'Reopen').click();
  await expect(inspectorButton(page, 'Mark done')).toBeVisible();
  const reopened = await cc.task(id);
  expect([reopened.state, reopened.work_status]).toEqual(['todo', undefined]);
});

test('feedback on a done task reopens it for another round', async ({ page, cc }) => {
  const id = cc.add('Ship the release', '--agent', BOT);
  cc.submit(BOT, id);
  await openTask(page, cc, 'Ship the release');
  await inspectorButton(page, 'Mark done').click();
  await expect(inspectorButton(page, 'Reopen')).toBeVisible();

  const composer = page.getByPlaceholder('Request changes…', { exact: false });
  await composer.fill('One more thing');
  await composer.press('Enter');
  await expect(statusControl(page)).toContainText('Changes Requested');
  await expect(inspectorButton(page, 'Mark done')).toBeVisible();
  const t = await cc.task(id);
  expect([t.status, t.state]).toEqual(['open', 'changes-requested']);
});

test('message: the human writes to a working agent without changing its status', async ({ page, cc }) => {
  const id = cc.add('Refactor the parser', '--agent', BOT);
  cc.agent(BOT, 'edit', String(id), '--work-status', 'in-progress');
  await openTask(page, cc, 'Refactor the parser');

  const composer = page.getByPlaceholder('Message the agent…', { exact: false });
  await composer.fill('Also handle empty input');
  await composer.press('Enter');
  await expect(page.getByRole('paragraph').filter({ hasText: 'Also handle empty input' })).toBeVisible();
  await expect(page.getByText('Not yet seen by the agent')).toBeVisible();
  // Writing is not a move: the task is still the agent's, in progress.
  expect((await cc.task(id)).state).toBe('in-progress');
  expect(cc.agent(BOT, 'next', '--json')).toContain('"pickup": "message"');

  // The agent answers and the marker clears.
  cc.reply(BOT, id, 'Done, empty input is handled.');
  await expect(page.getByRole('paragraph').filter({ hasText: 'Done, empty input is handled.' })).toBeVisible();
  await expect(page.getByText('Not yet seen by the agent')).toBeHidden();
});

test('approve: accepting reviewed work marks the task done', async ({ page, cc }) => {
  const id = cc.add('Draft the post', '--agent', BOT);
  cc.agent(BOT, 'edit', String(id), '--work-status', 'in-progress');
  cc.reply(BOT, id, 'Draft is ready.');
  cc.submit(BOT, id);
  await openTask(page, cc, 'Draft the post');

  await page.getByRole('button', { name: 'Approve' }).click();
  await expect(inspectorButton(page, 'Reopen')).toBeVisible();
  const t = await cc.task(id);
  expect([t.status, t.state]).toEqual(['done', 'done']);
  expect(cc.human('thread', 'show', String(id))).toContain('## Approval —');
});

test('question: the agent asks, the human answers', async ({ page, cc }) => {
  const id = cc.add('Pick a colour', '--agent', BOT);
  cc.agent(BOT, 'edit', String(id), '--work-status', 'in-progress');
  cc.ask(BOT, id, 'Blue or green?');

  await openTask(page, cc, 'Pick a colour');
  await expect(statusControl(page)).toContainText('Needs Input');
  await expect(page.getByText('Blue or green?')).toBeVisible();

  const answer = page.getByPlaceholder('Answer the agent…', { exact: false });
  await answer.fill('Blue');
  await answer.press('Enter');
  await expect(statusControl(page)).toContainText('Changes Requested');
  // An answered question stays hidden until the agent asks again.
  await expect(answer).toBeHidden();
  expect((await cc.task(id)).state).toBe('changes-requested');
});

test('the human works their own task from the status dropdown', async ({ page, cc }) => {
  const id = cc.add('Buy milk');
  await openTask(page, cc, 'Buy milk');

  const options = await statusOptions(page);
  expect(options).toEqual(['None', 'To Do', 'In Progress']);
  await page.locator('select#field-work-status').selectOption({ label: 'In Progress' });
  await expect(statusControl(page)).toContainText('In Progress');
  expect((await cc.task(id)).state).toBe('in-progress');

  expect(await statusOptions(page)).toEqual(['In Progress', 'To Do']);
  await page.locator('select#field-work-status').selectOption({ label: 'To Do' });
  await expect(statusControl(page)).toContainText('To Do');
  expect((await cc.task(id)).state).toBe('todo');
});

test("the dropdown never offers the agent's moves on an agent's task", async ({ page, cc }) => {
  const id = cc.add('Agent work', '--agent', BOT);
  await openTask(page, cc, 'Agent work');
  expect(await statusOptions(page)).toEqual(['None', 'To Do']);

  cc.agent(BOT, 'edit', String(id), '--work-status', 'in-progress');
  await page.keyboard.press('Escape');
  await page.locator('#field-title').click({ position: { x: 1, y: 1 } });
  await expect(statusControl(page)).toContainText('In Progress');
  expect(await statusOptions(page)).toEqual(['In Progress', 'To Do']);
});

test('pushing a review task later hands it back to the human', async ({ page, cc }) => {
  const id = cc.add('Read the draft', '--agent', BOT, '--due', isoDate(0));
  cc.submit(BOT, id);
  await openTask(page, cc, 'Read the draft');

  await page.locator('#field-due').click();
  await page.getByTitle('Next month').click();
  await page.getByRole('button', { name: '15', exact: true }).first().click();

  await expect.poll(async () => (await cc.task(id)).agent).toBe('human');
  const t = await cc.task(id);
  expect([t.state, t.work_status]).toEqual(['todo', undefined]);
  await expect(statusControl(page)).toContainText('+ Work status');
});

test('services: convert, pause, resume and a failed run', async ({ page, cc }) => {
  const id = cc.add('Nightly backups', '--agent', BOT, '--due', isoDate(0));
  await openTask(page, cc, 'Nightly backups');

  await page.getByRole('button', { name: 'Convert to service…' }).click();
  await page.getByPlaceholder('Schedule', { exact: false }).fill('daily');
  await page.getByRole('button', { name: 'Convert', exact: true }).click();

  // The Services screen opens with the new service on it.
  const row = page.locator('tr').filter({ hasText: 'Nightly backups' });
  await expect(row).toContainText('active');
  expect((await cc.service(id)).state).toBe('scheduled');

  await row.click();
  await page.getByRole('button', { name: 'Pause' }).click();
  await expect(row).toContainText('paused');
  expect((await cc.service(id)).paused).toBe(true);

  await page.getByRole('button', { name: 'Resume' }).click();
  await expect(row).toContainText('active');
  expect((await cc.service(id)).paused).toBe(false);

  // The agent records a failed run; the screen follows live.
  cc.agent(BOT, 'service', 'run', String(id), '--result', 'failed', '--summary', 'disk full');
  await expect(row).toContainText('failing');
  expect((await cc.service(id)).state).toBe('failing');
});
