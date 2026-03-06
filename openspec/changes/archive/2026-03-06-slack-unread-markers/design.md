## Context

The Slack integration currently tracks read position using a custom high-water-mark (HWM) stored in `inbox.md` as `<!-- hwm:CHANNEL:TS -->` comments. This is completely independent of Slack's native read cursor. The `conversations.info` API provides a `last_read` field and `conversations.mark` can update the read cursor. Using these native markers means the TUI shows only messages the user hasn't read in Slack, and marking done syncs back.

The current sync flow requires manual channel selection via a picker (`S` key) before the first `s` sync. This adds friction — the user just wants to see unread messages.

## Goals / Non-Goals

**Goals:**
- Use Slack's `last_read` as the source of truth for what's unread
- Auto-discover channels with unread messages (no manual setup required)
- Sync read state back to Slack when marking messages done
- Reduce API calls by skipping channels with no unread messages

**Non-Goals:**
- Real-time message streaming or websocket connections
- Thread/reply unread tracking (only top-level channel messages)
- Removing the channel picker entirely (keep it as `S` for pinning channels)

## Decisions

### 1. Use `conversations.info` for `last_read`

**Decision:** For each member channel, call `conversations.info` to get the `last_read` timestamp. Then call `conversations.history` with `oldest=last_read` to fetch only unread messages. This replaces the custom HWM system.

**Why:** Slack's `last_read` reflects the user's actual read position across all Slack clients (desktop, mobile, web). Our custom HWM only tracked what the TUI fetched, not what the user actually read.

### 2. Auto-discover unread channels

**Decision:** When the user presses `s`, fetch `conversations.list` (member channels only), then call `conversations.info` on each to check for unread messages. Only fetch history from channels where `last_read < latest.ts` (i.e., there are unread messages).

The flow:
1. `conversations.list` with `types=public_channel,private_channel,mpim,im` → get all member channels
2. For each channel, `conversations.info` → get `last_read` and `latest.ts`
3. Filter to channels where `last_read < latest.ts` (has unread)
4. For those channels, `conversations.history` with `oldest=last_read` → fetch unread messages

**Trade-off:** This adds N `conversations.info` calls (one per member channel). For users in many channels, this could be slow. Mitigation: the background thread + spinner keeps the UI responsive, and we can batch-skip channels efficiently.

### 3. Sync read state back with `conversations.mark`

**Decision:** When the user marks a message as "done" in the TUI, call `conversations.mark` with the message's `ts` to update Slack's read cursor for that channel. Only mark if the message's `ts` is greater than the channel's current `last_read` (to avoid moving the cursor backward).

**Why:** This closes the loop — reading in the TUI marks as read in Slack. Without this, the same messages would appear unread again on next sync.

**Scope requirement:** Needs `channels:write`, `groups:write`, `im:write`, `mpim:write` scopes. The auth setup instructions will be updated.

### 4. Remove HWM from inbox file

**Decision:** Remove `<!-- hwm:CHANNEL:TS -->` lines from the inbox markdown format. The `hwm` field is removed from `SlackInbox` struct. Slack's `last_read` is now the sole source of truth for tracking read position.

**Why:** The HWM was a workaround for not having access to Slack's read state. With native `last_read`, it's redundant and potentially confusing (two sources of truth).

### 5. Keep channel picker for pinning

**Decision:** The `S` key still opens the channel picker. Selected channels are saved to `slack-channels` config. When `slack-channels` is set, `s` only checks those channels (not all member channels). This lets users focus on specific channels if they prefer.

**Why:** Some users are in hundreds of channels but only care about a few. The picker provides an opt-in filter.

## Risks / Trade-offs

**[API call volume]** → For users in many channels (50+), the `conversations.info` per channel adds up. Tier 3 rate limit (~50 req/min) should be sufficient for most users. If needed, we could add a config option to limit discovery to recent channels only.

**[conversations.mark scope]** → Requires write scopes that weren't needed before. Users will need to update their Slack app's OAuth scopes. The auth flow will show updated instructions.

**[DM vs channel `last_read` behavior]** → The docs note `unread_count` is DM-only, but `last_read` should work for all channel types. If `last_read` is missing for some channel types, we fall back to fetching all recent messages (last 24h).
