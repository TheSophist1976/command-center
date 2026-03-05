## ADDED Requirements

### Requirement: Task note field
The `Task` struct SHALL include an optional `note: Option<String>` field containing the slug of a linked note file. The field SHALL be serialized as a `note:<slug>` entry in the task metadata comment and omitted when `None`.

#### Scenario: Task with linked note
- **WHEN** a task has `note: Some("meeting-notes")`
- **THEN** the task metadata comment SHALL contain `note:meeting-notes`

#### Scenario: Task without linked note
- **WHEN** a task has `note: None`
- **THEN** the task metadata comment SHALL NOT contain a `note` key

#### Scenario: Round-trip preserves note link
- **WHEN** a task file with `note:meeting-notes` metadata is parsed and re-serialized
- **THEN** the note link SHALL be identical

### Requirement: Link note from task detail panel
The task detail panel SHALL display the linked note slug (or "none") as a read-only field. The user SHALL press `n` in Normal mode while a task is selected to attach or change a linked note. The system SHALL present a list of available notes for selection, or allow creating a new note that auto-links to the task.

#### Scenario: Attach note to task
- **WHEN** the user presses `n` on a task and selects "meeting-notes" from the note list
- **THEN** the task's `note` field SHALL be set to `Some("meeting-notes")`

#### Scenario: Create and link new note from task
- **WHEN** the user presses `n` on a task and chooses to create a new note
- **THEN** a new note SHALL be created and automatically linked to the task

#### Scenario: Clear note link
- **WHEN** the user presses `n` on a task and selects "none"
- **THEN** the task's `note` field SHALL be set to `None`

### Requirement: Navigate from task to linked note
The user SHALL press `g` (go to note) on a task with a linked note to open that note in the inline editor. If the linked note file does not exist, the system SHALL display an error message.

#### Scenario: Open linked note
- **WHEN** the user presses `g` on a task with `note: Some("meeting-notes")`
- **THEN** the TUI SHALL open `meeting-notes.md` in the inline note editor

#### Scenario: Linked note file missing
- **WHEN** the user presses `g` on a task whose linked note file has been deleted
- **THEN** the TUI SHALL display an error message indicating the note file was not found

### Requirement: Note link shown in task list
The task list view SHALL display an indicator (e.g., a note icon or marker) on tasks that have a linked note, providing visual feedback that additional context is available.

#### Scenario: Task with note shows indicator
- **WHEN** a task has a non-None `note` field and is displayed in the task list
- **THEN** the task row SHALL include a visual indicator showing a note is linked
