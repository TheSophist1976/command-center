## 1. NLP module changes (src/nlp.rs)

- [x] 1.1 Add `Message(String)` variant to the `NlpAction` enum
- [x] 1.2 Add `text` field to `RawAction` struct for deserializing message responses
- [x] 1.3 Add `"message"` arm to `parse_response` that extracts the `text` field and returns `NlpAction::Message`
- [x] 1.4 Update system prompt to add the `{"action":"message","text":"..."}` format with instructions: use for ambiguous queries, task questions answerable from context, unsupported actions, and clarifications
- [x] 1.5 Update system prompt to explicitly tell the model it can answer questions about tasks using the provided task data (counts, summaries, field queries across id, title, status, priority, tags, due_date, project)

## 2. TUI handling (src/tui.rs)

- [x] 2.1 Add `NlpAction::Message(text)` match arm in the NLP result handler (~line 674) that sets `app.status_message` to the message text and returns to `Mode::Normal`

## 3. Tests

- [x] 3.1 Add unit test for `parse_response` with a valid `{"action":"message","text":"..."}` JSON input
- [x] 3.2 Add unit test for `parse_response` with a message action that has markdown fences
- [x] 3.3 Verify existing filter and update parse tests still pass (no regressions)
