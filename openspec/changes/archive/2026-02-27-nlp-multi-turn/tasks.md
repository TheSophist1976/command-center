## 1. NLP module: API and data model changes (src/nlp.rs)

- [x] 1.1 Add `ShowTasks { task_ids: Vec<u32>, text: String }` variant to `NlpAction` enum
- [x] 1.2 Make `ApiMessage` public so TUI can build message history
- [x] 1.3 Change `call_claude_api` signature to accept `messages: &[ApiMessage]` instead of `user_input: &str`
- [x] 1.4 Change `interpret` signature to accept `messages: &[ApiMessage]` instead of `input: &str`, build system prompt internally, pass messages through to `call_claude_api`
- [x] 1.5 Add `task_ids` and `text` fields to `RawAction` struct for deserializing `show_tasks` responses
- [x] 1.6 Add `"show_tasks"` arm to `parse_response` that returns `NlpAction::ShowTasks`
- [x] 1.7 Update system prompt to include `show_tasks` JSON format and instructions for multi-turn context usage
- [x] 1.8 Make `call_claude_api` return the raw response text alongside the parsed action (or separately) so the TUI can append it to `nlp_messages` as assistant content

## 2. TUI: Conversation state and mode (src/tui.rs)

- [x] 2.1 Add `ChatMessage` enum with variants: `User(String)`, `Assistant(String)`, `TaskList { text: String, tasks: Vec<(u32, String, String, String)> }`, `Error(String)`
- [x] 2.2 Add `chat_history: Vec<ChatMessage>` and `nlp_messages: Vec<ApiMessage>` fields to `App` struct
- [x] 2.3 Replace `NlpInput` mode with `NlpChat` in the `Mode` enum
- [x] 2.4 Update `:` key handler to enter `NlpChat` mode and initialize empty conversation state
- [x] 2.5 Update `Esc` in `NlpChat` to clear `chat_history`, clear `nlp_messages`, and return to Normal mode

## 3. TUI: NlpChat input handler (src/tui.rs)

- [x] 3.1 Refactor `handle_nlp_input` into `handle_nlp_chat`: on Enter, append user message to `chat_history`, build `ApiMessage` and append to `nlp_messages`, call `nlp::interpret` with full `nlp_messages`
- [x] 3.2 On successful response, append assistant's raw text to `nlp_messages` as an assistant `ApiMessage`
- [x] 3.3 Handle `NlpAction::Filter`: apply filter to table, append assistant message to `chat_history`, stay in `NlpChat`
- [x] 3.4 Handle `NlpAction::Message`: append text to `chat_history` as `Assistant`, stay in `NlpChat`
- [x] 3.5 Handle `NlpAction::ShowTasks`: resolve task IDs against current task list, append `TaskList` entry to `chat_history`, stay in `NlpChat`
- [x] 3.6 Handle `NlpAction::Update`: enter `ConfirmingNlp`, after confirm/cancel return to `NlpChat` (not Normal)
- [x] 3.7 Handle errors: append `Error` entry to `chat_history`, stay in `NlpChat`
- [x] 3.8 Cap `nlp_messages` at 20 entries, dropping oldest when exceeded

## 4. TUI: Split layout and chat panel rendering (src/tui.rs)

- [x] 4.1 Update `draw` function to detect `NlpChat` mode and switch to 4-region layout: header (1), table (~60%), chat panel (~40%), input prompt (1)
- [x] 4.2 Implement `draw_chat_panel` function: render `chat_history` as styled `Paragraph` with user messages prefixed `> ` and assistant messages unstyled
- [x] 4.3 Render `TaskList` entries in chat panel as indented compact rows: `  #ID Title [priority] (status)`
- [x] 4.4 Auto-scroll chat panel to bottom when new messages are added
- [x] 4.5 Render input prompt line in NlpChat mode: `> {input_buffer}_`

## 5. Tests

- [x] 5.1 Add unit test for `parse_response` with valid `show_tasks` JSON
- [x] 5.2 Add unit test for `parse_response` with `show_tasks` and empty `task_ids`
- [x] 5.3 Verify existing filter, update, and message parse tests still pass
- [x] 5.4 Add TUI test: `:` key enters NlpChat mode
- [x] 5.5 Add TUI test: `Esc` in NlpChat clears conversation and returns to Normal
