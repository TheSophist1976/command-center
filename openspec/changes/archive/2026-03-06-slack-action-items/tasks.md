## 1. Slack Auth

- [x] 1.1 Add `slack_token_path()`, `read_slack_token()`, `write_slack_token()`, `delete_slack_token()` functions to `src/auth.rs`, following the existing Todoist/Claude pattern. `read_slack_token()` checks `SLACK_BOT_TOKEN` env var first, then falls back to file.
- [x] 1.2 Add `prompt_for_slack_token(token_flag: Option<String>)` to `src/auth.rs` — validates `xoxb-` prefix, prints setup instructions with link to `https://api.slack.com/apps` and required scopes
- [x] 1.3 Add `Slack` variant to the `Auth` CLI enum in `src/cli.rs` with optional `--token` flag, wire up handler in `src/main.rs`
- [x] 1.4 Update `auth status` handler to include Slack token status, update `auth revoke` to delete Slack token
- [x] 1.5 Add unit tests for slack auth functions (path, read from env, read from file, write, delete, prompt validation)

## 2. Slack Client Module

- [x] 2.1 Create `src/slack.rs` with `SlackMessage`, `SlackChannel`, and paginated response wrapper structs (serde Deserialize). Add `mod slack;` to `src/main.rs`.
- [x] 2.2 Add `api_base_url()` helper reading `SLACK_API_BASE_URL` env var (default `https://slack.com/api`)
- [x] 2.3 Implement `fetch_channels(token: &str) -> Result<Vec<SlackChannel>, String>` calling `conversations.list` with pagination
- [x] 2.4 Implement `fetch_messages(token: &str, channel_id: &str, oldest: Option<&str>, limit: usize) -> Result<Vec<SlackMessage>, String>` calling `conversations.history`
- [x] 2.5 Implement `read_hwm(channel_id: &str) -> Option<String>` and `write_hwm(channel_id: &str, ts: &str)` using `slack_state.json` in the config dir
- [x] 2.6 Implement `fetch_new_messages(token: &str, channel_ids: &[String]) -> Result<Vec<(String, Vec<SlackMessage>)>, String>` — iterates channels, reads HWM, fetches new messages, returns grouped by channel. Skips errored channels with warnings.
- [x] 2.7 Add unit tests for HWM read/write, API type deserialization, and `api_base_url()` override

## 3. Slack NLP Analysis

- [x] 3.1 Add `SlackSuggestion` struct to `src/slack.rs` (or `src/nlp.rs`) with fields: `title`, `priority`, `due_date`, `source_channel`, `source_text`
- [x] 3.2 Implement `analyze_slack_messages(messages: &[(String, String, String)], api_key: &str, today: &str) -> Result<Vec<SlackSuggestion>, String>` — builds system prompt, sends to Claude, parses JSON response
- [x] 3.3 Write the Slack analysis system prompt: instruct Claude to identify actionable messages, return JSON array with title/priority/due_date/source_channel/source_text, include today's date for relative date resolution
- [x] 3.4 Implement response parsing: deserialize JSON array into `Vec<SlackSuggestion>`, skip malformed entries, parse priority strings using existing `Priority::from_str`
- [x] 3.5 Add unit tests for response parsing (valid JSON, partially malformed, non-JSON fallback)

## 4. TUI Slack Review Mode

- [x] 4.1 Add `Mode::SlackReview` and `Mode::SlackChannelPicker` variants to the Mode enum. Add App fields: `slack_suggestions: Vec<SlackSuggestion>`, `slack_selected: usize`, `slack_channels: Vec<SlackChannel>`, `slack_channel_selected: Vec<bool>`, `slack_channel_idx: usize`, `slack_reviewed_ts: HashMap<String, String>`
- [x] 4.2 Add `S` keybinding in `handle_normal`: check for Slack token → check for configured channels → enter `SlackChannelPicker` if no channels or `SlackReview` if channels exist. Show status message if no token.
- [x] 4.3 Implement `handle_slack_review` key handler: `j`/`k` navigation, `Enter` to accept (create task + remove from list + track HWM), `s` to skip (remove + track HWM), `e` to enter edit mode (pre-fill input buffer), `Esc` to exit (update HWMs for reviewed messages, return to Normal)
- [x] 4.4 Implement `draw_slack_review` rendering: header with "Slack Review — N suggestions", suggestion list with title/priority/channel/snippet, footer with keybinding hints
- [x] 4.5 Implement Slack fetch + NLP analysis flow on entering SlackReview: spawn blocking fetch in background, show spinner, then spawn NLP analysis, show spinner, then populate `slack_suggestions`
- [x] 4.6 Implement accept logic: create new task with suggested title, priority, due_date; set description to "From Slack #channel: <message>"; save task file; remove suggestion from list; if list empty, show "All suggestions reviewed" and return to Normal
- [x] 4.7 Implement edit-before-accept: on `e`, enter inline edit mode with title in input buffer; `Enter` accepts with edited title; `Esc` cancels back to suggestion list
- [x] 4.8 Implement HWM update on exit: for each reviewed suggestion, track the latest `ts` per channel; on `Esc` or completion, call `write_hwm` for each channel with reviewed messages

## 5. TUI Channel Picker Mode

- [x] 5.1 Implement `handle_slack_channel_picker` key handler: `j`/`k` navigation, `Space` to toggle channel, `Enter` to save selection to config and proceed to SlackReview, `Esc` to cancel
- [x] 5.2 Implement `draw_slack_channel_picker` rendering: list channels with `[x]`/`[ ]` indicators, header "Select Slack Channels", footer hints `j/k:nav  Space:toggle  Enter:save  Esc:cancel`
- [x] 5.3 On `Enter` in channel picker: write selected channel IDs to config as `slack-channels` (comma-separated), then trigger the Slack fetch flow

## 6. Deploy Script & Footer

- [x] 6.1 Add Slack token setup section to `deploy.sh` between Claude and Summary sections: check if `slack_token` file exists, prompt if not, validate `xoxb-` prefix, save with `chmod 600`. Add `STATUS_SLACK` tracking variable and include in summary output.
- [x] 6.2 Update the TUI normal-mode footer hints string to include `S:slack`
- [x] 6.3 Update README.md: add Slack integration section documenting setup (create Slack App, required scopes, `task auth slack`), TUI usage (`S` key), and channel configuration
