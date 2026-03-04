## MODIFIED Requirements

### Requirement: Multi-turn system prompt
The system prompt SHALL instruct the model about the `show_tasks` action, multi-turn context, and available tools. The prompt SHALL tell the model to use `show_tasks` when the user wants to see tasks inline rather than filtering the main table, to use prior conversation context for follow-up queries, to use the `fetch_url` tool when the user asks about a URL or wants linked content summarized, and to use the `query_tasks` tool for date-based queries (overdue, due this week, due before/after a date) and complex filtering where scanning the task list would be unreliable.

#### Scenario: System prompt includes show_tasks format
- **WHEN** the system prompt is built for an NLP request
- **THEN** it SHALL include the `{"action":"show_tasks","task_ids":[...],"text":"..."}` format and usage instructions

#### Scenario: Model uses conversation context
- **WHEN** the user says "mark those as high priority" after a previous `show_tasks` response
- **THEN** the model SHALL have access to the prior conversation and be able to construct an `update` action targeting the previously shown task IDs

#### Scenario: System prompt describes fetch_url tool
- **WHEN** the system prompt is built for an NLP request
- **THEN** the prompt SHALL instruct the model that it can use the `fetch_url` tool to read web pages when the user asks about a URL or wants content summarized

#### Scenario: System prompt describes query_tasks tool
- **WHEN** the system prompt is built for an NLP request
- **THEN** the prompt SHALL instruct the model to use the `query_tasks` tool for date-based queries such as overdue tasks, tasks due this week, and tasks due before or after a specific date, as this is more reliable than scanning the task list manually
