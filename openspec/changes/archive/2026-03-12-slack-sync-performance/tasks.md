## 1. Shared HTTP Client

- [x] 1.1 Add `client: &reqwest::blocking::Client` parameter to all public API functions in `slack.rs`: `fetch_channels`, `fetch_channel_info`, `fetch_messages`, `fetch_user_info`, `fetch_workspace_name`, `mark_read`, `send_message`
- [x] 1.2 Remove `reqwest::blocking::Client::new()` calls inside each function body
- [x] 1.3 Update `resolve_user_name` and `resolve_users_batch` to accept and pass through `&Client`
- [x] 1.4 Update `resolve_channel_display_names` to accept and pass through `&Client`
- [x] 1.5 Update all call sites in `tui.rs` to create a shared `Client` and pass it to slack functions (`sync_slack_inbox_bg`, `auto_discover_sync_bg`, `fetch_channels_bg`, and inline calls)

## 2. Concurrent Per-Channel Fetches

- [x] 2.1 Refactor `sync_slack_inbox_bg` to collect channel fetch work into a closure that returns per-channel results (channel_id, messages, errors)
- [x] 2.2 Replace the sequential `for channel_id in channel_ids` loop with `std::thread::scope` spawning up to 6 concurrent threads using `.chunks()` batching
- [x] 2.3 Collect results from all threads and merge into the inbox (add new messages, track new_count)
- [x] 2.4 Move `resolve_users_batch` call after the channel loop — collect all unknown user IDs from fetched messages first, then resolve in one batch

## 3. Concurrent Conversation Type Fetches

- [x] 3.1 Refactor `fetch_channels` to use `std::thread::scope` for fetching all 4 conversation types concurrently instead of the sequential `for conv_type in &type_list` loop
- [x] 3.2 Collect results and scope_warnings from all type threads, merge into `all_channels` and `scope_warnings`

## 4. Batched Concurrent User Resolution

- [x] 4.1 Refactor `resolve_users_batch` to use `std::thread::scope` for resolving unknown users concurrently (up to 6 threads)
- [x] 4.2 Return resolved mappings from threads and merge into the cache after all threads complete (avoids shared mutable cache across threads)

## 5. Testing and Verification

- [x] 5.1 Update existing integration tests in `tests/integration.rs` for new `&Client` parameter on slack API functions
- [x] 5.2 Verify `cargo build --release` compiles cleanly with no warnings
- [ ] 5.3 Manual test: run TUI, press `s` to enter Slack inbox, confirm sync completes and messages display correctly
- [ ] 5.4 Manual test: verify reply (`r`) and mark-done (`Enter`/`d`) still work after refactor
