## Why

The TUI has grown to include features (Slack inbox, Todoist import trigger) that belong in the CLI, creating unnecessary complexity and coupling in the terminal UI. Removing Slack entirely and restricting Todoist import to a CLI-only command simplifies the codebase and aligns each interface with its appropriate responsibilities.

## What Changes

- **BREAKING**: Remove all Slack functionality from the codebase — `src/slack.rs`, all Slack auth/API/inbox/reply/unread-sync code, and all Slack-related TUI views
- **BREAKING**: Remove `task auth slack` subcommand and all Slack auth storage/revocation
- Remove `tui-todoist-import` — the Todoist import trigger currently accessible from the TUI
- Retain `task import todoist` as a CLI-only command (no change to its behavior)
- Clean up `cli-interface` spec: remove `auth slack` sub-subcommand and any references to Slack

## Capabilities

### New Capabilities
<!-- None — this is a removal change -->

### Modified Capabilities
- `cli-interface`: Remove `auth slack` subcommand and any Slack-related CLI requirements
- `tui-todoist-import`: **Remove this capability entirely** — Todoist import is CLI-only going forward

## Impact

- `src/slack.rs` — deleted entirely
- `src/tui.rs` — remove Slack inbox view, reply panel, channel sync, and Todoist import trigger from TUI
- `src/cli.rs` — remove `auth slack` subcommand and Slack token handling
- `src/auth.rs` — remove Slack token storage/revocation
- `Cargo.toml` — remove any Slack-specific dependencies if applicable
- `openspec/specs/slack-*/` — all Slack specs become obsolete (no longer enforced)
- `tests/integration.rs` — remove Slack-related test cases
