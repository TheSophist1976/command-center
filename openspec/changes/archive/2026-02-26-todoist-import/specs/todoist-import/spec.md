## ADDED Requirements

### Requirement: Fetch open Todoist tasks
The system SHALL call the Todoist REST API v2 `GET /tasks` endpoint (filtered to open tasks) using the stored access token and return the full task list.

#### Scenario: Successful fetch
- **WHEN** the user runs `task import todoist` and a valid token is stored
- **THEN** the system SHALL retrieve all open tasks from Todoist and proceed to import

#### Scenario: No token stored
- **WHEN** the user runs `task import todoist` and no access token exists at the configured path
- **THEN** the system SHALL exit with code 1 and display a message directing the user to run `task auth todoist`

#### Scenario: API authentication failure
- **WHEN** the stored token is expired or invalid and the API returns 401
- **THEN** the system SHALL exit with code 1 and display an error indicating the token is invalid and should be refreshed via `task auth todoist`

### Requirement: Skip already-exported tasks
The system SHALL skip any Todoist task that already has the `exported` label. This makes the import operation idempotent across multiple runs.

#### Scenario: Skip task with exported label
- **WHEN** a Todoist task has the label `exported`
- **THEN** the system SHALL skip that task and NOT append it to the local task file

#### Scenario: Import task without exported label
- **WHEN** a Todoist task does not have the label `exported`
- **THEN** the system SHALL import that task into the local task file

### Requirement: Map Todoist fields to local Task
The system SHALL map Todoist task fields to the local `Task` struct according to the following rules: `content` → `title`, `description` → `description`, `priority` (1–4) → `Priority` (`critical`/`high`/`medium`/`low`), `labels` → `tags` (merged with an `imported` tag), `due.date` → `due_date`, resolved project name → `project`.

#### Scenario: Priority mapping
- **WHEN** a Todoist task has `priority: 1`
- **THEN** the imported task SHALL have priority `critical`

#### Scenario: Priority mapping P4
- **WHEN** a Todoist task has `priority: 4`
- **THEN** the imported task SHALL have priority `low`

#### Scenario: Labels become tags
- **WHEN** a Todoist task has labels `["work", "backend"]`
- **THEN** the imported task SHALL have tags `["work", "backend", "imported"]`

#### Scenario: Due date mapping
- **WHEN** a Todoist task has `due.date: "2025-06-15"`
- **THEN** the imported task SHALL have `due_date` set to 2025-06-15

#### Scenario: No due date
- **WHEN** a Todoist task has no `due` field
- **THEN** the imported task SHALL have `due_date` set to none

### Requirement: Resolve project names
The system SHALL call `GET /projects` once per import run to build a project-id-to-name map, then resolve each task's `project_id` to a project name string stored in the `project` field.

#### Scenario: Project name resolved
- **WHEN** a Todoist task has `project_id: "123"` and the projects response maps `"123"` to `"Work"`
- **THEN** the imported task SHALL have `project` set to `"Work"`

#### Scenario: Unknown project id
- **WHEN** a task's `project_id` does not appear in the projects response
- **THEN** the imported task SHALL have `project` set to none

### Requirement: Append imported tasks to local file
The system SHALL append all mapped tasks to the local task file, assigning sequential IDs starting from the file's current `next_id`, and save the file.

#### Scenario: Tasks appended
- **WHEN** the import runs and 5 Todoist tasks qualify for import
- **THEN** those 5 tasks SHALL be appended to the local task file and the file SHALL be saved

#### Scenario: All tasks already exported
- **WHEN** all Todoist tasks already have the `exported` label
- **THEN** no tasks SHALL be appended and the system SHALL output a message indicating 0 tasks were imported

### Requirement: Label imported Todoist tasks as exported
After successfully appending a task to the local file, the system SHALL add the `exported` label to that task in Todoist via `POST /tasks/{id}` with the updated labels list.

#### Scenario: Exported label applied
- **WHEN** a task is successfully appended to the local file
- **THEN** the system SHALL update that task in Todoist to add the `exported` label

#### Scenario: Labeling failure is non-fatal
- **WHEN** the API call to add `exported` label fails for a task
- **THEN** the system SHALL emit a warning for that task but continue processing remaining tasks

### Requirement: Test mode import
The system SHALL support a `--test` flag on `task import todoist` that limits the import to the first 3 qualifying tasks (those not already labeled `exported`) and skips applying the `exported` label back to Todoist. All other import behavior (field mapping, appending to local file, `imported` tag) SHALL remain the same in test mode.

#### Scenario: Test mode limits to 3 tasks
- **WHEN** the user runs `task import todoist --test` and 10 tasks qualify for import
- **THEN** only the first 3 SHALL be imported and the file SHALL be saved with those 3 tasks

#### Scenario: Test mode skips Todoist labeling
- **WHEN** the user runs `task import todoist --test`
- **THEN** the system SHALL NOT add the `exported` label to any task in Todoist

#### Scenario: Test mode output
- **WHEN** the import completes in test mode
- **THEN** the system SHALL output a summary indicating test mode was active (e.g., `[test mode] Imported 3 tasks (Todoist tasks not labeled)`)

#### Scenario: Test mode is repeatable
- **WHEN** the user runs `task import todoist --test` multiple times
- **THEN** the same tasks MAY be imported each time (no Todoist side-effects prevent re-import)

### Requirement: Import summary output
The system SHALL output a summary of the import operation including the number of tasks imported and the number skipped.

#### Scenario: Successful import summary
- **WHEN** the import completes in normal mode
- **THEN** the system SHALL output a line like `Imported 7 tasks, skipped 3 (already exported)`
