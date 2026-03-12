## Context

The Slack inbox sync (`sync_slack_inbox_bg` in `tui.rs`) runs on a background thread spawned from the TUI event loop. It calls into `slack.rs` functions which each create their own `reqwest::blocking::Client`. For N configured channels, the current flow is:

1. `fetch_workspace_name` — 1 API call (auth.test), cached after first run
2. For each channel (sequential):
   - `fetch_channel_info` — 1 API call (conversations.info)
   - `fetch_messages` — 1 API call (conversations.history), skipped if no unread
   - `resolve_users_batch` — 1 API call per unknown user (users.info)
3. Write user cache and inbox to disk

With 15 channels and 5 unknown users, that's ~36 sequential HTTP requests with no connection reuse.

The TUI already runs sync on a background thread via `std::thread::spawn` + `mpsc::channel`, polling the receiver in the event loop. The `bg_task` field on `App` holds at most one background task at a time.

## Goals / Non-Goals

**Goals:**
- Reduce wall-clock time of Slack inbox sync by 3-5x through concurrent API calls
- Reuse TCP/TLS connections across requests within a single sync operation
- Keep the existing TUI background task model (single `bg_task` slot, mpsc result channel)

**Non-Goals:**
- Async runtime (tokio/async-std) — too invasive; the app uses `reqwest::blocking` everywhere
- Persistent connection pooling across syncs — a per-sync shared client is sufficient
- Rate limit backoff/retry logic — out of scope, handle in a future change
- Changing the Slack API endpoints or response handling

## Decisions

### 1. Shared `reqwest::blocking::Client` passed by reference

**Decision:** Create one `Client` at the start of `sync_slack_inbox_bg` and pass `&Client` to all `slack.rs` API functions.

**Rationale:** `reqwest::blocking::Client` holds an internal connection pool. Reusing it means TLS handshakes and TCP connections persist across calls. This is the simplest change with immediate benefit.

**Alternative considered:** Global/static `Client` via `once_cell::Lazy` — rejected because it adds complexity for marginal benefit (the sync already runs on a dedicated thread with a short lifetime).

**Migration:** Every public API function in `slack.rs` that currently creates `reqwest::blocking::Client::new()` gains a `client: &reqwest::blocking::Client` parameter. Call sites in `tui.rs` create the client and pass it through.

### 2. Thread-pool parallelism for per-channel fetches

**Decision:** Use `std::thread::scope` (stable since Rust 1.63) to spawn scoped threads for channel info + message fetching, bounded to 6 concurrent threads.

**Rationale:** `thread::scope` allows borrowing `&Client`, `&str` (token), and other references without `Arc` — the scope guarantees threads complete before references are dropped. A cap of 6 concurrent requests stays well within Slack's Tier 3 rate limits (~50 req/min).

**Alternative considered:** `rayon` parallel iterators — rejected because it adds a dependency for a simple fan-out pattern. Thread pool crate (`threadpool`) — rejected for the same reason when `thread::scope` does exactly what we need with zero dependencies.

**Pattern:**
```
thread::scope(|s| {
    let handles: Vec<_> = channel_ids.chunks(1).map(|chunk| {
        s.spawn(|| {
            // fetch_channel_info + fetch_messages for this channel
        })
    }).collect();
    // join all, collect results
})
```

### 3. Parallel conversation type fetches in `fetch_channels`

**Decision:** Fetch all 4 conversation types (public_channel, private_channel, mpim, im) concurrently using `thread::scope` within `fetch_channels`.

**Rationale:** Currently loops sequentially over types. Each type may require pagination but the initial pages for all 4 types can be fetched in parallel. This speeds up the channel picker and auto-discover flows.

### 4. Batched parallel user resolution

**Decision:** Collect all unknown user IDs across all channels first, deduplicate, then resolve them concurrently (capped at 6 threads) in a single batch after message fetching completes.

**Rationale:** Currently `resolve_users_batch` is called per-channel inside the channel loop, making it impossible to deduplicate across channels. Moving it after the channel loop avoids duplicate lookups and enables parallelism.

**Alternative considered:** Slack `users.list` to bulk-fetch all workspace users — rejected because it can return thousands of users for large workspaces and requires pagination. Targeted `users.info` calls for only the users we need are more efficient.

## Risks / Trade-offs

**[Risk] Slack rate limits hit faster with concurrent requests** → Cap concurrency at 6 threads. Slack Tier 3 allows ~50 requests/minute; 6 concurrent requests for 15 channels completes in 2-3 waves, well within limits.

**[Risk] Error in one channel thread could panic** → Each thread returns `Result`; errors are collected and logged per-channel (matching current behavior of `eprintln!` + continue).

**[Risk] Thread overhead for small channel counts** → For 1-3 channels, thread spawning has negligible overhead (~μs) and the code path naturally serializes. No special casing needed.

**[Trade-off] API function signatures change** → Every `slack.rs` public API function gains a `client` parameter. This is a mechanical change but touches many call sites. Worth it for connection reuse.
