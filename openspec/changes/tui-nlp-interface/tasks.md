## 1. Claude Auth Infrastructure

- [x] 1.1 Add `read_claude_key()` to `auth.rs`: check `ANTHROPIC_API_KEY` env var first, then fall back to `{config_dir}/task-manager/claude_api_key` file
- [x] 1.2 Add `write_claude_key(key)` and `delete_claude_key()` to `auth.rs` following the same pattern as Todoist token (0600 permissions)
- [x] 1.3 Add `Claude` variant to `AuthCommand` enum in `cli.rs` with optional `--key` flag
- [x] 1.4 Implement `task auth claude` handler in `main.rs`: prompt for key or accept `--key` flag, call `write_claude_key()`
- [x] 1.5 Update `task auth status` handler to report Claude API key status (present/present (env)/not set)
- [x] 1.6 Update `task auth revoke` handler to also delete Claude API key
- [x] 1.7 Add unit tests for `read_claude_key`, `write_claude_key`, `delete_claude_key` (file and env var paths)

## 2. NLP Module

- [x] 2.1 Create `src/nlp.rs` with `NlpAction` enum: `Filter { project, status, priority, tag, title_contains }` and `Update { match_criteria, set_fields, description }`
- [x] 2.2 Implement `build_task_context(tasks)` helper: serialize up to 200 tasks as compact JSON (id, title, status, priority, tags, due_date, project)
- [x] 2.3 Implement `build_system_prompt(task_context)`: construct system prompt instructing Claude to return a JSON action object with the fixed schema
- [x] 2.4 Implement `call_claude_api(api_key, system_prompt, user_input) -> Result<String, String>`: blocking POST to `https://api.anthropic.com/v1/messages` using `claude-haiku-4-5-20251001`
- [x] 2.5 Implement `parse_response(json_str) -> Result<NlpAction, String>`: parse Claude's JSON response into `NlpAction`
- [x] 2.6 Implement public `interpret(tasks, input, api_key) -> Result<NlpAction, String>`: orchestrate build_task_context → build_system_prompt → call_claude_api → parse_response
- [x] 2.7 Add `mod nlp;` to `main.rs`
- [x] 2.8 Add unit tests for `build_task_context` (truncation at 200), `parse_response` (valid filter, valid update, invalid JSON)

## 3. TUI Mode + Keybinding

- [x] 3.1 Add `NlpInput` and `ConfirmingNlp` variants to the `Mode` enum in `tui.rs`
- [x] 3.2 Add `pending_nlp_update: Option<(NlpAction, Vec<usize>)>` field to `App` struct for holding a pending bulk update and its matching task indices
- [x] 3.3 Add `KeyCode::Char(':')` match arm in `handle_normal` to enter `NlpInput` mode and clear input buffer
- [x] 3.4 Add `NlpInput` handling in `handle_key`: dispatch to new `handle_nlp_input` function

## 4. NLP Input Handler

- [x] 4.1 Implement `handle_nlp_input(app, key)`: on Enter, read Claude key via `auth::read_claude_key()`, call `nlp::interpret()`, process result; on Esc, return to Normal; otherwise append to input buffer
- [x] 4.2 Handle `NlpAction::Filter` result: convert to `Filter` struct fields + `title_contains` matching, apply filter, clamp selection, return to Normal
- [x] 4.3 Handle `NlpAction::Update` result: compute matching task indices, if zero matches show status message, otherwise store pending update and enter `ConfirmingNlp` mode
- [x] 4.4 Handle interpret error: set status_message with error text, return to Normal

## 5. NLP Confirmation Handler

- [x] 5.1 Add `ConfirmingNlp` handling in `handle_key`: dispatch to new `handle_nlp_confirm` function
- [x] 5.2 Implement `handle_nlp_confirm(app, key)`: on `y`, apply the pending update (set fields on all matching tasks), save, clamp selection, set status message with count; on any other key, cancel and return to Normal
- [x] 5.3 Clear `pending_nlp_update` after confirmation or cancellation

## 6. TUI Rendering Updates

- [x] 6.1 Add `title_contains: Option<String>` field to `Filter` struct; update `Filter::matches` to check case-insensitive substring match on title; update `Filter::summary` to include it
- [x] 6.2 Update `draw_footer` for `NlpInput` mode: display ` > {input}_ `
- [x] 6.3 Update `draw_footer` for `ConfirmingNlp` mode: display pending action description + task count + "y/n"
- [x] 6.4 Add `::command` to Normal mode footer keybinding hints

## 7. Tests

- [x] 7.1 Add unit test: `:` key enters NlpInput mode
- [x] 7.2 Add unit test: Esc in NlpInput mode returns to Normal
- [x] 7.3 Add unit test: `title_contains` filter matches case-insensitively
- [x] 7.4 Add unit test: `NlpAction::Filter` parsing from valid JSON
- [x] 7.5 Add unit test: `NlpAction::Update` parsing from valid JSON
- [x] 7.6 Add integration test: `task auth claude --key` stores and `task auth status` reports it
