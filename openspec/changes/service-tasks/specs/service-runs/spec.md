## ADDED Requirements

### Requirement: Service run storage
The database SHALL have a `service_runs` table with columns `id`, `task_id`, `agent`, `scheduled_for`, `ran_at`, `result` (`ok` or `failed`) and `summary`. Run rows SHALL NOT be removed or changed by the task-table rewrite that happens on each commit.

#### Scenario: Runs survive unrelated edits
- **WHEN** a service has three recorded runs and another task is then edited
- **THEN** all three runs SHALL still exist, unchanged

#### Scenario: Table created on open
- **WHEN** a database from before this change is opened
- **THEN** the `service_runs` table SHALL be created, and existing tasks SHALL load unchanged

### Requirement: Run history
`task service runs <id> [--limit N] [--json]` SHALL list the service's runs, newest first, showing the run time, scheduled date, result and summary. The default limit SHALL be 20.

#### Scenario: Listing runs
- **WHEN** a service has 25 runs and the user runs `task service runs 40`
- **THEN** the 20 newest runs SHALL be printed, newest first

#### Scenario: JSON output
- **WHEN** the user runs `task service runs 40 --json`
- **THEN** the output SHALL be a JSON array of run objects with the fields `id`, `task_id`, `agent`, `scheduled_for`, `ran_at`, `result` and `summary`
