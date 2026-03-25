## Context

The NLP console spans `src/nlp.rs`, `src/auth.rs` (Claude key storage), `src/tui.rs` (NlpChat/ConfirmingNlp modes), `src/main.rs`, and `src/cli.rs`. It is a cross-cutting removal touching auth, TUI rendering, and the NLP subsystem. The `claude-session-launcher` feature now provides a richer Claude experience, making the in-app NLP console redundant.

## Goals / Non-Goals

**Goals:**
- Delete `src/nlp.rs` entirely
- Remove Claude API key auth (`auth claude` subcommand, `write_claude_key`, `read_claude_key_source`, `delete_claude_key`)
- Remove `Mode::NlpChat` and `Mode::ConfirmingNlp` from `tui.rs` with all associated state and handlers
- Remove `i` NLP keybinding and all NLP-related footer hints
- Remove `mod nlp` from `main.rs` and all `nlp::` call sites
- Remove `AuthCommand::Claude` from `cli.rs`
- Remove claude-auth deploy step from `deploy.sh`

**Non-Goals:**
- Removing `reqwest` (still used by Slack)
- Touching Slack, Todoist, or task storage modules
- Removing rule-based recurrence parsing (only `parse_recurrence_nlp` is removed)

## Decisions

**Delete `nlp.rs` wholesale rather than stub it out**
There are no remaining callers after the TUI and CLI changes. A tombstone module or empty file would be misleading. Hard deletion is the right choice.

**Keep `ANTHROPIC_API_KEY` env var check removed from auth**
The env var check lives in `auth.rs` alongside `read_claude_key`. With the entire Claude key subsystem gone, there is no remaining path that reads this env var. The check is dead code — removing it is correct.

**`parse_recurrence_nlp` call sites replaced with rule-based parser**
Any call site that was using the NLP-powered recurrence parser (`nlp::parse_recurrence_nlp`) must be replaced with the existing rule-based parser (`Recurrence::from_str`). This is behaviorally equivalent for well-formed inputs and removes the API dependency.

## Risks / Trade-offs

- **Users relying on `task auth claude`** → The subcommand will no longer exist. Mitigation: document in release notes; the claude-session workflow does not require an Anthropic API key.
- **Stored `claude_api_key` files left on disk** → `task auth revoke` will no longer delete them. Mitigation: the files are inert without the reading code; document that users can delete manually if desired.
- **Test coverage of removed code** → Existing NLP and auth tests will be deleted. No replacement tests needed since the functionality is gone.

## Migration Plan

1. Delete `src/nlp.rs`
2. Remove `mod nlp;` from `src/main.rs`; remove all `nlp::` call sites (recurrence parse, `interpret`, `parse_recurrence_nlp`)
3. Remove `AuthCommand::Claude` variant from `src/cli.rs`
4. Remove Claude key functions from `src/auth.rs` (`write_claude_key`, `read_claude_key_source`, `delete_claude_key`, env-var fallback logic); remove related tests
5. Remove from `src/tui.rs`: `use crate::nlp::...`, `nlp_messages`, `nlp_pending`, `nlp_spinner_frame`, `pending_nlp_update`, `Mode::NlpChat`, `Mode::ConfirmingNlp`, all NLP handlers and rendering, `i` keybinding and NLP-related footer hints
6. Remove claude-auth step from `deploy.sh`
7. Compile and test
