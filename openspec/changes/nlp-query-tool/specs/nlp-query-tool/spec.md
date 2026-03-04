## ADDED Requirements

### Requirement: query_tasks tool definition
The NLP module SHALL provide a `query_tasks` tool definition for the Claude API with the following optional input properties: `status` (string: "open"/"done"), `priority` (string: "critical"/"high"/"medium"/"low"), `project` (string), `tag` (string), `title_contains` (string for case-insensitive substring match), `overdue` (boolean), `has_due_date` (boolean), and `has_recurrence` (boolean). All properties SHALL be optional. The tool SHALL be included in the tools array sent to the Claude API alongside the existing `fetch_url` tool.

#### Scenario: Tool definition schema
- **WHEN** the `query_tasks` tool definition is constructed
- **THEN** it SHALL have name "query_tasks", a description explaining its filtering capabilities, and an input_schema with all eight optional properties

#### Scenario: Tool included in API request
- **WHEN** an NLP API request is sent to Claude
- **THEN** the tools array SHALL contain both `fetch_url` and `query_tasks` tool definitions

### Requirement: query_tasks execution
The NLP module SHALL provide a function that accepts tool input JSON and a task slice, filters tasks according to the input criteria, and returns matching tasks as a JSON array using the same `TaskSummary` format used in the system prompt context. The function SHALL apply all provided criteria as an AND filter (all conditions must match). Results SHALL be capped at 50 tasks; if more match, the response SHALL include a note stating the total count.

#### Scenario: Filter by status
- **WHEN** the tool is called with `{"status": "open"}`
- **THEN** the result SHALL contain only tasks with status "open"

#### Scenario: Filter by priority
- **WHEN** the tool is called with `{"priority": "high"}`
- **THEN** the result SHALL contain only tasks with priority "high" (case-insensitive)

#### Scenario: Filter by project
- **WHEN** the tool is called with `{"project": "FLOW AI"}`
- **THEN** the result SHALL contain only tasks whose project matches "FLOW AI" (case-insensitive)

#### Scenario: Filter by tag
- **WHEN** the tool is called with `{"tag": "frontend"}`
- **THEN** the result SHALL contain only tasks that have a tag matching "frontend" (case-insensitive)

#### Scenario: Filter by title substring
- **WHEN** the tool is called with `{"title_contains": "deploy"}`
- **THEN** the result SHALL contain only tasks whose title contains "deploy" (case-insensitive)

#### Scenario: Filter overdue tasks
- **WHEN** the tool is called with `{"overdue": true}` and today is 2026-03-04
- **THEN** the result SHALL contain only tasks with status "open" AND due_date before 2026-03-04 (strictly less than today)

#### Scenario: Filter tasks with due dates
- **WHEN** the tool is called with `{"has_due_date": true}`
- **THEN** the result SHALL contain only tasks where due_date is not null

#### Scenario: Filter tasks without due dates
- **WHEN** the tool is called with `{"has_due_date": false}`
- **THEN** the result SHALL contain only tasks where due_date is null

#### Scenario: Filter recurring tasks
- **WHEN** the tool is called with `{"has_recurrence": true}`
- **THEN** the result SHALL contain only tasks where recurrence is not null

#### Scenario: Combined criteria
- **WHEN** the tool is called with `{"status": "open", "priority": "high"}`
- **THEN** the result SHALL contain only tasks matching BOTH status "open" AND priority "high"

#### Scenario: No criteria returns all tasks
- **WHEN** the tool is called with `{}` (empty input)
- **THEN** the result SHALL contain all tasks (up to the 50-task cap)

#### Scenario: Results capped at 50
- **WHEN** more than 50 tasks match the criteria
- **THEN** the result SHALL contain the first 50 matching tasks and append a note: "(Showing 50 of N matching tasks)"

#### Scenario: No matching tasks
- **WHEN** no tasks match the criteria
- **THEN** the result SHALL be an empty JSON array `[]`

### Requirement: Task data threading
The `call_claude_api` function SHALL accept a task slice parameter and pass it to the tool execution layer. The `execute_tool` function SHALL accept a task slice parameter. The `fetch_url` tool SHALL ignore the task parameter. The `query_tasks` tool SHALL use the task parameter to perform filtering.

#### Scenario: call_claude_api receives tasks
- **WHEN** `interpret` calls `call_claude_api`
- **THEN** it SHALL pass the task slice alongside the API key, system prompt, and messages

#### Scenario: execute_tool receives tasks
- **WHEN** the API response contains a tool_use block
- **THEN** `execute_tool` SHALL be called with the tool name, input, and the task slice

#### Scenario: interpret signature unchanged
- **WHEN** the TUI calls `nlp::interpret`
- **THEN** the function signature SHALL remain `interpret(tasks, messages, api_key)` with no changes required at the call site
