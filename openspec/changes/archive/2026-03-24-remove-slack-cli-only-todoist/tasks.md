## 1. Remove Slack module

- [x] 1.1 Delete `src/slack.rs`
- [x] 1.2 Remove `mod slack;` and all `use crate::slack` imports from `src/lib.rs` and any other files
- [x] 1.3 Verify `cargo build` compiles after module removal (fix any broken imports)

## 2. Remove Slack from CLI auth

- [x] 2.1 Remove `AuthCommand::Slack` variant from the `AuthCommand` enum in `src/cli.rs`
- [x] 2.2 Remove the `Slack` match arm from the `auth` subcommand handler in `src/cli.rs`
- [x] 2.3 Remove Slack token read/write/revoke functions from `src/auth.rs`
- [x] 2.4 Update `task auth status` handler to no longer report Slack token presence
- [x] 2.5 Update `task auth revoke` handler to no longer delete Slack token

## 3. Remove Slack from TUI

- [x] 3.1 Remove Slack inbox view and all associated rendering code from `src/tui.rs`
- [x] 3.2 Remove Slack reply panel and state from `src/tui.rs`
- [x] 3.3 Remove Slack unread-sync background task from `src/tui.rs`
- [x] 3.4 Remove Slack channel auto-fetch state and logic from `src/tui.rs`
- [x] 3.5 Remove all Slack-related keybindings and match arms from the TUI event loop
- [x] 3.6 Remove Slack-related fields from the `App` struct

## 4. Remove Todoist import from TUI

- [x] 4.1 Remove the `i` keybinding handler that calls `todoist::run_import` from `src/tui.rs`
- [x] 4.2 Assess whether `status_message: Option<String>` on `App` is used by any remaining TUI feature; remove if unused

## 5. Remove Slack from deploy script

- [x] 5.1 Remove `STATUS_SLACK=""` variable from `deploy.sh`
- [x] 5.2 Remove the Slack Bot Token setup section (lines ~322–361) from `deploy.sh`
- [x] 5.3 Remove the `Slack:` summary line from the final summary block in `deploy.sh`

## 6. Clean up dependencies and tests

- [x] 6.1 Remove Slack-related test cases from `tests/integration.rs`
- [x] 6.2 Review `Cargo.toml` for any Slack-only dependencies and remove them
- [x] 6.3 Run `cargo build` — confirm zero errors
- [x] 6.4 Run `cargo test` — confirm all tests pass
