## MODIFIED Requirements

### Requirement: NLP action types
The `NlpAction` enum SHALL have five variants: `Filter` containing criteria fields (project, status, priority, tag, title_contains — all optional), `Update` containing match criteria, set fields, and a human-readable description string, `Message` containing a text string, `ShowTasks` containing task IDs and text, and `SetRecurrence` containing a task_id, an optional recurrence string, and a description string.

#### Scenario: Filter variant fields
- **WHEN** an `NlpAction::Filter` is constructed
- **THEN** it SHALL contain optional fields: `project`, `status`, `priority`, `tag`, and `title_contains`

#### Scenario: Update variant fields
- **WHEN** an `NlpAction::Update` is constructed
- **THEN** it SHALL contain `match_criteria` (same fields as Filter), `set_fields` (priority, status, tags — all optional), and a `description` string

#### Scenario: SetRecurrence variant fields
- **WHEN** an `NlpAction::SetRecurrence` is constructed
- **THEN** it SHALL contain `task_id` (u32), `recurrence` (Option<String> — None to remove, Some to set), and `description` (String)

### Requirement: NLP intent interpretation
The system SHALL provide an `nlp::interpret` function that accepts the current task list, a natural language input string, and a Claude API key, and returns a structured `NlpAction` result. The function SHALL call the Claude API (`claude-haiku-4-5-20251001`) with a system prompt that includes a JSON summary of the current tasks (capped at 200 tasks), the current date formatted as `YYYY-MM-DD (DayOfWeek)`, and instructions to return a JSON object describing the intended action. The system prompt SHALL instruct the model to use the provided date for interpreting relative time references such as "today", "this week", "overdue", "tomorrow", etc. The task context SHALL include a `recurrence` field for each task (null if no recurrence, otherwise the recurrence string).

#### Scenario: System prompt includes current date
- **WHEN** the NLP system prompt is constructed
- **THEN** the prompt SHALL include a line stating today's date in `YYYY-MM-DD (DayOfWeek)` format (e.g., "Today's date is 2026-03-02 (Monday).")

#### Scenario: Relative date query interpreted correctly
- **WHEN** the user inputs "show overdue tasks" and today is 2026-03-02
- **THEN** the model SHALL have access to today's date in the prompt to determine which tasks have due dates before 2026-03-02

#### Scenario: Filter action returned
- **WHEN** the user inputs "show all tasks for the FLOW AI project"
- **THEN** `interpret` SHALL return an `NlpAction::Filter` with criteria matching project "FLOW AI"

#### Scenario: Update action returned
- **WHEN** the user inputs "mark all frontend tasks as high priority"
- **THEN** `interpret` SHALL return an `NlpAction::Update` with match criteria for tag "frontend" and set fields for priority "high", including a human-readable description of the change

#### Scenario: Set recurrence action returned
- **WHEN** the user inputs "make task 5 repeat every third thursday"
- **THEN** `interpret` SHALL return an `NlpAction::SetRecurrence` with `task_id: 5`, `recurrence: Some("monthly:3:thu")`, and a description

#### Scenario: Remove recurrence action returned
- **WHEN** the user inputs "stop task 3 from repeating"
- **THEN** `interpret` SHALL return an `NlpAction::SetRecurrence` with `task_id: 3`, `recurrence: None`, and a description

#### Scenario: Unrecognized input
- **WHEN** the user inputs something that cannot be mapped to a filter, update, or set_recurrence action
- **THEN** `interpret` SHALL return an error string describing that the input could not be understood

#### Scenario: API error
- **WHEN** the Claude API call fails (network error, 401, rate limit)
- **THEN** `interpret` SHALL return an error string with the failure reason

#### Scenario: Task list truncation
- **WHEN** the task list exceeds 200 tasks
- **THEN** the system SHALL include only the first 200 tasks in the prompt context and note the truncation

#### Scenario: Task context includes recurrence
- **WHEN** the task context is built for the NLP prompt
- **THEN** each task summary SHALL include a `recurrence` field (null if no recurrence, otherwise the serialized recurrence string)

### Requirement: NLP recurrence execution in TUI
When `nlp::interpret` returns an `NlpAction::SetRecurrence`, the TUI SHALL find the task by ID, set or clear its recurrence, save the file, and display a status message. If the task ID does not exist, a status error message SHALL be shown.

#### Scenario: Set recurrence via NLP chat
- **WHEN** the NLP returns `SetRecurrence { task_id: 5, recurrence: Some("weekly"), description: "Set task 5 to repeat weekly" }`
- **THEN** the TUI SHALL set task 5's recurrence to `Interval(Weekly)`, save the file, display the action summary and status message, and remain in NlpChat mode

#### Scenario: Clear recurrence via NLP chat
- **WHEN** the NLP returns `SetRecurrence { task_id: 3, recurrence: None, description: "Removed recurrence from task 3" }`
- **THEN** the TUI SHALL set task 3's recurrence to `None`, save the file, display the status message, and remain in NlpChat mode

#### Scenario: Invalid task ID in SetRecurrence
- **WHEN** the NLP returns `SetRecurrence { task_id: 999, ... }` and task 999 does not exist
- **THEN** the TUI SHALL display a status error "Task 999 not found" and remain in NlpChat mode

#### Scenario: Invalid recurrence string in SetRecurrence
- **WHEN** the NLP returns `SetRecurrence { recurrence: Some("biweekly"), ... }` which cannot be parsed
- **THEN** the TUI SHALL display a status error with the parse failure and remain in NlpChat mode
