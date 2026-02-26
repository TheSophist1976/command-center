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
The task file SHALL contain a format version comment `<!-- format:1 -->` in the header. The parser SHALL check this version and reject files with unsupported versions.

#### Scenario: Supported format version
- **WHEN** the file contains `<!-- format:1 -->`
- **THEN** the parser SHALL proceed normally

#### Scenario: Missing format version
- **WHEN** the file has no format version comment
- **THEN** the parser SHALL assume format version 1

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
