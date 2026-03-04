## Why

The NLP model (Haiku) receives all tasks as JSON in the system prompt but struggles with date comparison across hundreds of tasks. When asked "show overdue tasks" or "update all overdue tasks", it fails to reliably identify which tasks have due dates before today because it must mentally compare each task's `due_date` against the current date. Exposing a `query_tasks` tool lets the model delegate filtering to Rust code, which handles date math accurately.

## What Changes

- Add a `query_tasks` tool definition to the Claude API tool list alongside the existing `fetch_url` tool
- Implement server-side task filtering that supports: status, priority, project, tag, title substring, overdue flag, has_due_date flag, and has_recurrence flag
- Thread task data through the API call chain (`interpret` → `call_claude_api` → `execute_tool`) so the tool executor can access tasks
- Update the system prompt to instruct the model to use `query_tasks` for date-based and complex filtering queries

## Capabilities

### New Capabilities
- `nlp-query-tool`: Defines the `query_tasks` tool schema, filtering logic, and integration with the existing tool-use loop

### Modified Capabilities
- `nlp-conversation`: The `call_claude_api` function signature changes to accept task data, and the system prompt gains `query_tasks` tool instructions

## Impact

- `src/nlp.rs`: New tool definition, new filtering function, refactored `call_claude_api` and `execute_tool` signatures to pass `&[Task]`
- No changes to `src/tui.rs` — the TUI already calls `nlp::interpret` with tasks
- No breaking changes to external interfaces
