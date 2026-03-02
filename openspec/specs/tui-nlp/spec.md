### Requirement: NLP intent interpretation
The system SHALL provide an `nlp::interpret` function that accepts the current task list, a natural language input string, and a Claude API key, and returns a structured `NlpAction` result. The function SHALL call the Claude API (`claude-haiku-4-5-20251001`) with a system prompt that includes a JSON summary of the current tasks (capped at 200 tasks), the current date formatted as `YYYY-MM-DD (DayOfWeek)`, and instructions to return a JSON object describing the intended action. The system prompt SHALL instruct the model to use the provided date for interpreting relative time references such as "today", "this week", "overdue", "tomorrow", etc.

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

### Requirement: NLP message responses
When the NLP model determines that the user's query is unclear, conversational, does not map to a filter, update, or show_tasks action, or is a question about the user's tasks, the system SHALL return a `Message(String)` action containing the model's plain-text response. The model SHALL use the task context (all fields: id, title, status, priority, tags, due_date, project) to answer task queries. The TUI SHALL display this message text in the chat panel and remain in NlpChat mode.

#### Scenario: Ambiguous query returns message
- **WHEN** the user enters NLP mode and types an ambiguous query like "hello"
- **THEN** the NLP module SHALL return `NlpAction::Message` with a helpful text response, and the TUI SHALL display that text in the chat panel

#### Scenario: Task count query
- **WHEN** the user enters NLP mode and types "how many high-priority tasks do I have?"
- **THEN** the NLP module SHALL return `NlpAction::Message` with the count derived from the task context

#### Scenario: Task field query
- **WHEN** the user enters NLP mode and types "what projects do I have tasks in?"
- **THEN** the NLP module SHALL return `NlpAction::Message` listing the distinct project names from the task data

#### Scenario: Task summary query
- **WHEN** the user enters NLP mode and types "what's my oldest open task?"
- **THEN** the NLP module SHALL return `NlpAction::Message` with the answer based on task creation dates and status

#### Scenario: Unsupported action returns message
- **WHEN** the user enters NLP mode and requests an action the system cannot perform (e.g., "email my tasks to Alice")
- **THEN** the NLP module SHALL return `NlpAction::Message` explaining that the action is not supported

#### Scenario: Message display stays in NlpChat mode
- **WHEN** the TUI receives an `NlpAction::Message` response while in NlpChat mode
- **THEN** the TUI SHALL append the message to the chat panel, clear the input buffer, and remain in NlpChat mode for follow-up input

### Requirement: Chat panel display
The TUI SHALL render a chat panel in NlpChat mode that displays the conversation history. User messages SHALL be prefixed with `> ` and visually distinguished from assistant messages. The chat panel SHALL auto-scroll to the most recent message when new messages are added.

#### Scenario: User message displayed
- **WHEN** the user submits a query in NlpChat mode
- **THEN** the chat panel SHALL display the user's message prefixed with `> `

#### Scenario: Assistant message displayed
- **WHEN** the model returns a Message or ShowTasks response
- **THEN** the chat panel SHALL display the assistant's text response below the user's message

#### Scenario: Auto-scroll on new message
- **WHEN** a new message is added to the conversation and the chat panel content exceeds the visible area
- **THEN** the chat panel SHALL scroll to show the most recent message

### Requirement: ShowTasks display in chat panel
When the model returns a `ShowTasks` action, the TUI SHALL display the accompanying text message followed by a compact task list in the chat panel. Each task SHALL be rendered with its ID, title, status, and priority. Task IDs that do not exist in the current task list SHALL be silently skipped.

#### Scenario: ShowTasks renders task list
- **WHEN** the model returns `ShowTasks` with task IDs `[1, 3, 7]` and text "Here are your overdue tasks:"
- **THEN** the chat panel SHALL display the text followed by a list showing each task's ID, title, status, and priority

#### Scenario: ShowTasks with invalid task IDs
- **WHEN** the model returns `ShowTasks` with task IDs `[1, 999]` and task 999 does not exist
- **THEN** the chat panel SHALL display task 1's details and silently skip task 999

#### Scenario: ShowTasks does not modify table filter
- **WHEN** the model returns a `ShowTasks` action
- **THEN** the main task table filter SHALL NOT be modified and the TUI SHALL remain in NlpChat mode

### Requirement: NlpChat conversation lifecycle
The conversation state SHALL be initialized when entering NlpChat mode and cleared when exiting. Pressing `Esc` in NlpChat mode SHALL clear the conversation history, restore the standard three-region layout, and return to Normal mode.

#### Scenario: Enter NlpChat mode
- **WHEN** the user presses `:` in Normal mode
- **THEN** the TUI SHALL enter NlpChat mode with empty conversation history and display the split layout with input prompt

#### Scenario: Stay in NlpChat after response
- **WHEN** the model returns any NlpAction (Filter, Update confirmation, Message, or ShowTasks)
- **THEN** the TUI SHALL remain in NlpChat mode with the input buffer cleared, ready for follow-up input

#### Scenario: Exit NlpChat mode
- **WHEN** the user presses `Esc` in NlpChat mode
- **THEN** the TUI SHALL clear conversation history, restore the three-region layout, and return to Normal mode

#### Scenario: NlpChat update confirmation
- **WHEN** the model returns an Update action while in NlpChat mode
- **THEN** the TUI SHALL enter ConfirmingNlp mode. After confirmation or cancellation, the TUI SHALL return to NlpChat mode (not Normal mode)
