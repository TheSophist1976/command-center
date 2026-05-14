## Why

Agents currently only have static instructions. They have no way to accumulate learned context across tasks — patterns about how Mark prefers work done, standing project context, recurring contacts, or anything else an agent should remember. Adding a memory file alongside instructions, in a per-agent folder, gives agents a persistent second brain that improves with every task.

## What Changes

- **New folder structure per agent**: `Notes/Agents/<agent-name>/` replaces `Notes/Instructions/<agent-name>.md`
  - `Notes/Agents/<agent-name>/instructions.md` — static operating instructions (was `Notes/Instructions/<agent-name>.md`)
  - `Notes/Agents/<agent-name>/memory.md` — learned context the agent writes and reads across tasks
- **Migration**: existing `Notes/Instructions/<agent-name>.md` files are moved to the new location
- **CLI update**: `task agent instructions` paths updated to new folder; new `task agent memory show/edit` subcommand for reading/writing agent memory
- **Skills update**: `work-agent-tasks` skill updated to load both instructions and memory per agent, with explicit guidance on when and what to write to memory
- **Agent instruction files updated**: each agent's instructions include a Memory section explaining how to use and update their memory file

## Capabilities

### New Capabilities
- `agent-memory`: Memory file format, read/write behavior, content guidelines, and update criteria

### Modified Capabilities
- `agent-instructions`: File path changes from `Notes/Instructions/<name>.md` to `Notes/Agents/<name>/instructions.md`

## Impact

- `src/bin/task.rs` — update `instructions_dir` path; add `memory` subcommand
- `src/cli.rs` — add `AgentMemoryCommand` (show / edit)
- `Notes/Instructions/*.md` → `Notes/Agents/*/instructions.md` (file migration)
- `~/.claude/skills/work-agent-tasks/SKILL.md` — load memory alongside instructions
- `Notes/Instructions/*.md` agent instruction files — add Memory section to each
- `AGENTS.md` — document new folder structure and memory field
