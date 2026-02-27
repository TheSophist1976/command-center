## Why

The NLP interface is currently single-shot: the user types a query, gets one result (filter, update, or message), and is dropped back to Normal mode. There's no way to have a back-and-forth conversation — e.g., asking "show me overdue tasks", then following up with "mark those as high priority". The model also can't display groups of tasks as part of its response; it can only set a status bar message or trigger a filter. Multi-turn conversation with task display would make the NLP interface significantly more useful as an interactive assistant.

## What Changes

- Maintain conversation history (list of user/assistant messages) across NLP turns instead of sending a single message per call
- Keep the TUI in NLP mode after receiving a response, allowing follow-up queries without pressing `:` again (`Esc` exits)
- Add a new `NlpAction::ShowTasks` variant that lets the model return a list of task IDs to display to the user as part of the conversation
- Replace the single-line status bar display with a scrollable chat panel that shows the conversation history, including rendered task groups
- Update `call_claude_api` to accept a full message history instead of a single user input
- Update the system prompt to instruct the model about multi-turn context and the new `show_tasks` action

## Capabilities

### New Capabilities

- `nlp-conversation`: Multi-turn conversation state management, message history, and the chat panel UI for displaying conversation alongside task groups

### Modified Capabilities

- `tui`: New `NlpChat` mode with a split-panel layout (chat panel + task table), replaces single-shot `NlpInput` mode

## Impact

- `src/nlp.rs`: New `ShowTasks` action variant, conversation history support in `call_claude_api` and `interpret`, updated system prompt
- `src/tui.rs`: New `NlpChat` mode, chat panel rendering, conversation state in `App`, split layout when in chat mode
- No new dependencies (ratatui already supports the needed layout primitives)
- No breaking changes to existing filter/update behavior outside of NLP mode
