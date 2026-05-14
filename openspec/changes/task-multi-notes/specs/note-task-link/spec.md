## MODIFIED Requirements

### Requirement: Task note field
The `Task` struct SHALL support multiple linked notes via `notes: Vec<String>`. See `task-storage` spec for serialization details.

#### Scenario: Task with multiple notes shows count badge
- **WHEN** a task has two or more notes and is displayed in the task list
- **THEN** the task row SHALL display a count badge (e.g. `[2]`) indicating the number of attached notes

#### Scenario: Task with one note shows indicator
- **WHEN** a task has exactly one note and is displayed in the task list
- **THEN** the task row SHALL display a note indicator (existing behavior preserved)

#### Scenario: Task with no notes shows no indicator
- **WHEN** a task has zero notes
- **THEN** no note indicator SHALL be shown

### Requirement: Multi-note picker via `n` key
The `n` key in Normal mode SHALL open a multi-note picker showing all currently attached notes plus actions to add or remove a note. Selecting an attached note slug SHALL open that note in the inline editor. Selecting "Add note" SHALL show the existing note list for linking. Selecting "Remove note" SHALL show the attached notes for removal.

#### Scenario: Open multi-note picker
- **WHEN** the user presses `n` on a task with multiple notes
- **THEN** the picker SHALL list all attached note slugs, "Add note", and "Remove note"

#### Scenario: Add note via picker
- **WHEN** the user selects "Add note" and chooses a slug
- **THEN** that slug SHALL be appended to the task's `notes` list if not already present

#### Scenario: Remove note via picker
- **WHEN** the user selects "Remove note" and chooses a slug
- **THEN** that slug SHALL be removed from the task's `notes` list

#### Scenario: Navigate to note via picker
- **WHEN** the user selects an attached note slug from the picker
- **THEN** the TUI SHALL open that note in the inline note editor

### Requirement: Navigate from task to linked notes via `g` key
The `g` key SHALL open the linked note directly if exactly one note is attached. If multiple notes are attached, the `g` key SHALL show a picker to select which note to open.

#### Scenario: Single note — direct open
- **WHEN** the user presses `g` on a task with exactly one linked note
- **THEN** the TUI SHALL open that note in the inline note editor (existing behavior)

#### Scenario: Multiple notes — show picker
- **WHEN** the user presses `g` on a task with two or more linked notes
- **THEN** the TUI SHALL show a picker listing the attached note slugs for selection

### Requirement: `task note link` appends to notes list
The `task note link <slug> <task-id>` CLI command SHALL append `slug` to the task's `notes` list if not already present. It SHALL NOT replace the existing notes.

#### Scenario: Link adds to existing notes
- **WHEN** a task has `notes: ["a"]` and `task note link b <id>` is run
- **THEN** the task SHALL have `notes: ["a", "b"]`

#### Scenario: Link is idempotent
- **WHEN** a task already has slug `a` in its `notes` and `task note link a <id>` is run
- **THEN** the notes list SHALL be unchanged

### Requirement: `task note unlink` removes from notes list
The `task note unlink <task-id>` CLI command SHALL remove the specified note slug from the task's `notes` list. When multiple notes are attached, the command SHALL require the slug to remove. When only one note is attached, it SHALL remove it without requiring the slug (backward-compatible behavior).

#### Scenario: Unlink removes specific slug
- **WHEN** a task has `notes: ["a", "b"]` and `task note unlink <id>` is run
- **THEN** the command SHALL prompt for which slug to remove, then remove it

#### Scenario: Unlink single note needs no slug
- **WHEN** a task has `notes: ["a"]` and `task note unlink <id>` is run
- **THEN** the task SHALL have `notes: []` without prompting

### Requirement: Instruction note convention
A note whose slug or title contains `instructions`, `how-to`, or `steps` SHALL be treated as an instruction note by AI agents. Agents SHALL read instruction notes before beginning Phase 0 of any task that has them attached. This is a naming convention enforced by agent skills and instructions, not by the data model.

#### Scenario: Instruction note identified by slug
- **WHEN** a task has a note attached with slug `deploy-instructions`
- **THEN** agents working that task SHALL read it before starting work

#### Scenario: Regular notes read for context
- **WHEN** a task has attached notes that are not instruction notes
- **THEN** agents SHALL read them as context after reading instruction notes
