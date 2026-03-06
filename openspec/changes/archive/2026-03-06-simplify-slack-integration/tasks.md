## 1. Remove NLP/AI Slack code

- [x] 1.1 Delete `SlackSuggestion`, `RawSuggestion`, `build_slack_analysis_prompt`, `analyze_slack_messages`, `parse_slack_suggestions` and related imports from `src/slack.rs`
- [x] 1.2 Delete all NLP-related tests from `src/slack.rs` (`test_parse_valid_suggestions`, `test_parse_partially_malformed`, `test_parse_non_json`, `test_parse_empty_array`, `test_parse_invalid_priority_defaults_to_medium`, `test_parse_critical_priority`)
- [x] 1.3 Remove `SlackReview`, `SlackReviewEditing` modes and all associated state fields (`slack_suggestions`, `slack_selected`) from `src/tui.rs`
- [x] 1.4 Remove `handle_slack_review`, `accept_slack_suggestion`, `flush_slack_hwm`, `draw_slack_review` functions from `src/tui.rs`
- [x] 1.5 Remove `start_slack_review` function and its Claude API key dependency from `src/tui.rs`
- [x] 1.6 Verify the project compiles after removals (`cargo check`)

## 2. Slack message storage (inbox.md)

- [x] 2.1 Define `SlackInboxMessage` struct with fields: channel_id, channel_name, user_id, user_name, text, ts, status (open/done), link
- [x] 2.2 Define `SlackInbox` struct with fields: workspace, last_sync, hwm (HashMap<String,String>), messages (Vec<SlackInboxMessage>)
- [x] 2.3 Implement `inbox_path()` returning `~/.config/task-manager/slack/inbox.md`
- [x] 2.4 Implement `load_inbox()` — parse the inbox.md format into `SlackInbox`
- [x] 2.5 Implement `save_inbox()` — serialize `SlackInbox` to markdown, write atomically (temp file + rename)
- [x] 2.6 Implement deep link construction: `https://{workspace}.slack.com/archives/{channel}/p{ts_no_dot}`
- [x] 2.7 Implement `fetch_workspace_name()` via Slack `auth.test` API, cache in config as `slack-workspace`
- [x] 2.8 Implement deduplication logic in sync (skip messages whose channel+ts already exist)
- [x] 2.9 Implement done-message pruning (remove `status:done` messages older than 7 days)
- [x] 2.10 Write tests for `load_inbox` / `save_inbox` roundtrip
- [x] 2.11 Write tests for deep link construction
- [x] 2.12 Write tests for deduplication and pruning logic

## 3. Slack reply API

- [x] 3.1 Add `send_message(token, channel_id, text)` function to `src/slack.rs` using `chat.postMessage`
- [x] 3.2 Handle error responses: `not_in_channel`, `missing_scope`, rate limiting (429)
- [x] 3.3 Write tests for `send_message` error handling (deserialization of error responses)

## 4. TUI: Slack inbox mode

- [x] 4.1 Add `SlackInbox` and `SlackReplying` to the `Mode` enum in `src/tui.rs`
- [x] 4.2 Add inbox state fields to `App`: `slack_inbox: SlackInbox`, `slack_inbox_selected: usize`, `slack_reply_buffer: String`
- [x] 4.3 Implement `s` key handler in Normal mode — sync messages and enter SlackInbox (or open channel picker if no channels configured)
- [x] 4.4 Implement `S` key handler in Normal mode — open channel picker
- [x] 4.5 Implement sync logic: fetch new messages per channel, resolve user names, build inbox entries with deep links, save inbox.md
- [x] 4.6 Implement `handle_slack_inbox()` — key handlers for j/k navigation, Enter/d (mark done), o (open link), r (enter reply mode), S (re-sync), Esc (exit)
- [x] 4.7 Implement `handle_slack_replying()` — text input, Enter (send via `send_message`), Esc (cancel)
- [x] 4.8 Implement `draw_slack_inbox()` — render table with Channel, Sender, Message, Time columns; header with message count; footer with keybinding hints
- [x] 4.9 Implement relative timestamp display (e.g., "2h ago", "yesterday", "3d ago")
- [x] 4.10 Implement `open` command to launch deep link in system browser

## 5. Cleanup and integration

- [x] 5.1 Remove old `slack_state.json` HWM functions (`hwm_path`, `read_hwm_state`, `write_hwm_state`, `read_hwm`, `write_hwm`) from `src/slack.rs`
- [x] 5.2 Remove old HWM tests from `src/slack.rs`
- [x] 5.3 Update footer keybinding hints in Normal mode to show `s:slack` instead of previous Slack key
- [x] 5.4 Update `src/main.rs` / `src/cli.rs` if any Slack CLI subcommands reference the removed NLP flow
- [x] 5.5 Run full test suite (`cargo test`) and fix any breakage
- [ ] 5.6 Manual smoke test: auth, sync, browse inbox, mark done, reply, open link, re-sync
