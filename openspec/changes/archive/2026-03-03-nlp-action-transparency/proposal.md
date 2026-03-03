## Why

When the user sends a natural language command, the TUI shows only a terse result like "Filter applied." or jumps straight to a confirmation dialog. There's no visibility into what the AI decided to do or why. The user can't tell:
- What action the AI chose (filter, update, show_tasks, message)
- What criteria it's matching on
- What fields it's setting in an update
- Whether its interpretation matched the user's intent

This makes the NLP feel like a black box. Additionally, the confirmation dialog for updates only shows a description and task count ("Set priority high (5 tasks) — y/n") without listing which tasks will be affected or what specifically will change on each one.

Two improvements:
1. **Action transparency** — show the AI's interpreted action in the chat panel so the user understands what it decided
2. **Detailed confirmation** — the update confirmation shows a list of affected tasks with before→after changes so the user can make an informed approval

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- **tui-nlp**: The chat panel will display structured action summaries after NLP responses. The update confirmation flow will show affected tasks with specific before→after field changes.

## Impact

- `src/tui.rs`: `handle_nlp_chat` adds action summary messages to chat; `ConfirmingNlp` mode renders task change details in the chat panel
- No changes to `src/nlp.rs` — the action data is already available, just not surfaced to the user
- No API or dependency changes
