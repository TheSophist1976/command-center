## Why

The current Slack integration only monitors public channels and requires users to manually select which ones via a channel picker. Action items from DMs, group DMs, and private channels are completely invisible. Users who want to capture action items from all their Slack activity must individually select every public channel they're in — and miss everything else. The feature should automatically pull all unread messages across all conversation types (public channels, private channels, group DMs, and direct messages), removing both the channel selection bottleneck and the conversation type limitation.

## What Changes

- Expand `conversations.list` API call to include all conversation types: `public_channel`, `private_channel`, `mpim` (group DMs), and `im` (direct messages)
- When pressing `S` (Slack review), automatically fetch unread messages from ALL conversations the user is a member of — no channel picker step required
- Keep the channel picker (`C` key) available as an optional override for users who want to limit scope to specific conversations
- Use the existing high-water-mark (HWM) system per-conversation to track "unread" state, so only new messages since the last review are fetched
- Display conversation source context in the review UI: channel name for channels, user name for DMs, participant names for group DMs
- Require additional Slack OAuth scopes: `groups:history` (private channels), `im:history` (DMs), `mpim:history` (group DMs), `groups:read`, `im:read`, `mpim:read`, `users:read` (for resolving user IDs to display names)
- **BREAKING**: The default `S` behavior changes from "use configured channels or open picker" to "fetch all conversations automatically"

## Capabilities

### New Capabilities
- `slack-auto-channels`: Automatic discovery and fetching of all conversations (public/private channels, DMs, group DMs) the user is a member of, replacing mandatory manual channel selection as the default Slack review workflow
- `slack-user-resolve`: Resolution of Slack user IDs to display names for DM and group DM context (needed to show "DM with Alice" instead of "DM with U12345")

### Modified Capabilities
_None — the existing channel picker, HWM tracking, NLP analysis, and review UI remain unchanged. The new capability layers on top by changing how conversation IDs are resolved before the existing fetch pipeline runs._

## Impact

- **src/slack.rs**: Expand `fetch_channels` to request all conversation types (`public_channel,private_channel,mpim,im`); add `users.info` API call for resolving user display names; add conversation display name resolution (channel name vs "DM with X" vs "Group DM: X, Y")
- **src/tui.rs**: Modify `S` key handler to auto-discover all conversations instead of requiring config; `C` key remains for manual override; update review UI to show meaningful source labels for DMs/group DMs
- **Config**: `slack-channels` config key becomes optional (used only when user explicitly picks conversations via `C`)
- **Slack API**: Same `conversations.list` and `conversations.history` endpoints, but with expanded `types` parameter. New `users.info` endpoint for display name resolution. Requires expanded OAuth scopes: `groups:history`, `groups:read`, `im:history`, `im:read`, `mpim:history`, `mpim:read`, `users:read`
- **Auth**: Update `prompt_for_slack_token` setup instructions to list the expanded required scopes
