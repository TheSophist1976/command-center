## Context

The current Slack integration in `src/slack.rs` fetches only public channels via `conversations.list` with `types=public_channel`. Users must manually select channels through a TUI picker (`C` key) before reviewing messages (`S` key). The `SlackChannel` struct already has an `is_member` field from the API but it's not used for filtering. Messages are fetched per-channel using high-water-marks (HWM) stored in `slack_state.json`, and channel names are resolved from a cached `app.slack_channels` list. DMs and group DMs have no name — they use user IDs that need resolution via `users.info`.

## Goals / Non-Goals

**Goals:**
- `S` key automatically fetches unread messages from all conversation types the user participates in (public channels, private channels, DMs, group DMs)
- DMs and group DMs display human-readable names ("DM with Alice", "Group: Alice, Bob") instead of raw user IDs
- User name lookups are cached to avoid redundant API calls
- Channel picker (`C` key) still works as an optional scope-limiting override
- Auth setup instructions list the full set of required OAuth scopes

**Non-Goals:**
- Sending messages or reacting from the TUI
- Thread/reply fetching (only top-level messages)
- Real-time/streaming message updates
- Filtering by conversation type in the review UI (all types shown together)

## Decisions

### 1. Expand `fetch_channels` types parameter vs. new function

**Decision**: Modify the existing `fetch_channels` to accept a `types` parameter, defaulting to all types.

**Rationale**: The `conversations.list` API already supports multiple types in a single comma-separated parameter. A new function would duplicate pagination logic. The channel picker and auto-fetch both need the same underlying API call — just with different filters.

**Alternative considered**: Separate `fetch_all_conversations` function. Rejected because it would duplicate the entire pagination loop and error handling.

### 2. User name resolution: `users.info` per-user vs. `users.list` bulk

**Decision**: Use `users.info` per unique user ID with an in-memory cache (HashMap) stored on the App struct, persisted to a JSON file alongside `slack_state.json`.

**Rationale**: `users.list` returns all workspace users which could be thousands, most irrelevant. `users.info` fetches only the users we encounter in messages. A file-backed cache (`slack_users.json`) avoids re-fetching across sessions. The cache maps user ID → display name.

**Alternative considered**: `users.list` bulk fetch. Rejected for large workspaces — too many irrelevant users and higher rate-limit risk.

### 3. Auto-fetch flow for `S` key

**Decision**: When `S` is pressed, check if `slack-channels` config exists. If it does, use those channels (existing behavior). If it doesn't, fetch all member conversations automatically — skip the channel picker entirely.

**Flow**:
1. Read `slack-channels` config
2. If present and non-empty → use those IDs (backward-compatible)
3. If absent/empty → call `fetch_channels` with all types, filter by `is_member: true`, use all resulting IDs
4. Proceed to `start_slack_review` with the resolved IDs

**Rationale**: This preserves backward compatibility for users who already have channels configured while giving new users the zero-config experience. Users can always reconfigure with `C` or clear `slack-channels` to return to auto-discovery.

### 4. Conversation display names

**Decision**: Add a `display_name` field to `SlackChannel` (computed, not from API) that provides a human-readable label:

| Type | `name` field from API | Display name |
|------|----------------------|--------------|
| `public_channel` | channel name | `#general` |
| `private_channel` | channel name | `🔒 #secret-project` |
| `im` | empty/user ID | `DM with Alice` |
| `mpim` | auto-generated | `Group: Alice, Bob, Carol` |

**Rationale**: The Slack API returns different `name` values per type. Public/private channels have meaningful names. IMs have no useful name. MPIMs have auto-generated names like `mpdm-user1--user2--user3-1` which are unreadable. We need to resolve user IDs to names for IMs and MPIMs.

### 5. SlackChannel struct expansion

**Decision**: Add an optional `conversation_type` field to `SlackChannel` to track what kind of conversation it is, derived from API response fields (`is_channel`, `is_im`, `is_mpim`). Also add an optional `user` field (present for IMs — the other participant's user ID).

```rust
pub struct SlackChannel {
    pub id: String,
    pub name: String,
    pub is_member: bool,
    pub display_name: String,       // Human-readable: "#general", "DM with Alice", etc.
    pub conversation_type: String,  // "channel", "private", "im", "mpim"
    pub user: Option<String>,       // For IMs: the other user's ID
}
```

### 6. User cache storage

**Decision**: Store user cache at `<config_dir>/task-manager/slack_users.json` as `{"U12345": "Alice", "U67890": "Bob"}`. Load on Slack review start, save after resolution. Cache entries don't expire (names rarely change).

## Risks / Trade-offs

**[Rate limiting with many conversations]** → The auto-fetch fetches history for ALL member conversations. Users in 100+ channels could hit Slack's rate limits. Mitigation: Slack's `conversations.history` tier 3 rate limit is ~50 req/min. We already handle `429 Too Many Requests` errors. For v1, accept partial results with a warning. Future: add configurable conversation limit or parallel fetch with backoff.

**[Large message volume]** → Fetching from all conversations at once could produce hundreds of messages, leading to long NLP analysis times and many suggestions. Mitigation: The existing `limit: 200` per conversation caps individual channels. The NLP analysis already handles large batches. For v1, accept the volume.

**[Stale user cache]** → Cached display names could become outdated if users change their Slack display name. Mitigation: Names rarely change. Cache is simple JSON that can be deleted manually. Not worth the complexity of TTL for v1.

**[Missing OAuth scopes]** → Existing users with only `channels:history` and `channels:read` scopes will get errors when trying to fetch DMs/private channels. Mitigation: Handle `missing_scope` API errors gracefully — skip conversation types that fail and show a status message suggesting scope updates. Don't block the entire flow for a missing scope.

**[Backward compatibility]** → Changing `S` default behavior could surprise existing users. Mitigation: Users with `slack-channels` already configured keep their existing behavior. Only users without config get auto-discovery. The `C` key is always available to reconfigure.
