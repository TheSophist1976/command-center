## MODIFIED Requirements

### Requirement: Note file format
Each note SHALL be stored as an individual markdown file in the `Notes/` subdirectory of the task file's parent directory. The first line of the file SHALL be a `# Title` heading. All subsequent lines SHALL be the note body content. The file extension SHALL be `.md`.

#### Scenario: Note file structure
- **WHEN** a note with title "Meeting Notes" and body "Discussed roadmap.\nAction items below." is saved
- **THEN** the file content SHALL be `# Meeting Notes\n\nDiscussed roadmap.\nAction items below.`

### Requirement: Note slug naming
Note filenames SHALL be derived from the title using a slugification algorithm: lowercase, replace spaces with hyphens, remove non-alphanumeric characters (except hyphens), collapse consecutive hyphens. The file SHALL be named `<slug>.md` inside the `Notes/` subdirectory.

#### Scenario: Title to slug conversion
- **WHEN** a note is created with title "My Project Plan!"
- **THEN** the file SHALL be stored at `Notes/my-project-plan.md`

#### Scenario: Slug collision avoidance
- **WHEN** a note is created with a title that produces a slug matching an existing note file in `Notes/`
- **THEN** the system SHALL append a numeric suffix (e.g., `Notes/my-note-2.md`) to produce a unique filename

### Requirement: Note directory auto-creation
The system SHALL automatically create the `Notes/` subdirectory if it does not exist when writing a note.

#### Scenario: First note creates directory
- **WHEN** no `Notes/` directory exists and a note is written
- **THEN** the `Notes/` directory SHALL be created and the note file SHALL be written inside it

### Requirement: Note discovery
The system SHALL discover notes by scanning the `Notes/` subdirectory of the task file directory for `*.md` files. Each discovered file SHALL be parsed to extract the title from the first `# ` heading line.

#### Scenario: List notes in Notes directory
- **WHEN** `Notes/` contains `meeting-notes.md` and `project-plan.md`
- **THEN** the note discovery SHALL return two notes

#### Scenario: Notes directory absent returns empty list
- **WHEN** no `Notes/` directory exists
- **THEN** note discovery SHALL return an empty list without error

### Requirement: Note read and write
The system SHALL provide functions to read a note file (returning title and body) and write a note file (given title and body content). Writing SHALL use atomic file operations and SHALL create the `Notes/` directory if needed.

#### Scenario: Read note from Notes directory
- **WHEN** `Notes/meeting-notes.md` contains `# Meeting Notes\n\nKey decisions made.`
- **THEN** reading it SHALL return title "Meeting Notes" and body "Key decisions made."

#### Scenario: Write note to Notes directory
- **WHEN** a note with title "Daily Log" and body "Today was productive." is written
- **THEN** the file `Notes/daily-log.md` SHALL be created with content `# Daily Log\n\nToday was productive.`
