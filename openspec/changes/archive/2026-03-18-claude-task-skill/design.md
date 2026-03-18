## Context

The task manager binary exposes a full CLI for task CRUD (`add`, `list`, `show`, `edit`, `done`, `undo`, `rm`) as well as `auth` and `config` subcommands. These are headless commands suitable for scripting — no TUI required. Claude Cowork skills are Markdown files with YAML frontmatter that Claude auto-loads from `~/.claude/skills/<name>/SKILL.md`. The deploy script (`deploy.sh`) already handles build, install, PATH configuration, and auth setup as a single local deployment flow.

## Goals / Non-Goals

**Goals:**
- Define a `SKILL.md` that gives Claude accurate knowledge of every `task` command it may need to read or modify tasks on the user's behalf
- Auto-install the skill to `~/.claude/skills/task-manager/` on every local deploy so it stays in sync with the binary
- Keep the skill content derived directly from the CLI spec so it doesn't drift

**Non-Goals:**
- Wrapping the CLI in a new API or MCP server
- Supporting task file paths other than the configured default
- Teaching Claude to launch the TUI (`task tui`)
- Modifying the Rust source code in any way

## Decisions

### 1. Skill lives at `skills/task-manager/SKILL.md` in the repo root

Keeping the skill file in the repo (rather than only in `~/.claude/skills/`) means it is versioned alongside the code. When CLI commands change, the skill is updated in the same commit. The `skills/` directory at the project root is a natural home for distributable Cowork skills.

**Alternative considered**: Store only in `~/.claude/skills/` and regenerate on deploy. Rejected — no source of truth, can't review skill changes in PRs.

### 2. Deploy step copies the skill directory verbatim

`deploy.sh` will use `cp -r skills/task-manager ~/.claude/skills/` (creating `~/.claude/skills/task-manager/`) to install the skill. This is idempotent and overwrites any prior version. The step runs after the binary is installed (step 4) so the skill always reflects the deployed binary's capabilities.

**Alternative considered**: Symlink `~/.claude/skills/task-manager` → repo. Rejected — breaks if the repo moves or is deleted; users may not keep the repo around after install.

### 3. Skill instructs Claude to use the headless CLI, not the TUI

All task operations are available via non-interactive CLI subcommands. Claude should never invoke `task tui` — that requires a terminal. The skill explicitly lists commands and flags so Claude doesn't guess syntax.

### 4. Skill content covers the full CRUD surface

The skill teaches Claude:
- `task list` with all filter flags (`--status`, `--priority`, `--tag`)
- `task show <id>` for detail
- `task add` with all creation flags
- `task edit <id>` with all edit flags
- `task done <id>` / `task undo <id>` for status changes
- `task rm <id>` for deletion
- Flag formats: priorities (`critical`, `high`, `medium`, `low`), tags (comma-separated, lowercase, alphanumeric + hyphens), due dates (`YYYY-MM-DD`)

## Risks / Trade-offs

- **Skill drift**: If a new CLI flag is added without updating `SKILL.md`, Claude will not know about it until the next deploy. Mitigation: the proposal explicitly requires the skill to be updated when new features are added; this is a documented convention.
- **`~/.claude/skills/` path assumption**: Assumes Cowork loads skills from this path. Mitigation: path is confirmed by public documentation; if it changes, only the deploy step needs updating.
- **Overwrite on deploy**: Each deploy replaces the installed skill. If a user has manually edited `~/.claude/skills/task-manager/SKILL.md`, changes will be lost. Mitigation: acceptable trade-off — the source of truth is the repo file.
