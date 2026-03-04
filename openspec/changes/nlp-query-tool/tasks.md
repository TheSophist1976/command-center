## 1. Tool Definition

- [x] 1.1 Add `query_tasks_tool_def()` function in `src/nlp.rs` that returns a `ToolDef` with name "query_tasks", description, and input_schema with eight optional properties (status, priority, project, tag, title_contains, overdue, has_due_date, has_recurrence)

## 2. Query Execution

- [x] 2.1 Add `execute_query_tasks(input: &serde_json::Value, tasks: &[Task]) -> String` function in `src/nlp.rs` that parses the JSON input, filters tasks by all provided criteria (AND logic), and returns matching tasks as a JSON array in `TaskSummary` format, capped at 50 results
- [x] 2.2 Implement the `overdue` filter: when `overdue: true`, match only tasks with status "open" AND `due_date < Local::now().date_naive()`

## 3. Data Threading

- [x] 3.1 Change `execute_tool` signature to `execute_tool(name, input, tasks: &[Task])` and route "query_tasks" to `execute_query_tasks`; `fetch_url` ignores the tasks parameter
- [x] 3.2 Change `call_claude_api` signature to accept `tasks: &[Task]`, pass tasks to `execute_tool`, and add `query_tasks_tool_def()` to the tools vector
- [x] 3.3 Update `interpret()` to pass tasks to `call_claude_api`

## 4. System Prompt

- [x] 4.1 Update the system prompt in `build_system_prompt` to instruct the model to use the `query_tasks` tool for date-based queries (overdue, due this week) as it is more reliable than scanning the task list manually

## 5. Tests

- [x] 5.1 Add tests: `query_tasks_tool_def_has_correct_name`, `execute_query_tasks_filters_by_status`, `execute_query_tasks_filters_overdue`, `execute_query_tasks_combined_criteria`, `execute_query_tasks_caps_at_50`, `system_prompt_mentions_query_tasks`
