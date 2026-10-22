## ADDED Requirements

### Requirement: Service kind
A task SHALL have a `kind` of `task` (the default) or `service`, and a `paused` flag (default false). A task with `kind = service` SHALL have a recurrence and a due date. Its due date SHALL be the date of its next scheduled run.

#### Scenario: Creating a service
- **WHEN** the user runs `task service add "Sync inbox" --agent Home --recur daily --due 2026-10-23`
- **THEN** a task SHALL be created with `kind = service`, `paused = false`, recurrence `daily` and due date 2026-10-23

#### Scenario: Service without recurrence rejected
- **WHEN** the user runs `task service add "Sync inbox" --agent Home --due 2026-10-23` without `--recur`
- **THEN** the command SHALL fail with an error that services must recur, and no task SHALL be created

#### Scenario: Service without due date rejected
- **WHEN** a service would be saved without a due date, by add, by `task edit --service`, or by `task edit --due ""` on a service
- **THEN** the command SHALL fail and the task SHALL be unchanged

#### Scenario: Converting a task
- **WHEN** the user runs `task edit 12 --service` on a recurring task that has a due date
- **THEN** task 12 SHALL become a service and keep its id, agent, instructions and notes

#### Scenario: Kind and paused survive a commit
- **WHEN** any other task is edited after a service exists
- **THEN** the service SHALL still have `kind = service` and its `paused` value

### Requirement: Recording a service run
`task service run <id> --result ok|failed [--summary "<text>"]` SHALL, in one transaction, insert a run record and advance the service's due date to the first date in its recurrence that is after today. A result of `ok` SHALL clear `work_status`, and a result of `failed` SHALL set `work_status` to `failing`. The command SHALL fail for non-service tasks, paused services, services not yet due, and callers whose agent identity does not match the task's agent.

#### Scenario: Successful run advances one step
- **WHEN** today is 2026-10-22, a daily service is due 2026-10-22, and its agent runs `task service run 40 --result ok --summary "12 items synced"`
- **THEN** a run SHALL be recorded with `scheduled_for = 2026-10-22`, `result = ok` and the summary
- **AND** the due date SHALL become 2026-10-23
- **AND** `work_status` SHALL be cleared

#### Scenario: Missed runs are skipped
- **WHEN** today is 2026-10-22, a daily service is due 2026-10-19, and a run is recorded
- **THEN** exactly one run SHALL be recorded, with `scheduled_for = 2026-10-19`
- **AND** the due date SHALL become 2026-10-23

#### Scenario: Failed run
- **WHEN** a run is recorded with `--result failed`
- **THEN** `work_status` SHALL be `failing`
- **AND** the due date SHALL still advance to the next scheduled date

#### Scenario: Recovery clears failing
- **WHEN** a `failing` service records an `ok` run
- **THEN** `work_status` SHALL be cleared

#### Scenario: Not a service
- **WHEN** `task service run` targets a task with `kind = task`
- **THEN** the command SHALL fail, and no run SHALL be recorded

#### Scenario: Not due yet
- **WHEN** a service is due 2026-10-25 and today is 2026-10-22
- **THEN** `task service run` SHALL fail with "not due until 2026-10-25"

### Requirement: Failing work status
`work_status` SHALL accept `failing` (aliases `fail`, `failed`), and only on services. Setting it by hand on a normal task SHALL be rejected.

#### Scenario: Failing on a normal task rejected
- **WHEN** the user runs `task edit 5 --work-status failing` on a task with `kind = task`
- **THEN** the command SHALL fail

### Requirement: Pause and resume
`task service pause <id>` SHALL set `paused = true`. `task service resume <id>` SHALL clear it. If the due date is before today, resume SHALL also move it to the first date in the recurrence that is on or after today. Pause and resume SHALL NOT record runs.

#### Scenario: Resume after a long pause
- **WHEN** today is Thursday 2026-10-22, a `weekly:mon` service is due Monday 2026-10-05, and it is resumed
- **THEN** the due date SHALL become Monday 2026-10-26

#### Scenario: Resume on the scheduled day
- **WHEN** a daily service due 2026-10-20 is resumed on 2026-10-22
- **THEN** the due date SHALL become 2026-10-22

### Requirement: Services in the agent queue
`task next` SHALL include an active (not paused) service assigned to the agent once its due date is on or before today. It SHALL report `pickup: "service_run"` when the service's `work_status` is unset or `failing`. Paused services SHALL never be included. The existing `in-progress`, review-feedback and answered-question pickups SHALL take precedence for services too.

#### Scenario: Due service appears
- **WHEN** a service assigned to Home is due today and not paused
- **THEN** `task next --json` for Home SHALL include it with `pickup` `service_run`

#### Scenario: Paused service hidden
- **WHEN** that service is paused
- **THEN** `task next` SHALL NOT include it

#### Scenario: Failing service retries on schedule
- **WHEN** a `failing` service reaches its next due date
- **THEN** `task next` SHALL include it with `pickup` `service_run`

### Requirement: Services excluded from normal task views
`task list`, `task due`, the TUI task views and `GET /api/tasks` SHALL exclude services. `task list --services` and `task service list` SHALL list only services. `task show <id>` SHALL work for services and SHALL show kind, paused state and the last run. `task done` on a service SHALL fail and point to `task service pause` or `task rm`.

#### Scenario: Normal list hides services
- **WHEN** the user runs `task list` and one of the tasks is a service
- **THEN** the service SHALL NOT appear in the output

#### Scenario: Done refused
- **WHEN** the user runs `task done 40` on a service
- **THEN** the command SHALL fail, and the task SHALL stay open with the same due date

#### Scenario: Removing a service removes its runs
- **WHEN** the user runs `task rm 40` on a service with recorded runs
- **THEN** the task and all of its `service_runs` rows SHALL be deleted
