## Why

The NLP console (Claude API-powered chat panel, natural-language task queries, and URL-fetch tool) adds significant complexity and a direct Anthropic API dependency to the application. With the `claude-session-launcher` feature providing a richer, full Claude Code session experience, the in-app NLP console is redundant. Removing it simplifies the codebase, eliminates the API key management burden, and reduces binary size.

## What Changes

- Remove the NLP chat panel from the TUI (`NlpChat` and `ConfirmingNlp` modes, `i` keybinding, chat panel rendering)
- Remove `src/nlp.rs` entirely (Claude API calls, system prompts, tool definitions, response parsing, recurrence NLP, URL fetch)
- Remove Claude API key auth: `auth claude` subcommand, `write_claude_key`, `read_claude_key_source`, `delete_claude_key`, `claude_api_key` file
- Remove `mod nlp` from `main.rs` and all `nlp::` usages in `tui.rs`
- Remove NLP-based recurrence parsing (`parse_recurrence_nlp`) and replace any call sites with the existing rule-based recurrence parser
- Remove the `claude-auth` deploy step from `deploy.sh`
- Remove `ANTHROPIC_API_KEY` references throughout

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `tui`: Remove `NlpChat` and `ConfirmingNlp` modes, `i` keybinding, and NLP-related footer hints.
- `tui-nlp`: **REMOVED** — entire capability deleted.
- `nlp-conversation`: **REMOVED** — entire capability deleted.
- `nlp-query-tool`: **REMOVED** — entire capability deleted.
- `nlp-url-fetch`: **REMOVED** — entire capability deleted.
- `note-nlp`: **REMOVED** — entire capability deleted.
- `claude-auth`: **REMOVED** — Claude API key storage and CLI subcommand deleted.
- `cli-interface`: Remove `auth claude` subcommand from the `auth` command group.

## Impact

- `src/nlp.rs`: deleted entirely
- `src/tui.rs`: remove `use crate::nlp::...`, `nlp_messages`, `nlp_pending`, `nlp_spinner_frame`, `pending_nlp_update`, `Mode::NlpChat`, `Mode::ConfirmingNlp`, `handle_nlp_chat`, `handle_nlp_confirm`, `process_nlp_result`, chat panel rendering, and `i` keybinding
- `src/auth.rs`: remove Claude API key functions and tests
- `src/main.rs`: remove `mod nlp`; remove `AuthCommand::Claude` match arm; remove `nlp::parse_recurrence_nlp` call sites
- `src/cli.rs`: remove `AuthCommand::Claude` variant
- `deploy.sh`: remove Claude API key setup step
- `Cargo.toml`: `reqwest` stays (Slack still uses it); no dependency changes needed
- No changes to `note.rs`, `storage.rs`, `task.rs`, or any Slack/Todoist modules
