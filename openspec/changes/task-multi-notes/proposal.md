## Why

Tasks currently support a single linked note. This means research, instructions, and reference material for a task must be crammed into one note or left unlinked. Supporting multiple notes lets a task carry a richer context: a research note, a how-to, a reference document, and a draft all attached to the same task. Instruction notes — notes whose name signals they contain task-specific guidance — let humans pre-load a task with steps that agents will automatically read before starting work.

## What Changes

- **Data model**: `note: Option<String>` on `Task` → `notes: Vec<String>` (list of slugs)
- **Serialization**: `note:<slug>` in metadata → `notes:<slug1>,<slug2>,...`; existing `note:` keys are parsed as a single-element list (backward compat)
- **TUI**: `n` key opens a multi-note picker — add, remove, or navigate to any attached note; task list shows note count badge when notes > 0
- **Agents**: `work-agent-tasks` skill updated — before working a task, agents read all attached notes; notes whose slug or title contains `instructions`, `how-to`, or `steps` are flagged as instruction notes and read with elevated priority
- **CLI**: `task note link` / `task note unlink` updated to append/remove from the notes list rather than replace
- **AGENTS.md**: document the `notes` field and instruction note naming convention

## Capabilities

### Modified Capabilities
- `task-storage`: `notes` field replaces `note`; comma-separated slugs; backward-compat parse of legacy `note:` key
- `note-task-link`: multi-note support — picker, count badge, navigation, add/remove; instruction note detection by slug/title naming

## Impact

- `src/task.rs` — `note: Option<String>` → `notes: Vec<String>`
- `src/parser.rs` — serialize/parse `notes` key; parse legacy `note:` as single-element
- `src/tui.rs` — multi-note picker, note count badge, note navigation updated
- `src/bin/task.rs` — `task note link/unlink` append/remove semantics
- `~/.claude/skills/work-agent-tasks/SKILL.md` — read all task notes; flag instruction notes
- Each agent's `instructions.md` — note instruction note naming convention
- `AGENTS.md` — document `notes` field and instruction note convention
