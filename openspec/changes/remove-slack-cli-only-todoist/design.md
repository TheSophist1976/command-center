## Context

The codebase currently has two features that need to be reorganized:

1. **Slack integration** — `src/slack.rs` provides Slack API types and HTTP calls. The TUI has an inbox view, reply panel, unread sync, and task-create-from-message. The CLI has `task auth slack`. This is a significant surface area that will be removed entirely.

2. **Todoist import in TUI** — The TUI exposes `i` keybinding to trigger Todoist import inline. The CLI already has `task import todoist`. The TUI trigger will be removed; the CLI command is unchanged.

## Goals / Non-Goals

**Goals:**
- Delete `src/slack.rs` and all code that imports it
- Remove all Slack-related TUI views, keybindings, and state from `src/tui.rs`
- Remove `AuthCommand::Slack` variant and all token storage/revocation for Slack from `src/cli.rs` and `src/auth.rs`
- Remove the TUI `i` keybinding and `todoist::run_import` call from `src/tui.rs`
- Remove Slack-related integration tests from `tests/integration.rs`
- Clean up `Cargo.toml` if any Slack-only dependencies exist

**Non-Goals:**
- Changing `task import todoist` CLI behavior in any way
- Removing Todoist auth from the CLI
- Modifying the TUI beyond removing Slack views and the Todoist import trigger
- Archiving or deleting Slack spec files (they can remain as historical reference)

## Decisions

### Decision: Delete slack.rs wholesale
Rather than feature-flagging or deprecating, delete `src/slack.rs` and all its call sites. The module is self-contained and no other capability depends on it.

**Alternatives considered**: Keep `slack.rs` but disable it — rejected because dead code increases maintenance burden with no benefit.

### Decision: Preserve `status_message` field in TUI App
The `status_message: Option<String>` field on the `App` struct was introduced alongside the Todoist TUI import. Even after removing the Todoist import trigger from the TUI, `status_message` may be used by other TUI interactions. Keep it unless a code audit shows it is exclusively used by the Todoist import path.

### Decision: CLI `task import todoist` is untouched
The user's intent is to make Todoist import CLI-only — the CLI command already exists and works. Zero changes needed there.

## Risks / Trade-offs

- [Risk] Removing Slack auth from `auth.rs` may leave orphaned token files on disk → Mitigation: Non-issue for users; no automatic cleanup needed.
- [Risk] Removing Slack TUI views may affect TUI view routing state machine → Mitigation: Audit `tui.rs` view enum and match arms carefully; remove all Slack-variant branches.

## Migration Plan

1. Delete `src/slack.rs`
2. Remove all `use crate::slack` / `mod slack` references
3. Remove Slack views and state from `src/tui.rs`
4. Remove `AuthCommand::Slack` from `src/cli.rs`
5. Remove Slack token functions from `src/auth.rs`
6. Remove TUI Todoist import keybinding (`i`) from `src/tui.rs`
7. Remove Slack test cases from `tests/integration.rs`
8. Run `cargo build` to confirm clean compilation
9. Run `cargo test` to confirm no regressions
