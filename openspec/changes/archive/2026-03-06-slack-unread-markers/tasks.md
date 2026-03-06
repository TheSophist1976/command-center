## 1. Add `conversations.info` and `conversations.mark` API functions

- [x] 1.1 Add `ChannelInfo` struct with `last_read: Option<String>` and `latest_ts: Option<String>` fields
- [x] 1.2 Add `fetch_channel_info(token, channel_id) -> Result<ChannelInfo, String>` that calls `conversations.info` and extracts `last_read` and `latest.ts`
- [x] 1.3 Add `mark_read(token, channel_id, ts) -> Result<(), String>` that calls `conversations.mark`
- [x] 1.4 Handle `missing_scope` error in `mark_read` — return descriptive error string mentioning write scopes

## 2. Remove HWM from inbox storage

- [x] 2.1 Remove `hwm` field from `SlackInbox` struct
- [x] 2.2 Update `SlackInbox::new()` to not initialize `hwm`
- [x] 2.3 Update `load_inbox` parser to silently skip `<!-- hwm:... -->` lines (backward compat)
- [x] 2.4 Update `save_inbox` / `serialize_inbox` to not write `<!-- hwm:... -->` lines
- [x] 2.5 Remove `inbox_has_message` HWM-related logic if unused after refactor
- [x] 2.6 Update all `SlackInbox` construction sites in tui.rs (test helpers, etc.) to remove `hwm` field

## 3. Refactor sync to use Slack's `last_read`

- [x] 3.1 Add `fetch_unread_channels(token, channel_ids) -> Result<Vec<(String, String, String)>, String>` that returns `(channel_id, last_read, channel_name)` tuples for channels with unread messages
- [x] 3.2 Update `sync_slack_inbox_bg` to use `fetch_unread_channels` instead of HWM — pass `last_read` as `oldest` to `fetch_messages`
- [x] 3.3 When no `slack-channels` config is set, auto-discover by calling `fetch_channels` then `fetch_unread_channels` on all member channels
- [x] 3.4 When `slack-channels` config is set, call `fetch_unread_channels` on only those channels
- [x] 3.5 Remove HWM update logic from `sync_slack_inbox_bg` (no more `inbox.hwm.insert`)

## 4. Sync read state back on mark-done

- [x] 4.1 In TUI `handle_slack_inbox` Enter/d handler, after marking message done, call `mark_read` in background
- [x] 4.2 Handle `mark_read` errors gracefully — show status message but keep message marked done locally
- [x] 4.3 Only call `mark_read` if the message ts is newer than what Slack already considers read (best-effort)

## 5. Update auth instructions

- [x] 5.1 Update `prompt_for_slack_token` in `src/auth.rs` to list write scopes: `channels:write`, `groups:write`, `im:write`, `mpim:write`

## 6. Testing and verification

- [x] 6.1 Add test for `fetch_channel_info` response parsing (mock JSON with `last_read` and `latest`)
- [x] 6.2 Add test for `mark_read` response parsing (success and error cases)
- [x] 6.3 Update inbox roundtrip test to not use HWM fields
- [x] 6.4 Add test that legacy `<!-- hwm:... -->` lines are silently ignored on load
- [x] 6.5 Run full test suite (`cargo test`) and fix any breakage
