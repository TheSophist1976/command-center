## Context

The NLP module (`src/nlp.rs`) calls the Claude API with a system prompt containing all tasks as JSON (up to 200). The model must interpret natural language and return structured actions. Currently, date-based queries ("show overdue tasks") fail because Haiku struggles to compare dates across hundreds of tasks. The existing tool infrastructure supports `fetch_url` via a tool-use loop (up to 3 iterations). The key constraint: `execute_tool` currently has no access to task data — it only receives the tool name and JSON input.

## Goals / Non-Goals

**Goals:**
- Give the model a `query_tasks` tool that filters tasks server-side with accurate date comparison
- Support filtering by: status, priority, project, tag, title substring, overdue, has_due_date, has_recurrence
- Thread task data through the existing call chain so tools can access it
- Keep the existing task context in the system prompt (the model still needs it for general questions)

**Non-Goals:**
- Replacing the system prompt task context with tool-only access
- Adding write/mutation tools (the model already returns Update actions for that)
- Pagination or sorting in query results
- Changes to the TUI layer — `interpret()` signature stays the same

## Decisions

### 1. Pass `&[Task]` through `call_claude_api` → `execute_tool`

**Choice:** Add a `tasks: &[Task]` parameter to both `call_claude_api` and `execute_tool`.

**Rationale:** This is the simplest threading approach. The alternative — using a closure or trait object for tool execution — adds complexity for no benefit since `interpret()` already has the tasks. The `fetch_url` tool simply ignores the parameter.

**Signature changes:**
- `call_claude_api(api_key, system_prompt, messages)` → `call_claude_api(api_key, tasks, system_prompt, messages)`
- `execute_tool(name, input)` → `execute_tool(name, input, tasks)`

### 2. Structured JSON input schema for `query_tasks`

**Choice:** Use individual optional fields rather than a query string.

**Rationale:** Structured fields are easier for the model to use correctly and map directly to filtering logic. A query string (e.g., `"status:open priority:high"`) would need parsing and is more error-prone.

**Fields:**
- `status`: string ("open"/"done")
- `priority`: string ("critical"/"high"/"medium"/"low")
- `project`: string
- `tag`: string
- `title_contains`: string (case-insensitive substring)
- `overdue`: boolean (true = open + due_date < today)
- `has_due_date`: boolean
- `has_recurrence`: boolean

### 3. Cap query results at 50 tasks

**Choice:** Return at most 50 matching tasks to keep token usage reasonable.

**Rationale:** Returning hundreds of tasks in a tool result would blow the context window. 50 is enough for any practical query while staying within token limits. Include a count note if truncated.

### 4. Return same format as TaskSummary

**Choice:** Reuse the existing `TaskSummary` serialization format for tool results.

**Rationale:** The model already sees tasks in this format in the system prompt, so it knows how to interpret the fields. No new format to learn.

## Risks / Trade-offs

- **Extra API round-trip** → The model must call the tool and wait for results, adding latency. Mitigation: only needed for date-based queries; simple filters still work from the system prompt context.
- **Model may not use the tool** → Haiku might try to answer from the system prompt instead of calling the tool. Mitigation: system prompt explicitly instructs to use `query_tasks` for date-based and overdue queries.
- **Increased prompt size** → Tool definitions add tokens. Mitigation: `query_tasks` schema is small (~200 tokens).
