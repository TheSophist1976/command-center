import { test as base, expect, type Page } from '@playwright/test';
import { execFileSync, spawn, type ChildProcess } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

/** The `task` binary under test; CI points this at the build it just made. */
const TASK_BIN = process.env.TASK_BIN ?? fileURLToPath(new URL('../../target/debug/task', import.meta.url));

export interface StoredTask {
  id: number;
  title: string;
  status: 'open' | 'done';
  agent?: string;
  work_status?: string;
  kind?: string;
  paused?: boolean;
  state: string;
  allowed_actions: string[];
}

/** One Command Center: a temporary database, `task serve` on a free port, and the CLI. */
export class CommandCenter {
  constructor(
    readonly dir: string,
    readonly db: string,
    readonly baseURL: string,
    private readonly server: ChildProcess,
  ) {}

  static async start(): Promise<CommandCenter> {
    const dir = mkdtempSync(join(tmpdir(), 'cc-e2e-'));
    const port = await freePort();
    const db = join(dir, 'tasks.db');
    const server = spawn(TASK_BIN, ['--file', db, 'serve'], {
      env: { ...isolatedEnv(dir), TASK_SERVER_PORT: String(port) },
      stdio: 'ignore',
    });
    const cc = new CommandCenter(dir, db, `http://127.0.0.1:${port}`, server);
    await cc.waitUntilServing();
    return cc;
  }

  async stop() {
    this.server.kill();
    rmSync(this.dir, { recursive: true, force: true });
  }

  /** Run the CLI as the human. */
  human(...args: string[]): string {
    return this.run(undefined, args);
  }

  /** Run the CLI as an agent (TASK_AGENT set). */
  agent(name: string, ...args: string[]): string {
    return this.run(name, args);
  }

  /** The task as the API reports it, including its lifecycle state. */
  async task(id: number): Promise<StoredTask> {
    const r = await fetch(`${this.baseURL}/api/tasks/${id}`);
    expect(r.ok, `GET task ${id}`).toBe(true);
    return r.json();
  }

  async service(id: number): Promise<StoredTask> {
    const r = await fetch(`${this.baseURL}/api/services`);
    const all: StoredTask[] = await r.json();
    const s = all.find((x) => x.id === id);
    expect(s, `service ${id}`).toBeTruthy();
    return s!;
  }

  /** Add a task assigned to `agent` and return its id. */
  add(title: string, ...args: string[]): number {
    const out = this.human('add', title, ...args);
    return Number(/task (\d+)/.exec(out)![1]);
  }

  /** The agent links a result note and hands the task back for review. */
  submit(agent: string, id: number) {
    this.agent(agent, 'note', 'add', `Result ${id}`, '--task', String(id));
    this.agent(agent, 'edit', String(id), '--work-status', 'waiting-for-review');
  }

  /** The agent writes a question note and blocks on it. */
  ask(agent: string, id: number, question: string) {
    this.agent(agent, 'note', 'add', `Task ${id} Question`, '--task', String(id));
    this.agent(agent, 'note', 'edit', `task-${id}-question`, '--body', question);
    this.agent(agent, 'edit', String(id), '--work-status', 'needs-input');
  }

  private run(agent: string | undefined, args: string[]): string {
    const env = isolatedEnv(this.dir);
    if (agent) env.TASK_AGENT = agent;
    return execFileSync(TASK_BIN, ['--file', this.db, ...args], { env, cwd: this.dir, encoding: 'utf8' });
  }

  private async waitUntilServing() {
    const deadline = Date.now() + 20_000;
    for (;;) {
      try {
        if ((await fetch(`${this.baseURL}/api/tasks`)).ok) return;
      } catch {
        // not listening yet
      }
      if (Date.now() > deadline) throw new Error(`task serve did not start at ${this.baseURL}`);
      await new Promise((r) => setTimeout(r, 100));
    }
  }
}

/** The environment for every `task` process: no real config, home or agent identity. */
function isolatedEnv(dir: string): NodeJS.ProcessEnv {
  const env: NodeJS.ProcessEnv = { ...process.env, HOME: dir, TASK_CONFIG_FILE: join(dir, 'config') };
  delete env.TASK_AGENT;
  return env;
}

function freePort(): Promise<number> {
  return new Promise((resolvePort, reject) => {
    const srv = createServer();
    srv.once('error', reject);
    srv.listen(0, '127.0.0.1', () => {
      const { port } = srv.address() as { port: number };
      srv.close(() => resolvePort(port));
    });
  });
}

export function isoDate(daysFromToday: number): string {
  const d = new Date();
  d.setDate(d.getDate() + daysFromToday);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

/** Open the app on the "All tasks" view and select a task by title. */
export async function openTask(page: Page, cc: CommandCenter, title: string) {
  await page.goto(cc.baseURL);
  await page.getByText('All tasks', { exact: true }).first().click();
  await page.locator('[data-task-row]').filter({ hasText: title }).first().click();
  await expect(page.locator('#field-title')).toContainText(title);
}

/** The work-status control in the inspector (a select while editing). */
export function statusControl(page: Page) {
  return page.locator('#field-work-status');
}

/** The labels the work-status dropdown offers. */
export async function statusOptions(page: Page): Promise<string[]> {
  await statusControl(page).click();
  const select = page.locator('select#field-work-status');
  await expect(select).toBeVisible();
  const labels = await select.locator('option').allTextContents();
  return labels;
}

/** A labelled button in the inspector (the list row's checkbox shares the name, so skip aria-labelled icons). */
export function inspectorButton(page: Page, name: string) {
  return page.locator('button:not([aria-label])', { hasText: name });
}

export const test = base.extend<{ cc: CommandCenter }>({
  cc: async ({}, use) => {
    const cc = await CommandCenter.start();
    await use(cc);
    await cc.stop();
  },
});

export { expect };
