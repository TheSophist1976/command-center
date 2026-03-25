## 1. Delete src/nlp.rs

- [x] 1.1 Delete `src/nlp.rs` entirely

## 2. Remove nlp from main.rs and cli.rs

- [x] 2.1 Remove `mod nlp;` from `src/bin/task.rs` (entry point)
- [x] 2.2 Remove `nlp::parse_recurrence_nlp` call sites — replaced with rule-based `Recurrence::from_str`
- [x] 2.3 Remove `AuthCommand::Claude` variant from `src/cli.rs`
- [x] 2.4 Remove the `AuthCommand::Claude` match arm from `src/bin/task.rs`

## 3. Remove Claude API key functions from src/auth.rs

- [x] 3.1 Remove `write_claude_key`, `read_claude_key_source`, `delete_claude_key` functions
- [x] 3.2 Remove `ANTHROPIC_API_KEY` env var check and `read_claude_key` function
- [x] 3.3 Remove Claude key from `auth_status` output
- [x] 3.4 Remove Claude key deletion from `auth_revoke`
- [x] 3.5 Remove auth tests for Claude key functions

## 4. Remove NLP state and modes from src/tui.rs

- [x] 4.1 Remove `use crate::nlp::...` import
- [x] 4.2 Remove `Mode::NlpChat` and `Mode::ConfirmingNlp` from the `Mode` enum
- [x] 4.3 Remove `nlp_messages`, `nlp_pending`, `nlp_spinner_frame`, `pending_nlp_update` fields from `App`
- [x] 4.4 Remove their initializers from `App::new` and any test `App` struct literals
- [x] 4.5 Remove `handle_nlp_chat` and `handle_nlp_confirm` functions
- [x] 4.6 Remove `process_nlp_result` function
- [x] 4.7 Remove chat panel rendering (`draw_chat_panel` or equivalent)
- [x] 4.8 Remove `Mode::NlpChat` and `Mode::ConfirmingNlp` arms from the main event dispatch
- [x] 4.9 Remove `Mode::NlpChat` and `Mode::ConfirmingNlp` arms from the draw function
- [x] 4.10 Remove `:` keybinding (enter NlpChat) from `handle_normal`
- [x] 4.11 Remove NLP-related footer hints from `draw_footer`
- [x] 4.12 Remove the four-region NlpChat layout from `draw`

## 5. Remove claude-auth from deploy.sh

- [x] 5.1 Remove the Claude API key setup step from `deploy.sh`

## 6. Verify and clean up

- [x] 6.1 Ensure the project compiles with no errors (`cargo build`)
- [x] 6.2 Run `cargo test` and confirm no regressions beyond removed NLP/auth tests
- [x] 6.3 Remove any dead imports or unused constants
