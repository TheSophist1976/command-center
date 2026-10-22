## 1. Data Model

- [ ] 1.1 Add `TaskKind { Task, Service }` and `paused: bool` to `Task` in `src/task.rs`, with `is_service()`
- [ ] 1.2 Add `WorkStatus::Failing` (`failing`, aliases `fail`/`failed`) with Display/FromStr tests
- [ ] 1.3 Add `next_due_after(recur, due, today, inclusive)`, which steps `next_due_date` until the date is after today (or on/after today when inclusive), with tests for daily/weekly:DAY/monthly:N:DAY gaps
- [ ] 1.4 Add a service validation helper: requires recurrence and due date, and allows `failing` only on services

## 2. Storage

- [ ] 2.1 Add the `kind` and `paused` columns in `open_conn` (the `ALTER TABLE … .ok()` pattern), and read and write them in `row_to_task`/`insert_task_row`
- [ ] 2.2 Create the `service_runs` table and index in `SCHEMA_SQL`
- [ ] 2.3 Add `db::insert_run`, `db::delete_runs`, `db::list_runs` and `db::last_run`. Let `TaskTxn` run the insert/delete inside its transaction.
- [ ] 2.4 Tests: kind/paused round-trip, runs survive `write_all`, rolling back a txn drops the run, opening a pre-change database

## 3. CLI

- [ ] 3.1 Add the `task service` group in `src/cli.rs`: `add`, `list`, `show`, `run`, `pause`, `resume`, `runs`
- [ ] 3.2 `task service run`: check the agent identity, kind, paused and due date, then insert the run, advance the due date, and set or clear `failing`. Support `--json`.
- [ ] 3.3 `task edit --service/--no-service`, with validation on every edit of a service
- [ ] 3.4 `task done` refuses services, and `task rm` deletes the runs
- [ ] 3.5 `task list`/`task due` exclude services. Add `task list --services`.
- [ ] 3.6 `task show` prints kind, paused and the last run for services

## 4. Agent Queue

- [ ] 4.1 `agent::eligible()` excludes paused services
- [ ] 4.2 Add `Pickup::ServiceRun` (`service_run`) to `pickup_kind`, after the in-progress, review and question checks
- [ ] 4.3 Tests: due, paused, failing and not-yet-due services, and an unassigned service never queued

## 5. TUI

- [ ] 5.1 Filter services out of every task view in `src/tui.rs`

## 6. Web API

- [ ] 6.1 `GET /api/tasks` excludes services
- [ ] 6.2 `GET/POST /api/services`, `POST /api/services/:id/pause|resume`, `GET /api/services/:id/runs`
- [ ] 6.3 `PATCH /api/tasks/:id` enforces the service rules

## 7. Web UI

- [ ] 7.1 Add the Services API client and types in `web/src/api.ts` and `types.ts`
- [ ] 7.2 Add a Services nav entry with a failing count badge
- [ ] 7.3 Services table: title, agent, schedule, next run, last run, state
- [ ] 7.4 Detail pane: pause/resume, schedule editor, instructions editor, run history
- [ ] 7.5 Refresh on SSE change events

## 8. Skills and Docs

- [ ] 8.1 `plugin/skills/work-agent-tasks/SKILL.md`: handle the `service_run` pickup (do the work, call `task service run`, never set waiting-for-review, never run `task done`)
- [ ] 8.2 `plugin/skills/task-manager/SKILL.md` and `skills/task-manager/SKILL.md`: document `task service`, the `failing` status, and that services are hidden from `task list`
- [ ] 8.3 `AGENTS.md`: the `kind`/`paused` fields, `failing`, and the service operations
