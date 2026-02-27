## ADDED Requirements

### Requirement: Conversation history management
The NLP module SHALL maintain a conversation history as an ordered list of user and assistant messages. The `call_claude_api` function SHALL accept the full message history (not a single user input) and send all messages in the API request. The `interpret` function SHALL accept the accumulated message history and return the parsed action.

#### Scenario: First message in conversation
- **WHEN** the user sends their first NLP query in a session
- **THEN** the API request SHALL contain a messages array with one user message and the system prompt SHALL include the current task context

#### Scenario: Follow-up message includes history
- **WHEN** the user sends a second query after receiving a response
- **THEN** the API request SHALL contain the full conversation: first user message, first assistant response, and second user message

#### Scenario: History cap at 20 messages
- **WHEN** the conversation exceeds 20 messages (10 turns)
- **THEN** the oldest messages SHALL be dropped from the API request while the most recent 20 messages are preserved

### Requirement: ShowTasks action
The NLP module SHALL support a `ShowTasks` action that returns a list of task IDs and an accompanying text message. The model SHALL respond with `{"action":"show_tasks","task_ids":[...],"text":"..."}` when the user asks to see specific tasks without modifying the table filter.

#### Scenario: Model returns show_tasks
- **WHEN** the user asks "show me my high-priority tasks"
- **THEN** the model SHALL return a `ShowTasks` action with the matching task IDs and a descriptive text message

#### Scenario: Parse show_tasks response
- **WHEN** the API returns `{"action":"show_tasks","task_ids":[1,3,7],"text":"Here are your high-priority tasks:"}`
- **THEN** `parse_response` SHALL return `NlpAction::ShowTasks` with task IDs `[1, 3, 7]` and the text string

#### Scenario: show_tasks with no matching tasks
- **WHEN** the model returns a `ShowTasks` action with an empty task_ids array
- **THEN** the system SHALL return `NlpAction::ShowTasks` with an empty vec and the text message (e.g., "No tasks match that criteria.")

### Requirement: Multi-turn system prompt
The system prompt SHALL instruct the model about the `show_tasks` action and multi-turn context. The prompt SHALL tell the model to use `show_tasks` when the user wants to see tasks inline rather than filtering the main table, and to use prior conversation context for follow-up queries (e.g., "mark those as high priority" referring to previously shown tasks).

#### Scenario: System prompt includes show_tasks format
- **WHEN** the system prompt is built for an NLP request
- **THEN** it SHALL include the `{"action":"show_tasks","task_ids":[...],"text":"..."}` format and usage instructions

#### Scenario: Model uses conversation context
- **WHEN** the user says "mark those as high priority" after a previous `show_tasks` response
- **THEN** the model SHALL have access to the prior conversation and be able to construct an `update` action targeting the previously shown task IDs
