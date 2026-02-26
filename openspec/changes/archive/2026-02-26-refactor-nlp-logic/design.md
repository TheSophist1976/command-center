## Context

The NLP module (`src/nlp.rs`) sends user input to Claude's API with a system prompt that demands a JSON response in one of two formats: `filter` or `update`. The `interpret()` function returns `Result<NlpAction, String>`, where the `Err` path is the only way non-structured responses surface — as error messages in the TUI status bar.

The TUI (`src/tui.rs`) matches on `NlpAction::Filter` and `NlpAction::Update` in the NLP result handler (~line 674). Errors from `interpret()` are displayed as status messages.

## Goals / Non-Goals

**Goals:**
- Allow the model to return plain-text responses for unclear, conversational, or unsupported queries
- Enable the model to answer questions about the user's tasks using the task context already provided in the system prompt (e.g., counts, summaries, queries across all fields)
- Display these responses to the user in the TUI without them appearing as errors
- Keep the existing filter/update paths unchanged

**Non-Goals:**
- Multi-turn conversation or chat history — this is still single-shot
- Adding new structured action types beyond message
- Changing the API transport or model selection

## Decisions

### 1. New `NlpAction::Message(String)` variant

Add a third variant to the `NlpAction` enum. This keeps the return type as `Result<NlpAction, String>` — the `Ok` path now has three cases, and `Err` remains for actual failures (network errors, API errors).

**Alternative considered**: Return `Result<Option<NlpAction>, String>` with `None` for messages. Rejected because a message is a valid action result, not an absence of result.

### 2. New JSON response format: `{"action":"message","text":"..."}`

Add a third JSON format to the system prompt. The model responds with `{"action":"message","text":"..."}` when:
- The query is ambiguous or doesn't clearly map to filter/update
- The user asks a question about their tasks that can be answered from the task context (e.g., "how many high-priority tasks do I have?", "what's my oldest open task?", "list tasks due this week"). The model has full access to task fields: id, title, status, priority, tags, due_date, and project.
- The input is conversational or the model needs to ask for clarification

**Alternative considered**: Let the model respond with raw text (non-JSON) and detect it by failed JSON parse. Rejected because it conflates intentional messages with actual malformed responses.

### 3. Display messages in the TUI status bar

Reuse the existing `app.status_message` field (already used for errors and confirmations) to show model messages. The message replaces the current status and persists until the next action clears it.

**Alternative considered**: A dedicated message overlay/popup. Rejected as over-engineered for v1 — the status bar already handles transient messages and the pattern is established.

## Risks / Trade-offs

- **Model may over-use message responses** → Mitigate by keeping the system prompt directive clear: prefer filter/update when possible, only use message when the query truly doesn't map to a structured action.
- **Long messages may truncate in status bar** → Acceptable for v1; status bar already truncates long text. Can revisit with a scrollable area later if needed.
