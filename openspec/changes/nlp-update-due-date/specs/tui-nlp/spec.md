## MODIFIED Requirements

### Requirement: NLP action types
The `NlpAction` enum SHALL have five variants: `Filter` containing criteria fields (project, status, priority, tag, title_contains — all optional), `Update` containing match criteria, set fields, and a human-readable description string, `Message` containing a text string, `ShowTasks` containing task IDs and text, and `SetRecurrence` containing a task_id, an optional recurrence string, and a description string. The `SetFields` struct SHALL include optional fields for `priority`, `status`, `tags`, and `due_date`.

#### Scenario: Update variant fields
- **WHEN** an `NlpAction::Update` is constructed
- **THEN** it SHALL contain `match_criteria` (same fields as Filter), `set_fields` (priority, status, tags, due_date — all optional), and a `description` string

#### Scenario: SetFields due_date field
- **WHEN** `set_fields.due_date` is `Some("2026-03-10")`
- **THEN** the system SHALL set matching tasks' due date to March 10, 2026

#### Scenario: SetFields due_date clear
- **WHEN** `set_fields.due_date` is `Some("")` or the model sets due_date to null in the JSON
- **THEN** the system SHALL clear matching tasks' due date

### Requirement: NLP bulk update execution with confirmation
When `nlp::interpret` returns an `NlpAction::Update`, the TUI SHALL enter a `ConfirmingNlp` mode. The chat panel SHALL display per-task before→after changes including due date changes. The footer SHALL display the action description and the count of tasks matching the criteria, followed by "y/n". Pressing `y` SHALL apply the update to all matching tasks, save the file, and show a status message with the result. Any other key SHALL cancel.

#### Scenario: Due date change shown in preview
- **WHEN** the NLP returns an update action that changes due dates
- **THEN** the chat panel SHALL list each affected task with its due date change (e.g., `#3 "Fix bug": due_date 2026-03-01 → 2026-03-10`)

#### Scenario: Due date applied on confirm
- **WHEN** the user confirms an update that sets `due_date` to "2026-03-10" on matching tasks
- **THEN** the system SHALL parse the date string and set each matching task's `due_date` to `Some(NaiveDate)`, then save the file

#### Scenario: Invalid due date string
- **WHEN** the model returns a `due_date` value that cannot be parsed as YYYY-MM-DD
- **THEN** the system SHALL display a status error and not modify any tasks
