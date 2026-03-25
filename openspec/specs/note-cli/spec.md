### Requirement: Note list command
The CLI SHALL provide a `task note list` subcommand that prints all notes in the task directory, one per line, in the format `<slug>  <title>`.

#### Scenario: Notes exist
- **WHEN** the user runs `task note list` and notes exist in the task directory
- **THEN** the system SHALL print each note as `<slug>  <title>` on its own line, sorted alphabetically by slug

#### Scenario: No notes exist
- **WHEN** the user runs `task note list` and no notes exist
- **THEN** the system SHALL print nothing and exit with code 0

### Requirement: Note add command
The CLI SHALL provide a `task note add <title>` subcommand that creates a new note with the given title and an empty body, then prints the path of the created file.

#### Scenario: Create note with title
- **WHEN** the user runs `task note add "Meeting Notes"`
- **THEN** the system SHALL create `meeting-notes.md` in the task directory with content `# Meeting Notes\n` and print the file path

#### Scenario: Slug collision avoidance
- **WHEN** the user runs `task note add "My Note"` and `my-note.md` already exists
- **THEN** the system SHALL create `my-note-2.md` and print its path

### Requirement: Note show command
The CLI SHALL provide a `task note show <slug>` subcommand that prints the full content of the note (title and body) to stdout.

#### Scenario: Show existing note
- **WHEN** the user runs `task note show meeting-notes`
- **THEN** the system SHALL print the raw markdown content of `meeting-notes.md` to stdout

#### Scenario: Show nonexistent note
- **WHEN** the user runs `task note show nonexistent-slug`
- **THEN** the system SHALL print an error message to stderr and exit with code 1

### Requirement: Note edit command
The CLI SHALL provide a `task note edit <slug>` subcommand with `--title` and `--body` flags that update the specified fields of an existing note.

#### Scenario: Edit title
- **WHEN** the user runs `task note edit my-note --title "New Title"`
- **THEN** the system SHALL update the first line of `my-note.md` to `# New Title` and print the file path

#### Scenario: Edit body
- **WHEN** the user runs `task note edit my-note --body "Updated content"`
- **THEN** the system SHALL replace the body of `my-note.md` with "Updated content" and print the file path

#### Scenario: Edit nonexistent note
- **WHEN** the user runs `task note edit nonexistent --title "X"`
- **THEN** the system SHALL print an error message to stderr and exit with code 1

#### Scenario: Edit with no flags
- **WHEN** the user runs `task note edit my-note` with no flags
- **THEN** the system SHALL print a usage error and exit with code 1

### Requirement: Note add with task link
The `task note add <title>` subcommand SHALL accept an optional `--task <id>` flag. When provided, after creating the note file the system SHALL load the task file, set the matching task's `note` field to the new note's slug, and save the task file.

#### Scenario: Create note linked to task
- **WHEN** the user runs `task note add "Meeting Notes" --task 42`
- **THEN** the system SHALL create `meeting-notes.md`, set `note:meeting-notes` on task 42, and print the file path

#### Scenario: Task not found during add
- **WHEN** the user runs `task note add "Meeting Notes" --task 999` and task 999 does not exist
- **THEN** the note file SHALL still be created, the system SHALL print a warning that task 999 was not found, and exit with code 1

### Requirement: Note link command
The CLI SHALL provide a `task note link <slug> <task-id>` subcommand that sets the `note` field on the specified task to the given slug.

#### Scenario: Link existing note to task
- **WHEN** the user runs `task note link meeting-notes 42`
- **THEN** task 42's `note` field SHALL be set to `meeting-notes` and the system SHALL print a confirmation

#### Scenario: Link overwrites existing note link
- **WHEN** the user runs `task note link new-notes 42` and task 42 already has `note:old-notes`
- **THEN** task 42's `note` field SHALL be updated to `new-notes`

#### Scenario: Task not found on link
- **WHEN** the user runs `task note link meeting-notes 999` and task 999 does not exist
- **THEN** the system SHALL print an error message to stderr and exit with code 1

### Requirement: Note unlink command
The CLI SHALL provide a `task note unlink <task-id>` subcommand that clears the `note` field on the specified task.

#### Scenario: Unlink note from task
- **WHEN** the user runs `task note unlink 42` and task 42 has `note:meeting-notes`
- **THEN** task 42's `note` field SHALL be cleared and the system SHALL print a confirmation

#### Scenario: Unlink on task with no note
- **WHEN** the user runs `task note unlink 42` and task 42 has no note linked
- **THEN** the system SHALL succeed and print a confirmation (idempotent)

#### Scenario: Task not found on unlink
- **WHEN** the user runs `task note unlink 999` and task 999 does not exist
- **THEN** the system SHALL print an error message to stderr and exit with code 1

### Requirement: Note rm command
The CLI SHALL provide a `task note rm <slug>` subcommand that deletes the note file with the given slug.

#### Scenario: Delete existing note
- **WHEN** the user runs `task note rm meeting-notes`
- **THEN** the system SHALL delete `meeting-notes.md` and print a confirmation message

#### Scenario: Delete nonexistent note
- **WHEN** the user runs `task note rm nonexistent`
- **THEN** the system SHALL print an error message to stderr and exit with code 1
