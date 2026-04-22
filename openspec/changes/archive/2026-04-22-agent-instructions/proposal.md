## Why

Agents assigned tasks need context about how to approach their work. Currently there is no standard way for a human to communicate expectations, style, or process guidance to a specific AI agent. Attaching a markdown instruction note to each agent gives agents a consistent place to read their own operating instructions before starting work.

Additionally, when an AI agent launches `task-tui` from their project directory, they currently see all tasks — not just the ones assigned to them. Auto-filtering to the active agent's tasks on launch reduces noise and makes the TUI immediately useful for agent-driven workflows.

## What Changes

- **`Notes/Instructions/` subfolder**: instruction notes for agents are stored as markdown files at `<task-dir>/Notes/Instructions/<agent-name>.md`, separate from regular task notes in `Notes/`
- **Config link**: each agent can have an optional instruction note slug stored in config as `agent-<name>-instructions: <slug>`. The TUI and CLI use this to locate the instruction note.
- **CLI management**: AI agents create and edit their instruction notes using the existing `task note` CLI, targeting the `Notes/Instructions/` subfolder. A new `task agent instructions` subcommand provides a direct shortcut.
- **TUI auto-filter on launch**: when `task-tui` launches, it calls `find_agent_for_cwd(cwd)`. If a matching agent profile is found, the initial filter is set to `agent:<name>` so the agent sees only their own tasks immediately.
- **AGENTS.md updated**: documents the instruction note convention, how to read your own instructions, and the auto-filter behavior.

## Capabilities

### New Capabilities
- `agent-instructions`: Storing, linking, and reading instruction notes for agents via `Notes/Instructions/` subfolder and config key
- `tui-agent-autofilter`: TUI auto-applies `agent:<name>` filter on launch when CWD matches an agent profile

### Modified Capabilities
- `app-config`: New `agent-<name>-instructions: <slug>` config key pattern for linking instruction notes to agents

## Impact

- `src/tui.rs`: On `App::new`, call `find_agent_for_cwd(&cwd)` and if found set `app.filter = Filter::parse(&format!("agent:{}", name))`
- `src/bin/task.rs`: New `agent` subcommand with `instructions` sub-subcommand (`show`, `edit`, `set`)
- `src/cli.rs`: New `AgentCommand` and `AgentInstructionsCommand` enums
- `src/note.rs`: No changes needed — `write_note`/`read_note` already accept any `dir: &Path`
- `AGENTS.md`: Updated to document instruction note convention
- `openspec/specs/app-config/spec.md`: Delta for new config key
