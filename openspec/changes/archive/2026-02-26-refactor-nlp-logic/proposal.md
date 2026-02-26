## Why

The NLP interface currently forces every user query into a structured action (`filter` or `update`). When the user types something unclear, conversational, or outside those two actions, the model either produces invalid JSON (surfaced as a parse error) or force-fits the input into the wrong action. The model should be able to respond with plain text when the query is ambiguous, unsupported, or conversational — giving the user helpful feedback instead of a cryptic error. Additionally, the model receives task context but has no way to answer questions about tasks (e.g., "how many high-priority tasks do I have?", "what's my oldest open task?") — the message response enables this.

## What Changes

- Add a `Message(String)` variant to `NlpAction` so the model can return free-text responses
- Update the system prompt to instruct the model to use a `{"action":"message","text":"..."}` response when the query is unclear, doesn't map to filter/update, or is a question about the user's tasks that can be answered from the provided task context
- Update `parse_response` to handle the new `message` action type
- Update the TUI to display message responses inline (e.g., in the status bar or a transient overlay) instead of treating them as errors

## Capabilities

### New Capabilities

_(none — this extends the existing NLP and TUI capabilities)_

### Modified Capabilities

- `tui`: NLP input mode now displays conversational model responses instead of only filter/update results

## Impact

- `src/nlp.rs`: New `NlpAction::Message` variant, updated prompt, updated parser
- `src/tui.rs`: Handle `NlpAction::Message` in the NLP result match arm, display text to user
- No API changes, no dependency changes, no breaking changes
