## ADDED Requirements

### Requirement: Markdown file format
The system SHALL store tasks in a single Markdown file (`tasks.md` by default). The file SHALL use H2 headings with checkbox syntax for each task, with metadata stored in HTML comments immediately following the heading.

#### Scenario: Well-formed task file
- **WHEN** the file contains tasks in the expected format
- **THEN** the parser SHALL produce a list of task records with all metadata fields populated

#### Scenario: Empty file
- **WHEN** the task file exists but contains no tasks
- **THEN** the parser SHALL return an empty task list without error

#### Scenario: File does not exist
- **WHEN** the specified task file does not exist and a read operation is attempted
- **THEN** the system SHALL return an empty task list without error

### Requirement: Task heading format
Each task SHALL be represented as an H2 heading with a checkbox prefix: `## [ ] <title>` for open tasks and `## [x] <title>` for completed tasks.

#### Scenario: Open task heading
- **WHEN** a task heading reads `## [ ] Build the login page`
- **THEN** the parser SHALL extract title "Build the login page" with status "open"

#### Scenario: Completed task heading
- **WHEN** a task heading reads `## [x] Set up CI pipeline`
- **THEN** the parser SHALL extract the title with status "done"

### Requirement: Task metadata in HTML comments
Each task SHALL store metadata in an HTML comment on the line immediately after the heading. The comment SHALL use space-delimited `key:value` pairs: `id`, `priority`, `tags`, `created`, `updated`.

#### Scenario: Full metadata comment
- **WHEN** a metadata comment reads `<!-- id:3 priority:high tags:frontend,auth created:2025-01-15T10:00:00Z -->`
- **THEN** the parser SHALL extract id=3, priority="high", tags=["frontend","auth"], and the created timestamp

#### Scenario: Missing optional fields
- **WHEN** a metadata comment omits `tags` or `updated`
- **THEN** the parser SHALL use empty defaults for missing optional fields (empty tag list, no updated timestamp)

### Requirement: Task description body
A task MAY have a description body consisting of any Markdown content between its metadata comment and the next H2 heading.

#### Scenario: Task with description
- **WHEN** there is text between a task's metadata comment and the next `## ` heading
- **THEN** the parser SHALL capture that text as the task's description

#### Scenario: Task without description
- **WHEN** there is no text between a task's metadata comment and the next `## ` heading
- **THEN** the task's description SHALL be empty

### Requirement: File header with next-id counter
The task file SHALL contain a header comment `<!-- next-id:N -->` tracking the next available integer ID. When a new task is added, the system SHALL assign it the current next-id value and increment the counter.

#### Scenario: Adding a task increments the counter
- **WHEN** the header reads `<!-- next-id:5 -->` and a new task is added
- **THEN** the new task SHALL receive id=5 and the header SHALL update to `<!-- next-id:6 -->`

#### Scenario: File without header
- **WHEN** the task file exists but has no `<!-- next-id:N -->` comment
- **THEN** the system SHALL scan existing tasks to determine the max id and set next-id to max+1

### Requirement: Format version header
The task file SHALL contain a format version comment `<!-- format:N -->` in the header. The parser SHALL accept format version 1 and format version 2 files. Files with any other version SHALL be treated as unsupported.

#### Scenario: Format version 1 accepted
- **WHEN** the file contains `<!-- format:1 -->`
- **THEN** the parser SHALL proceed normally

#### Scenario: Format version 2 accepted
- **WHEN** the file contains `<!-- format:2 -->`
- **THEN** the parser SHALL proceed normally

#### Scenario: Missing format version
- **WHEN** the file has no format version comment
- **THEN** the parser SHALL assume format version 1

#### Scenario: Unsupported format version
- **WHEN** the file contains a format version other than 1 or 2
- **THEN** the parser SHALL emit a warning in tolerant mode and return an error in strict mode

### Requirement: Tolerant parsing
The parser SHALL skip malformed task sections rather than failing. A task section with an unparseable heading or missing metadata SHALL be ignored in default mode.

#### Scenario: Malformed heading
- **WHEN** an H2 heading does not match the checkbox pattern (e.g., `## Some random heading`)
- **THEN** the parser SHALL skip that section and continue parsing subsequent tasks

#### Scenario: Missing metadata comment
- **WHEN** a valid task heading has no metadata comment on the following line
- **THEN** the parser SHALL skip that task in default mode

#### Scenario: Strict mode
- **WHEN** the `--strict` flag is active and a malformed section is encountered
- **THEN** the system SHALL report an error describing the issue

### Requirement: File-level locking
The system SHALL acquire an advisory file lock before writing to the task file and release it after the write completes.

#### Scenario: Concurrent write attempt
- **WHEN** another process holds the lock on the task file
- **THEN** the system SHALL wait briefly and retry, failing with an error if the lock cannot be acquired

### Requirement: Atomic writes
The system SHALL write changes to a temporary file and rename it over the original to prevent corruption from interrupted writes.

#### Scenario: Interrupted write
- **WHEN** the process is killed during a write operation
- **THEN** the original task file SHALL remain intact (the temp file may be left behind)

### Requirement: Due date and project metadata keys
The task metadata comment SHALL support two new optional keys: `due` (a date string in `YYYY-MM-DD` format) and `project` (a string). Both keys are omitted from the comment when the corresponding field is `None`.

#### Scenario: Task with due and project serialized
- **WHEN** a task with `due_date: 2025-06-01` and `project: "Work"` is saved
- **THEN** the metadata comment SHALL contain `due:2025-06-01 project:Work`

#### Scenario: Task without due or project
- **WHEN** a task has no `due_date` and no `project`
- **THEN** the metadata comment SHALL not contain `due` or `project` keys

#### Scenario: Parse due and project keys
- **WHEN** a metadata comment contains `due:2025-06-01 project:Work`
- **THEN** the parser SHALL set `due_date` to 2025-06-01 and `project` to `"Work"`

#### Scenario: Missing optional keys parse gracefully
- **WHEN** a metadata comment does not contain `due` or `project`
- **THEN** the parser SHALL set both fields to `None` without error

### Requirement: Auto-migration from format version 1 to version 2
When a format:1 file is loaded, the system SHALL parse it successfully (new fields `due_date` and `project` default to `None`). On the next write operation (any mutation), the file SHALL be written with `<\!-- format:2 -->` and any new fields serialized as appropriate.

#### Scenario: Format 1 file loads without error
- **WHEN** a file with `<\!-- format:1 -->` is loaded
- **THEN** the system SHALL parse it successfully with `due_date` and `project` defaulting to `None` for all tasks

#### Scenario: Format 1 file upgraded on next save
- **WHEN** a task in a format:1 file is mutated and saved
- **THEN** the saved file SHALL contain `<\!-- format:2 -->` in the header

### Requirement: Explicit migration with task migrate
The system SHALL provide a `task migrate` subcommand that loads the current task file and saves it back with format:2 headers without modifying any task data.

#### Scenario: Migrate upgrades format version
- **WHEN** the user runs `task migrate` on a format:1 file
- **THEN** the saved file SHALL contain `<\!-- format:2 -->` and all task data SHALL be unchanged

#### Scenario: Migrate on already-format-2 file
- **WHEN** the user runs `task migrate` on a file already at format:2
- **THEN** the system SHALL re-save the file (no-op from a data perspective) and output a confirmation
