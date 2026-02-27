## Context

The NLP interface is single-shot: pressing `:` enters `NlpInput` mode, the user types a query, the TUI calls `nlp::interpret()` which builds a system prompt with task context, sends one user message to Claude, parses the JSON response into `NlpAction`, and returns to `Normal` mode. The `call_claude_api` function takes a single `user_input: &str` and constructs a one-element `messages` array. Responses are displayed in the single-line footer status bar.

The current layout is a fixed 3-row vertical split: header (1 line), task table (fill), footer (1 line). All NLP interaction happens in the footer line.

## Goals / Non-Goals

**Goals:**
- Multi-turn conversation: user can send follow-up messages without re-entering NLP mode
- Conversation history sent to Claude API so the model has context from prior turns
- Model can return groups of tasks to display inline in the conversation
- Dedicated chat panel for viewing conversation history with task groups
- Preserve existing single-shot behavior for users who type one query and press `Esc`

**Non-Goals:**
- Streaming responses (stay with blocking `reqwest` for now)
- Persistent conversation history across TUI sessions
- Tool use / function calling via Claude API (we parse JSON responses, not tool calls)
- Rich markdown rendering in the chat panel (plain text is sufficient)

## Decisions

### 1. Conversation state in `App`

Add a `chat_history: Vec<ChatMessage>` field to `App`, where `ChatMessage` is an enum with variants for user messages, assistant text, assistant task lists, and errors. Also add `nlp_messages: Vec<ApiMessage>` to track the raw message history sent to the API.

When the user enters NLP mode, `chat_history` starts empty. Each user input appends a user message, the API response appends an assistant message. On `Esc`, conversation state is cleared and the TUI returns to Normal.

**Alternative considered**: Store conversation state in the `nlp` module. Rejected because the TUI owns the lifecycle (when to start/clear conversations) and the rendering (chat panel), so it's simpler to keep state in `App`.

### 2. Update `call_claude_api` to accept message history

Change signature from `call_claude_api(api_key, system_prompt, user_input)` to `call_claude_api(api_key, system_prompt, messages: &[ApiMessage])`. The caller builds the full message list. `ApiMessage` becomes public.

The existing `interpret` function changes to `interpret(tasks, messages, api_key)` accepting the accumulated message history. After each call, the caller appends both the user input and the assistant's raw response to `nlp_messages` for the next turn.

**Alternative considered**: Have `interpret` own the history internally with a session object. Rejected because it adds statefulness to a module that's currently stateless — the TUI already manages state.

### 3. New `NlpAction::ShowTasks(Vec<u32>, String)` variant

The model can return `{"action":"show_tasks","task_ids":[1,3,7],"text":"Here are your overdue tasks:"}`. The `task_ids` field contains task IDs to display, and `text` is the accompanying message. The TUI renders the message followed by a compact task list (id, title, status, priority) in the chat panel.

This lets the model answer questions like "show me my high-priority tasks" by returning the actual tasks rather than just a filter. The key difference from `filter`: `show_tasks` displays tasks inline in the conversation without changing the main table view, while `filter` modifies the table.

**Alternative considered**: Reuse `Filter` and display filtered results. Rejected because filters modify the main table view — `show_tasks` is non-destructive and conversational.

### 4. New `NlpChat` mode with split layout

When the user enters NLP mode, the TUI switches to `NlpChat` mode. The layout changes to:

```
┌──────────────────────────────────┐
│ header (1 line)                  │
├──────────────────────────────────┤
│ task table (top half)            │
│                                  │
├──────────────────────────────────┤
│ chat panel (bottom half)         │
│  User: show me overdue tasks     │
│  AI: Here are your overdue tasks │
│   #3 Fix login bug [high]        │
│   #7 Update docs [medium]        │
│                                  │
├──────────────────────────────────┤
│ > input prompt_ (1 line)         │
└──────────────────────────────────┘
```

The layout splits the middle area: task table gets the top portion, chat panel gets the bottom. The footer becomes the input prompt. `Esc` collapses back to the standard 3-row layout.

**Alternative considered**: Full-screen chat panel replacing the task table. Rejected because seeing the task table alongside the conversation is essential — the user needs to see their tasks while chatting about them.

**Alternative considered**: Overlay/popup for chat. Rejected because ratatui popups are awkward for scrollable multi-turn content and the split is simpler.

### 5. Chat panel rendering

The chat panel is a scrollable `Paragraph` widget with styled text. User messages are prefixed with `> ` and displayed in one color, assistant messages in another. Task groups from `ShowTasks` are rendered as indented compact rows below the assistant message.

The panel auto-scrolls to the bottom on new messages. No manual scroll for v1 — can be added later if conversations get long.

### 6. Remove `NlpInput` mode, replace with `NlpChat`

The existing `NlpInput` mode is replaced by `NlpChat`. The `:` key enters `NlpChat`, which shows the split layout with an input prompt. The `ConfirmingNlp` mode remains unchanged for update confirmations — after confirmation, the TUI stays in `NlpChat` rather than returning to Normal.

## Risks / Trade-offs

- **Token cost increases with conversation length** → Mitigate by capping conversation history at ~20 messages (10 turns). Older messages are dropped from the API request but remain visible in the chat panel.
- **Blocking API calls freeze the TUI during each turn** → Acceptable for v1 since this is the existing behavior. Async can be added later.
- **Chat panel takes screen space from the task table** → Use a ~40% split for chat, leaving 60% for the table. Users on small terminals may find this tight, but `Esc` restores the full table instantly.
- **`show_tasks` with invalid IDs** → Silently skip task IDs that don't exist in the current task list. Display a note if some IDs were skipped.
