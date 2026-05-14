## Context

Agent instructions currently live at `Notes/Instructions/<agent-name>.md` and are read by AI agents before working tasks. The `task agent instructions` CLI manages these files. There is no memory mechanism — agents start fresh on every session with no accumulated context.

The new structure consolidates per-agent files into a folder: `Notes/Agents/<agent-name>/`. This mirrors a common AI memory pattern and is extensible (future files like `history.md` or `templates.md` can live alongside).

## Goals / Non-Goals

**Goals:**
- `Notes/Agents/<name>/instructions.md` replaces `Notes/Instructions/<name>.md`
- `Notes/Agents/<name>/memory.md` is a new file agents read at session start and update when they learn something worth remembering
- CLI: `task agent instructions` updated to new path; `task agent memory show/edit` added
- `work-agent-tasks` skill loads both instructions and memory for each active agent
- Each agent's instruction file gets a Memory section explaining usage
- Existing instruction files are migrated to the new location

**Non-Goals:**
- Automatic memory updates without agent judgment
- Structured/queryable memory (free-form markdown is sufficient)
- Memory size limits or pruning automation

## Decisions

### Folder per agent, not flat files
A single `Notes/Agents/<name>/` directory scales to multiple file types without polluting the flat `Notes/` namespace. The TUI already ignores subdirectories of `Notes/`, so neither instructions nor memory will appear in the Notes view.

### Memory format: free-form markdown with suggested sections
No rigid schema — agents write what they observe in plain markdown. Suggested sections (Preferences, Patterns, Standing Context) give structure without enforcement. Agents can add or remove sections as needed.

### Memory update criteria (in skill, not in code)
When to update memory is a judgment call the agent makes. The skill provides explicit guidance:
- **Update memory when**: a pattern repeats (user preference observed 2+ times), a standing fact is established (recurring contact, project detail), or the agent corrects a past mistake
- **Don't update memory for**: one-off task details, information that belongs in the task note, anything likely to change soon

### CLI: `task agent memory` mirrors `task agent instructions`
Same `show` / `edit --body` pattern. Stored at `Notes/Agents/<name>/memory.md`. Agents can read and write via CLI or directly via file access.

### Migration: move files, don't copy
`Notes/Instructions/<name>.md` → `Notes/Agents/<name>/instructions.md`. The old path is no longer read. If the new path exists and the old path also exists, new path takes precedence.

## Risks / Trade-offs

- [Risk: Memory grows stale or contradictory] → Agent instructions include guidance to review and prune memory periodically; free-form format lets agents rewrite sections as context changes
- [Risk: Breaking existing workflows] → Migration is a file move; the CLI will fall back gracefully if the new path is missing and the old path exists (one-release backward compat)

## Open Questions

None — design is fully specified.
