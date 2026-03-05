## ADDED Requirements

### Requirement: Note file format
Each note SHALL be stored as an individual markdown file in the same directory as the task file. The first line of the file SHALL be a `# Title` heading. All subsequent lines SHALL be the note body content. The file extension SHALL be `.md`.

#### Scenario: Note file structure
- **WHEN** a note with title "Meeting Notes" and body "Discussed roadmap.\nAction items below." is saved
- **THEN** the file content SHALL be `# Meeting Notes\n\nDiscussed roadmap.\nAction items below.`

### Requirement: Note slug naming
Note filenames SHALL be derived from the title using a slugification algorithm: lowercase, replace spaces with hyphens, remove non-alphanumeric characters (except hyphens), collapse consecutive hyphens. The file SHALL be named `<slug>.md`.

#### Scenario: Title to slug conversion
- **WHEN** a note is created with title "My Project Plan!"
- **THEN** the filename SHALL be `my-project-plan.md`

#### Scenario: Slug collision avoidance
- **WHEN** a note is created with a title that produces a slug matching an existing note file
- **THEN** the system SHALL append a numeric suffix (e.g., `my-note-2.md`) to produce a unique filename

### Requirement: Note discovery
The system SHALL discover notes by scanning the task file directory for `*.md` files, excluding the task file itself (e.g., `tasks.md`) and any files in subdirectories or backup directories. Each discovered file SHALL be parsed to extract the title from the first `# ` heading line.

#### Scenario: List notes in directory
- **WHEN** the task directory contains `tasks.md`, `meeting-notes.md`, and `project-plan.md`
- **THEN** the note discovery SHALL return two notes: "meeting-notes" and "project-plan"

#### Scenario: Task file excluded from notes
- **WHEN** the task file is named `tasks.md`
- **THEN** it SHALL NOT appear in the notes list

### Requirement: Note read and write
The system SHALL provide functions to read a note file (returning title and body) and write a note file (given title and body content). Writing SHALL use atomic file operations consistent with the existing task file storage pattern.

#### Scenario: Read note from file
- **WHEN** a file `meeting-notes.md` contains `# Meeting Notes\n\nKey decisions made.`
- **THEN** reading it SHALL return title "Meeting Notes" and body "Key decisions made."

#### Scenario: Write note to file
- **WHEN** a note with title "Daily Log" and body "Today was productive." is written
- **THEN** the file `daily-log.md` SHALL be created with content `# Daily Log\n\nToday was productive.`
