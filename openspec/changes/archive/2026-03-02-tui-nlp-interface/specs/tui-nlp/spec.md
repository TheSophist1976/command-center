## ADDED Requirements

### Requirement: NLP intent interpretation
The system SHALL provide an `nlp::interpret` function that accepts the current task list, a natural language input string, and a Claude API key, and returns a structured `NlpAction` result. The function SHALL call the Claude API (`claude-haiku-4-5-20251001`) with a system prompt that includes a JSON summary of the current tasks (capped at 200 tasks) and instructs the model to return a JSON object describing the intended action.

#### Scenario: Filter action returned
- **WHEN** the user inputs "show all tasks for the FLOW AI project"
- **THEN** `interpret` SHALL return an `NlpAction::Filter` with criteria matching project "FLOW AI"

#### Scenario: Update action returned
- **WHEN** the user inputs "mark all frontend tasks as high priority"
- **THEN** `interpret` SHALL return an `NlpAction::Update` with match criteria for tag "frontend" and set fields for priority "high", including a human-readable description of the change

#### Scenario: Unrecognized input
- **WHEN** the user inputs something that cannot be mapped to a filter or update action
- **THEN** `interpret` SHALL return an error string describing that the input could not be understood

#### Scenario: API error
- **WHEN** the Claude API call fails (network error, 401, rate limit)
- **THEN** `interpret` SHALL return an error string with the failure reason

#### Scenario: Task list truncation
- **WHEN** the task list exceeds 200 tasks
- **THEN** the system SHALL include only the first 200 tasks in the prompt context and note the truncation

### Requirement: NLP action types
The `NlpAction` enum SHALL have two variants: `Filter` containing criteria fields (project, status, priority, tag, title_contains — all optional) and `Update` containing match criteria, set fields, and a human-readable description string.

#### Scenario: Filter variant fields
- **WHEN** an `NlpAction::Filter` is constructed
- **THEN** it SHALL contain optional fields: `project`, `status`, `priority`, `tag`, and `title_contains`

#### Scenario: Update variant fields
- **WHEN** an `NlpAction::Update` is constructed
- **THEN** it SHALL contain `match_criteria` (same fields as Filter), `set_fields` (priority, status, tags — all optional), and a `description` string

### Requirement: NLP input mode in TUI
The user SHALL press `:` in Normal mode to enter `NlpInput` mode. The footer SHALL display a text input prompt. The user SHALL type a natural language command and press `Enter` to submit or `Esc` to cancel.

#### Scenario: Enter NLP input mode
- **WHEN** the user presses `:` in Normal mode
- **THEN** the TUI SHALL enter `NlpInput` mode and the footer SHALL display ` > _ ` with a text input cursor

#### Scenario: Submit NLP command
- **WHEN** the user types "show tasks due this week" and presses `Enter`
- **THEN** the system SHALL call `nlp::interpret` with the input and process the result

#### Scenario: Cancel NLP input
- **WHEN** the user presses `Esc` in `NlpInput` mode
- **THEN** the TUI SHALL return to Normal mode without making any API call

### Requirement: NLP filter execution
When `nlp::interpret` returns an `NlpAction::Filter`, the TUI SHALL apply the filter criteria to the displayed task list. The `title_contains` field SHALL perform a case-insensitive substring match on task titles. The filter SHALL be displayed in the header and clearable with `Esc` like any other filter.

#### Scenario: Filter by project via NLP
- **WHEN** the NLP result is a Filter with project "FLOW AI"
- **THEN** the task table SHALL show only tasks with project "FLOW AI" (case-insensitive)

#### Scenario: Filter by title substring via NLP
- **WHEN** the NLP result is a Filter with title_contains "deployment"
- **THEN** the task table SHALL show only tasks whose title contains "deployment" (case-insensitive)

#### Scenario: Combined NLP filter criteria
- **WHEN** the NLP result is a Filter with project "FLOW AI" and status "open"
- **THEN** the task table SHALL show only open tasks in the "FLOW AI" project

### Requirement: NLP bulk update execution with confirmation
When `nlp::interpret` returns an `NlpAction::Update`, the TUI SHALL enter a `ConfirmingNlp` mode. The footer SHALL display the action description and the count of tasks matching the criteria, followed by "y/n". Pressing `y` SHALL apply the update to all matching tasks, save the file, and show a status message with the result. Any other key SHALL cancel.

#### Scenario: Confirm bulk priority update
- **WHEN** the NLP result is an Update setting priority to high on 5 matching tasks and the user presses `y`
- **THEN** the system SHALL set priority to high on all 5 matching tasks, save the file, and display "Updated 5 tasks"

#### Scenario: Cancel bulk update
- **WHEN** the NLP result is an Update and the user presses any key other than `y`
- **THEN** no tasks SHALL be modified and the TUI SHALL return to Normal mode

#### Scenario: Zero matching tasks
- **WHEN** the NLP result is an Update but no tasks match the criteria
- **THEN** the TUI SHALL display a status message "No tasks match the criteria" and return to Normal mode without entering confirmation

### Requirement: NLP error display
When `nlp::interpret` returns an error, the TUI SHALL display the error as a status message in the footer and return to Normal mode.

#### Scenario: No API key error
- **WHEN** no Claude API key is configured and the user submits an NLP command
- **THEN** the footer SHALL display "No Claude API key. Run `task auth claude` or set ANTHROPIC_API_KEY."

#### Scenario: API failure error
- **WHEN** the Claude API call fails
- **THEN** the footer SHALL display the error message and the TUI SHALL return to Normal mode
