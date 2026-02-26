## ADDED Requirements

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
When a format:1 file is loaded, the system SHALL parse it successfully (new fields `due_date` and `project` default to `None`). On the next write operation (any mutation), the file SHALL be written with `<!-- format:2 -->` and any new fields serialized as appropriate.

#### Scenario: Format 1 file loads without error
- **WHEN** a file with `<!-- format:1 -->` is loaded
- **THEN** the system SHALL parse it successfully with `due_date` and `project` defaulting to `None` for all tasks

#### Scenario: Format 1 file upgraded on next save
- **WHEN** a task in a format:1 file is mutated and saved
- **THEN** the saved file SHALL contain `<!-- format:2 -->` in the header

### Requirement: Explicit migration with task migrate
The system SHALL provide a `task migrate` subcommand that loads the current task file and saves it back with format:2 headers without modifying any task data.

#### Scenario: Migrate upgrades format version
- **WHEN** the user runs `task migrate` on a format:1 file
- **THEN** the saved file SHALL contain `<!-- format:2 -->` and all task data SHALL be unchanged

#### Scenario: Migrate on already-format-2 file
- **WHEN** the user runs `task migrate` on a file already at format:2
- **THEN** the system SHALL re-save the file (no-op from a data perspective) and output a confirmation

## MODIFIED Requirements

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
