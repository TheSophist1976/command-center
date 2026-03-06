## Why

Users receive actionable messages in Slack — requests, follow-ups, deadlines — but these often get lost in the stream. Command Center currently has no way to pull external messages into the task workflow. A Slack integration that reads recent/unread channel messages and uses the NLP engine to suggest action items would close this gap, turning Slack noise into tracked tasks without leaving the TUI.

## What Changes

- Add Slack OAuth bot token authentication (`task auth slack`) with secure credential storage following the existing Todoist/Claude auth pattern
- Add a Slack client module that fetches conversation history from configured channels using the `conversations.history` API, with high-water-mark tracking to fetch only new messages since last check
- Add a new TUI mode for reviewing Slack messages and AI-suggested action items — the user can accept (create task), skip, or edit each suggestion before committing
- Integrate with the existing NLP engine (Claude) to analyze batches of Slack messages and propose task titles, priorities, and due dates
- Add a `conversations.list` call to let users pick which Slack channels to monitor

## Capabilities

### New Capabilities
- `slack-auth`: Slack bot token storage, validation, and credential management
- `slack-client`: HTTP client for Slack API (conversations.list, conversations.history), pagination, high-water-mark tracking for "new messages since last check"
- `slack-tui`: TUI mode for reviewing Slack messages, displaying AI-suggested action items, and accepting/skipping/editing suggestions into tasks
- `slack-nlp`: NLP prompt and action types for analyzing Slack messages and generating task suggestions (title, priority, due date)

### Modified Capabilities
- `tui`: Add `S` keybinding in normal mode to trigger Slack review flow
- `cli-interface`: Add `task auth slack` subcommand

## Impact

- **New files**: `src/slack.rs` (client + auth), new TUI mode/handler code in `src/tui.rs`
- **Modified files**: `src/auth.rs` (slack token storage), `src/main.rs` (auth subcommand), `src/tui.rs` (keybinding + mode), `src/nlp.rs` (new prompt/action types), `deploy.sh` (add Slack token setup step alongside Todoist/Claude)
- **New dependency**: None — uses `ureq` (already in use for Todoist) for HTTP calls to Slack API
- **External dependency**: Slack Bot Token with `channels:history`, `channels:read` scopes; requires a Slack App created in the user's workspace
- **Config**: Channel list stored in config file; high-water-mark timestamps stored per-channel in a local state file
