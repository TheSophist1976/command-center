## Why

Some agent tasks are routine, recurring work, such as syncing a source or refreshing a dashboard, that needs no human review. Today a recurring agent task stops at `waiting-for-review` until a human runs `task done`. Only then does it spawn the next occurrence. So a daily job stalls whenever nobody clicks it, and each cycle leaves another done row in the task list.

ServiceTasks lets this work run on its schedule, record each run, and roll forward on its own. It asks for a human only when a run fails. Services are managed on their own screen and never clutter the normal task lists.

## What Changes

- **Data model**: tasks gain a `kind` (`task`, the default, or `service`) and a `paused` flag. A service MUST have a recurrence and a due date, and its due date is the next scheduled run.
- **New `service_runs` table**: one row per run (task id, agent, the date the run was scheduled for, when it ran, result `ok`/`failed`, summary). It is separate from `tasks`, so the whole-table rewrite in `db::write_all` never touches it.
- **New work status `failing`**: a failed run sets it. It shows up as a badge in the UI, and the next `ok` run clears it.
- **CLI**: a new `task service` command group: `add`, `list`, `show`, `run`, `pause`, `resume`, `runs`. `task edit --service/--no-service` converts an existing task, after checking it is valid. `task done` refuses service tasks.
- **Recording a run**: `task service run <id> --result ok|failed [--summary "..."]` inserts a run row and advances the due date in the same transaction. A missed run is skipped, not caught up: the due date moves to the first scheduled date after today, so one run happens, not one per missed day.
- **`task next`**: includes due, active (not paused) service tasks for the agent with `pickup: "service_run"`. Paused services are never listed. A `failing` service is still listed when it is next due, so it retries on schedule.
- **Normal task lists exclude services**: `task list`, `task due`, the TUI lists and the web task list all leave out `kind = service`.
- **Web UI — new Services screen**: lists services (title, agent, schedule, next run, last run result, state). Pause and resume a service, change its schedule (recurrence and next run), edit its instructions, and browse its run history.
- **Agent skill**: `work-agent-tasks` (in `plugin/skills/` and `skills/`) learns the `service_run` pickup. The agent does the work, then calls `task service run`, and never sets `waiting-for-review`.
- **AGENTS.md / task-manager skill**: document the `kind` and `paused` fields, the `failing` status and the `task service` commands.

## Non-goals

- No per-cycle "review the next run" option. The human decided it is not needed.
- No one-shot services. Every service must recur.
- No catch-up runs for missed dates.
- No cron-style times of day. Scheduling stays at the date level of the existing recurrence patterns.
- No TUI services screen in this change. The TUI only hides services. Managing services is a web UI job.

## Capabilities

### New Capabilities
- `service-tasks`: the service kind, its rules, pause/resume, recording runs, skip-ahead scheduling, how services appear in `task next`, and keeping them out of normal lists.
- `service-runs`: the `service_runs` table, the run record and run history queries.
- `service-management-ui`: the web Services screen and its API.

## Impact

- `src/task.rs`: `TaskKind`, `paused`, `WorkStatus::Failing`, a `next_due_after(recur, due, today)` helper
- `src/db.rs`: `kind`/`paused` columns (the same `ALTER TABLE … .ok()` pattern), the `service_runs` table, run insert/query functions
- `src/agent.rs`: `eligible()` handles paused services, plus the `Pickup::ServiceRun` value
- `src/cli.rs`, `src/bin/task.rs`, `src/commands.rs`: the `task service` group, `--service` on edit, the `task done` guard, list filtering
- `src/tui.rs`: filter services out of the task views
- `src/server.rs`: `/api/services` routes, plus filtering services out of `/api/tasks`
- `web/src/`: a Services screen, run history panel, failing badge and nav entry
- `plugin/skills/work-agent-tasks/SKILL.md`, `plugin/skills/task-manager/SKILL.md`, `skills/task-manager/SKILL.md`, `AGENTS.md`
