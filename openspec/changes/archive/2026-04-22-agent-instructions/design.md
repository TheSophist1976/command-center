## Context

The codebase already has `config::find_agent_for_cwd(cwd)` which returns the agent name matching the CWD. `App::new` initialises `filter: Filter::default()` (no filter). `Filter::parse("agent:name")` produces a working filter. Instruction notes use the existing `note::write_note` / `note::read_note` infrastructure which already accepts any `dir: &Path`.

## Goals / Non-Goals

**Goals:**
- `Notes/Instructions/` subfolder stores one markdown file per agent (`<agent-name>.md`)
- Config key `agent-<name>-instructions: <slug>` links an agent to their instruction note (optional — instructions can also be found by convention without the key)
- New CLI subcommand: `task agent instructions <name> show|edit --body "..."` for AI agents to create/edit their instructions
- TUI auto-filter: on `App::new`, if CWD matches an agent profile, set initial filter to `agent:<name>`
- AGENTS.md updated: document how to read your instruction note and the auto-filter behaviour

**Non-Goals:**
- TUI UI for editing agent instructions (CLI only)
- Multiple instruction notes per agent
- Instruction notes appearing in the TUI Notes view

## Decisions

**`Notes/Instructions/` subfolder, slug = agent name**
Convention-based: `Notes/Instructions/<agent-name>.md`. No lookup needed — the file path is deterministic from the agent name. Config key (`agent-<name>-instructions`) is optional and exists for cases where the slug differs, but the default convention avoids extra config.

**New `task agent` top-level subcommand**
Rather than overloading `task note`, a dedicated `task agent instructions` subcommand gives AI agents a clear, discoverable entry point:
```
task agent instructions <name> show
task agent instructions <name> edit --body "<markdown>"
task agent instructions <name> edit --title "<title>"
```
Internally it delegates to `note::write_note` / `note::read_note` targeting `<task-dir>/Notes/Instructions/`.

**TUI auto-filter in `App::new`**
After constructing the `App`, call `config::find_agent_for_cwd(&std::env::current_dir())`. If found, set `app.filter = Filter::parse(&format!("agent:{}", name))`. This is the last step in `App::new` so it doesn't interfere with other initialisation. The user can still press `Esc` to clear.

**Task dir resolution for CLI**
The `task agent` subcommand resolves the task dir from the `--file` flag (same as `task note`): `path.parent().unwrap_or(".")`. The `Notes/Instructions/` dir is `task_dir.join("Notes").join("Instructions")`.

## Risks / Trade-offs

- [Risk] Agent name contains characters invalid in filenames → slugify agent name for the filename (replace spaces with `-`, lowercase). Low risk since current agent names are simple identifiers.
- [Risk] Auto-filter surprises a human user launching the TUI from a project directory → filter is visible in the header and clearable with `Esc`. Acceptable.
- [Risk] `Notes/Instructions/` dir is excluded from the Notes view (it's a subdirectory) → `discover_notes` already scans only the immediate `Notes/` dir, not subdirectories. Instructions are invisible to the Notes view by design.

## Migration Plan

No migration needed. The `Notes/Instructions/` directory is created on first write via `fs::create_dir_all`. Existing configs are unaffected.
