## Why

The current Slack integration requires users to manually select channels via a picker before syncing messages. It also tracks read position using a custom high-water-mark (HWM) in the inbox file, which is completely separate from Slack's own read state. This means the TUI shows messages the user already read in Slack, and marking messages "done" in the TUI doesn't update Slack's read cursor. The sync should use Slack's native `last_read` marker so only truly unread messages appear, and marking them done should sync read state back to Slack.

## What Changes

- Replace custom HWM tracking with Slack's `last_read` timestamp from `conversations.info`
- Auto-discover channels with unread messages on sync — no manual channel selection required for the `s` key
- Only fetch messages newer than `last_read` from each channel (truly unread)
- Call `conversations.mark` when user marks a message done in the TUI to sync read state back to Slack
- Keep the channel picker (`S` key) as an optional override for pinning specific channels to always check
- Skip channels that have zero unread messages to reduce API calls

## Capabilities

### New Capabilities
- `slack-unread-sync`: Fetch unread messages using Slack's native read cursor (`conversations.info` → `last_read`) and sync read state back via `conversations.mark`

### Modified Capabilities
- `slack-message-storage`: Remove custom HWM tracking from inbox file, replace with Slack's `last_read` as the source of truth
- `slack-auto-channels`: Auto-discover now filters to channels with unread messages, not just member channels

## Impact

- **Code modified**: `src/slack.rs` — new `fetch_unread_channels`, `mark_read` functions; remove HWM read/write from inbox format. `src/tui.rs` — update `sync_slack_inbox_bg` to use new unread-based flow, call `mark_read` on done.
- **API changes**: New API calls to `conversations.info` (per channel) and `conversations.mark` (on done). Additional OAuth scopes may be needed: `channels:write`, `groups:write`, `im:write`, `mpim:write` for `conversations.mark`.
- **Storage change**: Inbox markdown file no longer contains `<!-- hwm:... -->` lines. The `last_read` from Slack is the source of truth.
- **No new dependencies**: Uses existing `reqwest::blocking` client.
