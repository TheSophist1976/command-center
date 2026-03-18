## Why

Claude Cowork users who also use this task manager have no way to read or modify their tasks from within a Cowork session. A Cowork skill teaches Claude the task manager's CLI so it can list, add, edit, and complete tasks without the user switching contexts. The skill should stay in sync with the binary — automatically installed and updated whenever the user deploys locally.

## What Changes

- Add a new Claude Cowork skill at `skills/task-manager/SKILL.md` in the repo — a YAML-frontmatter skill file describing how Claude should use the `task` CLI
- The skill covers: listing tasks (with filters), adding tasks, editing task fields (priority, tags, due date, title), completing/reopening tasks, and viewing task details by ID
- Update `deploy.sh` to automatically copy `skills/task-manager/` to `~/.claude/skills/task-manager/` as a new deploy step, so the skill is always in sync with the installed binary
- The skill is self-contained: uses only the `task` CLI installed by the same deploy script

## Capabilities

### New Capabilities
- `cowork-task-skill`: A Claude Cowork skill definition (`skills/task-manager/SKILL.md`) with YAML frontmatter (`name`, `description`) and plain-language instructions for using the `task` CLI to read and edit tasks

### Modified Capabilities
<!-- None - no existing spec requirements are changing -->

## Impact

- New file: `skills/task-manager/SKILL.md`
- Modified: `deploy.sh` — adds a skill installation step (copies `skills/task-manager/` → `~/.claude/skills/task-manager/`)
- No changes to Rust source code
- Depends on `cli-interface` spec for accurate command syntax
- Installed skill path: `~/.claude/skills/task-manager/SKILL.md`
