## 1. Task data model

- [x] 1.1 Add `agent: Option<String>` field to the `Task` struct in `src/task.rs`
- [x] 1.2 Update `src/parser.rs` — parse `agent` key from metadata comment into `Task.agent`
- [x] 1.3 Update `src/parser.rs` — serialize `agent` field into metadata comment (omit when `None`)
- [x] 1.4 Add parser round-trip tests for `agent` field (present, `human`, absent)

## 2. Config: agent profiles

- [x] 2.1 Add `list_agent_profiles() -> Vec<(String, String)>` to `src/config.rs` — scans config for `agent-*` keys and returns `(name, dir)` pairs
- [x] 2.2 Add `find_agent_for_cwd(cwd: &Path) -> Option<String>` to `src/config.rs` — expands tildes, prefix-matches profiles against CWD, returns longest match profile name
- [x] 2.3 Add unit tests for `list_agent_profiles` (empty, one, multiple profiles)
- [x] 2.4 Add unit tests for `find_agent_for_cwd` (exact match, subdirectory match, tilde expansion, no match, longest match wins)

## 3. TUI: agent picker

- [x] 3.1 Add `EditingAgent` variant to the `Mode` enum in `src/tui.rs`
- [x] 3.2 Add `agent_picker_items: Vec<String>` and `agent_picker_selected: usize` fields to the `App` struct
- [x] 3.3 Add `A` keybinding in `handle_normal` — load profiles from config, populate picker items (`[profiles..., "human", "(clear)"]`), enter `EditingAgent` mode
- [x] 3.4 Add `handle_agent_picker` function — `j`/`k` navigate, `Enter` applies selection to selected task and saves, `Esc` cancels
- [x] 3.5 Add `draw_agent_picker` function — renders a centered list of picker options with the current selection highlighted
- [x] 3.6 Wire `EditingAgent` into `handle_key` and `draw` (same layout pattern as `NotePicker`)
- [x] 3.7 Add footer hint for `EditingAgent` mode: `" j/k:nav  Enter:select  Esc:cancel "`
- [x] 3.8 Show current agent on selected task in detail panel (if detail panel is open)

## 4. Documentation

- [x] 4.1 Update `AGENTS.md` — add "Finding your tasks" section explaining CWD-based lookup: read `agent-*` config entries, find prefix match against CWD, filter `tasks.md` to that agent name
- [x] 4.2 Update `skills/task-manager/SKILL.md` — add equivalent agent routing instructions

## 5. Verification

- [x] 5.1 Run `cargo build --features tui` — confirm zero errors
- [x] 5.2 Run `cargo test --features tui` — confirm all tests pass
