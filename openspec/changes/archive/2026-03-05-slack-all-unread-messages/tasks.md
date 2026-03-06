## 1. Expand SlackChannel struct and API types

- [x] 1.1 Add `display_name: String`, `conversation_type: String`, and `user: Option<String>` fields to `SlackChannel` struct in `src/slack.rs` with appropriate serde defaults
- [x] 1.2 Update `SlackChannelListResponse` deserialization to handle the expanded fields from the Slack API (the API returns `is_channel`, `is_im`, `is_mpim`, `is_group`, `user` fields on conversation objects)
- [x] 1.3 Add a helper function `resolve_conversation_type` that maps API boolean fields to a type string (`"channel"`, `"private"`, `"im"`, `"mpim"`)
- [x] 1.4 Update existing tests that construct or deserialize `SlackChannel` to include the new fields

## 2. User name resolution and caching

- [x] 2.1 Add `fetch_user_info(token: &str, user_id: &str) -> Result<String, String>` function in `src/slack.rs` that calls Slack `users.info` API and extracts display name (prefer `profile.display_name`, fall back to `profile.real_name`, then `name`)
- [x] 2.2 Add `slack_users_cache_path() -> PathBuf` function returning `<config_dir>/task-manager/slack_users.json`
- [x] 2.3 Add `read_user_cache() -> HashMap<String, String>` and `write_user_cache(cache: &HashMap<String, String>) -> Result<(), String>` functions for file-backed persistence
- [x] 2.4 Add `resolve_user_name(token: &str, user_id: &str, cache: &mut HashMap<String, String>) -> String` that checks cache first, falls back to API, updates cache, and returns raw user ID on failure
- [x] 2.5 Add `resolve_users_batch(token: &str, user_ids: &[String], cache: &mut HashMap<String, String>) -> ()` that resolves all unique user IDs, updating the cache
- [x] 2.6 Add tests for user cache read/write (empty file, existing entries) and `resolve_user_name` fallback behavior

## 3. Expand fetch_channels to all conversation types

- [x] 3.1 Modify `fetch_channels` signature to accept an optional `types` parameter, defaulting to `"public_channel,private_channel,mpim,im"`
- [x] 3.2 After fetching, compute `conversation_type` and populate `display_name` for channels/private channels (using `#name` / `🔒 #name`)
- [x] 3.3 For IM and MPIM conversations, resolve user IDs to display names using the user cache and populate `display_name` as `"DM with <name>"` / `"Group: <name1>, <name2>, ..."`
- [x] 3.4 Update the channel picker (`open_slack_channel_picker`) to use `display_name` instead of raw `ch.name` in the table
- [x] 3.5 Update tests for `fetch_channels` to cover the new types parameter and display name generation

## 4. Auto-discover conversations for S key

- [x] 4.1 Modify the `S` key handler in `handle_normal` to: when no `slack-channels` config is set, call `fetch_channels` with all types, filter by `is_member: true`, and pass all IDs directly to `start_slack_review`
- [x] 4.2 Cache the fetched conversations on `app.slack_channels` during auto-discovery so `start_slack_review` can resolve conversation names
- [x] 4.3 Update `start_slack_review` to use `display_name` instead of `ch.name` when resolving channel names for the NLP analysis prompt
- [x] 4.4 Resolve message author user IDs to display names before sending to NLP analysis (use user cache)

## 5. Graceful scope degradation

- [x] 5.1 Handle `missing_scope` error from Slack API in `fetch_channels` — when a specific conversation type fails, skip it and continue with other types
- [x] 5.2 Return a warning alongside successful results so the TUI can show a status message about missing scopes (e.g., "Some conversation types skipped — update OAuth scopes for full access")
- [x] 5.3 Add test for `missing_scope` error handling

## 6. Update auth setup instructions

- [x] 6.1 Update `prompt_for_slack_token` in `src/auth.rs` to list expanded required scopes: `channels:history`, `channels:read`, `groups:history`, `groups:read`, `im:history`, `im:read`, `mpim:history`, `mpim:read`, `users:read`
- [x] 6.2 Update the existing auth prompt test to verify the new scope list

## 7. Integration testing

- [x] 7.1 Add integration test: auto-discover with mock Slack API returning mixed conversation types
- [x] 7.2 Add integration test: user cache persistence across calls
- [x] 7.3 Verify existing TUI tests still pass with expanded `SlackChannel` struct
