## ADDED Requirements

### Requirement: Services API
The web server SHALL provide:
- `GET /api/services`: every service, with its `last_run` (or null) and `run_count`
- `POST /api/services`: create a service. The same rules as `task service add` apply.
- `POST /api/services/:id/pause` and `POST /api/services/:id/resume`
- `GET /api/services/:id/runs?limit=&before=`: runs, newest first

`PATCH /api/tasks/:id` SHALL enforce the service rules when the task is a service. `GET /api/tasks` SHALL exclude services.

#### Scenario: Task list API excludes services
- **WHEN** a client calls `GET /api/tasks`
- **THEN** no task with `kind = service` SHALL be returned

#### Scenario: Clearing a service's recurrence rejected
- **WHEN** a client sends `PATCH /api/tasks/40` with `recur: null` for a service
- **THEN** the server SHALL respond 400, and the service SHALL be unchanged

### Requirement: Services screen
The web UI SHALL have a Services screen, reachable from the main navigation. The nav entry SHALL show a badge with the number of failing services when that number is above zero. The screen SHALL list every service with its title, agent, schedule, next run date, last run time and result, and its state (active, paused or failing).

#### Scenario: Failing badge
- **WHEN** one service has `work_status = failing`
- **THEN** the Services nav entry SHALL show a badge of 1, and that service's row SHALL show the failing state

### Requirement: Managing a service
Selecting a service SHALL open a detail pane where the user can pause or resume it, change its recurrence and next run date, edit its instructions, and view its run history with summaries. The screen SHALL refresh when the server's change event fires.

#### Scenario: Pausing from the screen
- **WHEN** the user clicks Pause on an active service
- **THEN** the service SHALL show as paused, and `task next` SHALL no longer include it

#### Scenario: Live run update
- **WHEN** an agent records a run while the Services screen is open
- **THEN** the row's last run and next run SHALL update without a manual reload
