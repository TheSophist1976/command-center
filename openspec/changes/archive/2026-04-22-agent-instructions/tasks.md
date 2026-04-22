## 1. TUI — Agent auto-filter on launch

- [x] 1.1 In `App::new` (just before `Ok(app)`), call `config::find_agent_for_cwd(&std::env::current_dir().unwrap_or_default())` and if `Some(name)` is returned, set `app.filter = Filter::parse(&format!("agent:{}", name))`
- [x] 1.2 Call `app.clamp_selection()` after setting the filter to ensure selection is valid

## 2. CLI — Add `task agent` subcommand

- [x] 2.1 Add `AgentCommand` enum to `src/cli.rs` with subcommand `Instructions { name: String, #[command(subcommand)] action: AgentInstructionsCommand }`
- [x] 2.2 Add `AgentInstructionsCommand` enum to `src/cli.rs` with variants `Show` and `Edit { title: Option<String>, body: Option<String> }`
- [x] 2.3 Add `Agent { #[command(subcommand)] subcommand: AgentCommand }` variant to the top-level `Command` enum in `src/cli.rs`

## 3. CLI — Implement `task agent instructions` handler

- [x] 3.1 In `src/bin/task.rs`, add handler for `Command::Agent`: resolve `task_dir` from `path.parent()`, then `instructions_dir = task_dir.join("Notes").join("Instructions")`
- [x] 3.2 Implement `AgentInstructionsCommand::Show`: slugify the agent name, read `instructions_dir/<slug>.md` via `task::note::read_note`, print title and body. Print "No instructions found." if the file does not exist.
- [x] 3.3 Implement `AgentInstructionsCommand::Edit`: slugify the agent name. Read existing note if present (to preserve title/body not being replaced). Write updated note via `task::note::write_note` to `instructions_dir`. `fs::create_dir_all` the `instructions_dir` before writing.
- [x] 3.4 Add slugify helper for agent names: reuse `task::note::slugify`

## 4. Config — Agent instructions key lookup (optional override)

- [x] 4.1 Add `pub fn agent_instructions_slug(agent_name: &str) -> Option<String>` to `src/config.rs`: reads `agent-<name>-instructions` from config. Returns `None` if absent (caller falls back to convention).
- [x] 4.2 Update the CLI `show` and `edit` handlers to use this function: if `Some(slug)` use that, else use slugified agent name as the filename stem.

## 5. AGENTS.md — Document instructions and auto-filter

- [x] 5.1 Add a "Reading Your Instructions" section to `AGENTS.md` explaining that `Notes/Instructions/<agent-name>.md` contains operating instructions. Agents should read this file at the start of a session.
- [x] 5.2 Add a note explaining that `task-tui` auto-filters to the agent's tasks on launch when run from the project directory.
- [x] 5.3 Document `task agent instructions <name> show` and `task agent instructions <name> edit --body "..."` CLI commands.

## 6. Tests

- [x] 6.1 Unit test: `App::new` with a config that has an agent profile matching a temp CWD sets `app.filter` to `agent:<name>` (tested via `find_agent_for_cwd_from` unit tests in config.rs)
- [x] 6.2 Unit test: `App::new` with no matching agent profile leaves `app.filter` as default (empty)
- [x] 6.3 Integration test: `task agent instructions mybot edit --body "foo"` creates `Notes/Instructions/mybot.md`
- [x] 6.4 Integration test: `task agent instructions mybot show` prints the body after editing
