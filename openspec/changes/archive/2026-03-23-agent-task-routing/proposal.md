## Why

Tasks have no concept of who should do them — AI agents reading the task list cannot distinguish their work from human work, nor know which codebase or working directory a task belongs to. Adding named agent profiles (name + directory) and an `agent` field on tasks gives AI agents a reliable, low-friction way to find and execute their assigned work using their current working directory as the routing key.

## What Changes

- Add `agent-<name>: <dir>` config entries to `config.md` — each entry defines a named agent profile with an associated working directory
- Add optional `agent` field to the `Task` struct and its parser/serializer — stores a profile name (e.g., `agent:command-center`) or the literal `human`
- TUI: new keybinding to assign/clear the agent on the selected task, with a picker populated from configured profiles plus `human` and `(none)`
- Update `AGENTS.md` with the CWD-based task routing rule: find the agent profile whose directory is a prefix of CWD, filter tasks to that agent name
- Update the Cowork skill (`skills/task-manager/SKILL.md`) with the same routing instructions

## Capabilities

### New Capabilities
- `agent-profiles`: Named agent profiles stored in config (`agent-<name>: <dir>`), the `agent` field on tasks, and CWD-based task lookup — the full routing system connecting AI agents to their work

### Modified Capabilities
- `app-config`: New `agent-<name>` config key family for registering agent profiles with working directories
- `task-storage`: New optional `agent` metadata field in the task metadata comment

## Impact

- `src/config.rs` — read/write/list `agent-*` config entries
- `src/task.rs` — add `agent: Option<String>` field to `Task`
- `src/parser.rs` — parse and serialize `agent` metadata key
- `src/tui.rs` — agent picker mode and keybinding to assign agent to a task
- `AGENTS.md` — CWD-based task routing rule
- `skills/task-manager/SKILL.md` — same routing instructions for Cowork
