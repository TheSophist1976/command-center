## Why

The Slack inbox sync is noticeably slow because all API calls are made sequentially using blocking HTTP. With 10-15 configured channels, entering the Slack inbox requires 20-30+ serial HTTP roundtrips (channel info check + message fetch per channel, plus individual user resolution calls). This makes the TUI feel unresponsive on every sync.

## What Changes

- Reuse a single `reqwest::blocking::Client` instance across all Slack API calls instead of creating a new client per request (enables TCP/TLS connection pooling)
- Parallelize per-channel operations (`conversations.info` + `conversations.history`) using thread-based concurrency so multiple channels are fetched simultaneously
- Parallelize the 4 conversation type fetches in `fetch_channels` (public, private, mpim, im) instead of iterating sequentially
- Batch user ID resolution: collect all unknown user IDs across all channels first, then resolve them concurrently instead of one-by-one

## Capabilities

### New Capabilities

- `slack-http-client`: Shared HTTP client management for Slack API calls — client creation, reuse, and connection pooling

### Modified Capabilities

_(No spec-level requirement changes. All existing behavioral requirements remain unchanged — this is an implementation-only performance optimization. The same API calls are made with the same error handling; only their execution strategy changes from sequential to concurrent.)_

## Impact

- **Code**: `src/slack.rs` (all API functions gain a `&Client` parameter instead of creating their own), `src/tui.rs` (sync functions pass shared client, use threads for parallel fetches)
- **Dependencies**: No new crates needed — `reqwest::blocking::Client` already supports `Clone` for sharing, and `std::thread` provides parallelism
- **APIs**: Same Slack API endpoints, same rate limit handling — just called concurrently
- **Risk**: Slack API rate limits (Tier 3: ~50 req/min for most endpoints) could be hit faster with parallel calls; may need a simple concurrency cap
