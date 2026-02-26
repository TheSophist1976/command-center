## Context

The TUI currently supports structured filters via `f` key (`status:open`, `priority:high`, `tag:name`, `project:name`) and predefined time-based views (Today, Weekly, Monthly, etc.). The filter system uses a `Filter` struct with exact-match fields. Bulk operations don't exist — users modify tasks one at a time via keybindings.

The CLI has an established auth pattern: `task auth todoist` stores a token via `auth::write_token()` to `{config_dir}/task-manager/todoist_token`. The `auth` module provides `read_token()`, `write_token()`, and `delete_token()` functions.

## Goals / Non-Goals

**Goals:**
- Let users type natural language commands in the TUI to query and modify tasks
- Use Claude API to interpret intent and produce structured commands
- Support two operation types: filter/view queries and bulk updates
- Require confirmation before applying bulk modifications
- Follow existing auth patterns for API key management

**Non-Goals:**
- Streaming responses — blocking request is fine for single-shot intent parsing
- Conversation memory — each NLP command is stateless, no multi-turn context
- Task creation via NLP — too error-prone; users should create tasks explicitly
- Offline/fallback mode — NLP requires network; existing filters remain available for offline use

## Decisions

### 1. New `src/nlp.rs` module for Claude API interaction

**Choice**: Create a dedicated module that handles the Claude API call, prompt construction, and response parsing. The module exposes a single public function: `interpret(tasks: &[Task], input: &str, api_key: &str) -> Result<NlpAction, String>`.

**Why**: Isolates LLM concerns from TUI logic. The TUI calls `nlp::interpret()` and receives a typed action to execute. This keeps the TUI handler clean and makes the NLP logic independently testable.

**Alternative considered**: Inline the API call in the TUI handler. Rejected because the prompt construction and response parsing are complex enough to warrant their own module.

### 2. Structured JSON output from Claude

**Choice**: The system prompt instructs Claude to return a JSON object describing the action. Two action types:

```
// Query: filter the task list
{ "action": "filter", "criteria": { "project": "FLOW AI", "status": "open", "priority": null, "tag": null, "title_contains": "deployment" } }

// Bulk update: modify matching tasks
{ "action": "update", "match": { "tag": "frontend", "status": "open" }, "set": { "priority": "high" }, "description": "Set priority to high on 5 open frontend tasks" }
```

**Why**: Structured output is parseable and verifiable. The TUI can show the interpreted action to the user before executing. Using a constrained schema reduces hallucination risk.

**Alternative considered**: Free-form text response that we parse. Rejected because it's fragile and hard to validate.

### 3. Task context in the prompt

**Choice**: Send a JSON summary of all current tasks (id, title, status, priority, tags, due_date, project) as context in the system prompt. This lets Claude reference actual task data when interpreting queries.

**Why**: Claude needs to know what projects, tags, and tasks exist to interpret queries like "show tasks for FLOW AI". Without context, it can only guess at field values.

**Risk**: Large task lists could exceed token limits. Mitigation: truncate to first 200 tasks and note the truncation in the prompt. Most users won't have more than this in a single task file.

### 4. API key storage alongside Todoist token

**Choice**: Store the Claude API key at `{config_dir}/task-manager/claude_api_key` with 0600 permissions, following the exact same pattern as the Todoist token. Add `read_claude_key()`, `write_claude_key()`, and `delete_claude_key()` functions to `auth.rs`.

**Why**: Consistent with existing auth pattern. The `task auth claude` command mirrors `task auth todoist`.

**Alternative considered**: Environment variable `ANTHROPIC_API_KEY`. Rejected as primary mechanism because it doesn't persist across sessions, but we'll check for it as a fallback (env var takes precedence if set).

### 5. Confirmation flow for bulk updates

**Choice**: When `nlp::interpret()` returns an `Update` action, the TUI enters a `ConfirmingNlp` mode that displays the action description and affected task count in the footer. The user presses `y` to apply or any other key to cancel.

**Why**: Bulk updates are destructive. The confirmation step shows exactly what will change (e.g., "Set priority high on 5 tasks? y/n") before modifying anything.

### 6. NLP input via `:` keybinding

**Choice**: Press `:` in Normal mode to enter `NlpInput` mode. The footer shows a text input prompt. The user types their natural language command and presses Enter.

**Why**: `:` evokes the vim command-mode convention, signaling "I'm about to give a complex command". It's distinct from `f` (structured filter) and doesn't conflict with existing keybindings.

### 7. Claude API model selection

**Choice**: Use `claude-haiku-4-5-20251001` for intent parsing. The task is structured (JSON output from a constrained schema) and doesn't require deep reasoning.

**Why**: Haiku is fast (~200ms) and cheap. Intent parsing from a short user input with task context is well within its capability. Keeps the TUI responsive.

**Alternative considered**: Sonnet for better reasoning. Rejected because the constrained output format makes this a classification/extraction task, not a reasoning task.

## Risks / Trade-offs

- **API latency** → Haiku is typically <500ms. The TUI will block during the API call. Acceptable for single-shot commands. Status message will show "Thinking..." but it won't render until after the call returns (same pattern as Todoist import).
- **API key exposure** → Key stored on disk with 0600 permissions, same security model as Todoist token. Not perfect but consistent with the app's threat model.
- **Prompt injection via task data** → Task titles could contain adversarial text. Mitigation: the system prompt constrains output to JSON with a fixed schema, and we validate the parsed response before executing.
- **Token limits with large task lists** → Truncate to 200 tasks in the prompt context. Add a note so Claude knows the list is partial.
- **Cost** → Each NLP command costs ~0.001 USD with Haiku. Negligible for interactive use.

## Open Questions

- Should the env var fallback (`ANTHROPIC_API_KEY`) be documented as the primary method for users who already have it set from other tools?
- Should we rate-limit NLP calls to prevent accidental cost spikes (e.g., holding down Enter in NLP mode)?
