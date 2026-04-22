## ADDED Requirements

### Requirement: Agent instruction note storage
Agent instruction notes SHALL be stored as individual markdown files at `<task-dir>/Notes/Instructions/<agent-name>.md`. The `Notes/Instructions/` directory SHALL be created automatically on first write. Instruction notes SHALL NOT appear in the TUI Notes view (which scans only the immediate `Notes/` directory, not subdirectories).

#### Scenario: Instruction file path is deterministic
- **WHEN** an instruction note is written for agent `command-center`
- **THEN** the file SHALL be created at `<task-dir>/Notes/Instructions/command-center.md`

#### Scenario: Instructions directory auto-created
- **WHEN** no `Notes/Instructions/` directory exists and an instruction note is written
- **THEN** the directory SHALL be created before the file is written

#### Scenario: Instruction notes excluded from Notes view
- **WHEN** the TUI Notes view lists notes by scanning `<task-dir>/Notes/`
- **THEN** files in `Notes/Instructions/` SHALL NOT appear (subdirectories are not scanned)

### Requirement: `task agent instructions` CLI subcommand
The CLI SHALL provide a `task agent instructions <name>` subcommand for creating and reading agent instruction notes. The subcommand SHALL support `show` (print current instructions) and `edit` (create or replace instructions with `--title` and/or `--body` flags).

#### Scenario: Show instructions for an agent
- **WHEN** the user runs `task agent instructions command-center show`
- **THEN** the system SHALL print the title and body of `Notes/Instructions/command-center.md`, or print "No instructions found" if the file does not exist

#### Scenario: Create or replace instructions
- **WHEN** the user runs `task agent instructions command-center edit --body "Focus on Rust best practices"`
- **THEN** the system SHALL write `Notes/Instructions/command-center.md` with that body and exit with code 0

#### Scenario: Edit title only
- **WHEN** the user runs `task agent instructions command-center edit --title "Command Center Instructions"`
- **THEN** the system SHALL update the title of the instruction note, preserving the existing body

#### Scenario: Agent name with spaces is slugified
- **WHEN** the agent name contains spaces or uppercase letters
- **THEN** the filename SHALL be lowercased with spaces replaced by hyphens
