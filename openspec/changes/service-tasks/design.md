## Context

This design follows the ServiceTasks brainstorm (task 75, note `commander-center-servicetasks`) and the human's answers to it:

1. A failed run goes to a new `failing` status, not `needs-input`.
2. Runs live in their own table. A dedicated screen reviews them and pauses, resumes and schedules services. Service tasks appear only there, never in the normal task lists.
3. Services must recur.
4. Missed runs are skipped, not caught up.
5. There is no "review the next run" option.

Current behaviour that matters here:
- `agent::eligible()` (src/agent.rs) builds `task next`. It hides `waiting-for-review`, `complete` and `needs-input` tasks, and recurring tasks that are not due yet.
- `cmd_done` spawns a new task row for the next occurrence via `task::next_due_date`, which advances exactly one step.
- `db::write_all` deletes and reinserts every row in `tasks` on each commit. Any new per-run data must therefore live in another table.

## Goals / Non-Goals

**Goals**
- A service keeps one stable task id for its whole life, and its due date always means "next run".
- Recording a run and advancing the schedule happen in one transaction.
- Agents never compute dates. The CLI owns the schedule.
- Services never appear in normal task views, and the human manages them on one screen.

**Non-Goals**
- Times of day or cron expressions, one-shot services, catch-up runs, and per-cycle review.
- A TUI services screen. The TUI hides services and nothing more for now.

## Decisions

**D1. A service is a task with `kind = service`. It is not a separate `services` table.**
A service needs everything a task already has: agent, instructions, notes, recurrence, due date, priority and project. It also has to flow through `task next`, the question and review machinery, and the agent skill. A `kind` column reuses all of that. The alternative, a separate `services` table that spawns run records, would duplicate recurrence, the agent fields and the queue logic. It would also need its own notes and instructions linking. We rejected it.
- New columns are `kind TEXT` (NULL means `task`) and `paused INTEGER` (NULL or 0 means active). They are added with the existing `ALTER TABLE … .ok()` pattern in `open_conn`.
- Invariant, checked on add, edit and convert: `kind = service` requires `recurrence IS NOT NULL` and `due_date IS NOT NULL`. A due date is required because #34 put undated tasks at the top of `task next`. An undated service would come up on every run.

**D2. Runs go in a `service_runs` table.**
```
CREATE TABLE IF NOT EXISTS service_runs (
    id            INTEGER PRIMARY KEY,
    task_id       INTEGER NOT NULL,
    agent         TEXT,
    scheduled_for TEXT NOT NULL,   -- the due date this run satisfied
    ran_at        TEXT NOT NULL,   -- RFC 3339 UTC
    result        TEXT NOT NULL,   -- 'ok' | 'failed'
    summary       TEXT
);
CREATE INDEX IF NOT EXISTS service_runs_task ON service_runs(task_id, ran_at);
```
- `write_all` only clears `tasks`, so runs survive every commit. `task rm` on a service also deletes its runs, in the same transaction. To make that possible, `TaskTxn` exposes its connection to a `db::insert_run` / `db::delete_runs` helper used before `commit`.
- There is no foreign key, because `write_all` deletes and reinserts the task rows and a cascade would wipe the runs.
- History is kept forever. Rows are small, and if needed, retention can be a later change.

**D3. `task service run` is the only way an agent advances a service.**
`task service run <id> --result ok|failed [--summary "<text>"]`, in a single `TaskTxn`:
1. Check that the task is a service, is open and is not paused, and that the caller's identity (`TASK_AGENT` / cwd) matches the task's agent. A human without an agent identity may also record a run, for example after running it by hand.
2. Insert a `service_runs` row with `scheduled_for` set to the current due date.
3. Set `due_date = next_due_after(recur, due, today)`, which keeps applying `next_due_date` until the result is after today. This is the skip-ahead rule: one run per wake-up, however many days were missed.
4. If the result is `ok`, clear `work_status`. If it is `failed`, set `work_status = failing`.
5. Set `updated`.

The output is one line naming the result and the next run date. `--json` returns the run row and the new due date.

**D4. The `failing` status.**
`WorkStatus::Failing` serializes as `failing` (aliases `fail`, `failed`). It applies only to services. Setting it by hand on a normal task is rejected.
- `eligible()` keeps listing a failing service when it is due, so it retries on its next scheduled date. The human sees the failing badge on the Services screen and in the nav count. A failure does not hold the schedule. Holding would turn into needs-input under another name, which the human chose against.
- The next `ok` run clears `failing`.

**D5. How services appear in `task next`.**
`eligible()` adds `!(kind == service && paused)`. The existing filters already handle "not due yet", because a service always recurs and has a due date. `pickup_kind` returns `ServiceRun` (`"service_run"`) for a service whose `work_status` is none or `failing`. A service in `in-progress` still gets `Continue`, and the question and review pickups win as they do today. The existing rule that unassigned tasks never get auto-worked still applies first.

**D6. Pause and resume.**
`task service pause <id>` sets `paused = 1`. `task service resume <id>` clears it. If the due date is then before today, resume moves it to the first scheduled date on or after today, using the same stepping as D3 but with `>=`. So a service resumed on its scheduled day runs that day. Neither command writes a run row.

**D7. Keeping services out of normal lists.**
A shared helper, `Task::is_service()`, filters services out of `task list`, `task due`, the TUI views and `GET /api/tasks`. `task list --services` and `task service list` show them. `task show <id>` still works on a service and adds the kind, paused and last-run lines. `task done <id>` on a service fails with "use `task service pause` or `task rm`". `task reopen` is irrelevant, since a service is never done.

**D8. Web UI and API.**
- `GET /api/services` returns each service as a task plus `last_run` (the newest run row) and `run_count`.
- `GET /api/services/:id/runs?limit=&before=` returns runs newest first.
- `POST /api/services` creates a service. `POST /api/services/:id/pause` and `/resume` do what the CLI does.
- Schedule and instruction edits reuse `PATCH /api/tasks/:id` (`recur`, `due`, `instructions`), with the D1 invariant enforced on the server.
- Run recording stays CLI-only. Agents run the CLI, and there is no "run now" button in this change.
- The `/api/events` SSE stream already fires on every DB version bump, and recording a run commits a version bump, so the screen refreshes live.
- The Services screen is a new top-level view in the nav, with a count badge for failing services. It shows a table (title, agent, schedule text, next run, last run time and result, active/paused/failing state) and a detail pane (instructions editor, schedule editor, pause/resume button, run history list with summaries).

## Risks / Trade-offs

- **Older binaries.** A `task` binary from before this change would show services in its lists and allow `task done` on them. That is acceptable: it is a single-user tool that updates through the managed release. The new columns are nullable, so older binaries keep working.
- **`write_all` rewrites every row.** Adding `kind`/`paused` to `row_to_task` and `insert_task_row` is required, because otherwise each commit erases them. Unit tests must round-trip both fields.
- **The agent acts as its own reviewer.** A broken service can keep reporting `ok`. Mitigation: summaries are visible in run history, and the human can pause the service. The human accepted this when they declined per-run review.

## Migration Plan

This change is additive: two nullable columns, one new table and one new enum value. There is no data migration. Rollback means not using `kind = service`. The leftover columns and table are harmless.

## Open Questions

- Should `task service run` refuse a run recorded long before the due date, for example when an agent runs a weekly service twice in one day? The proposal: allow it only when `due <= today`, the same check `task next` uses. Otherwise error with "not due until <date>". Implemented that way unless the human says otherwise.
