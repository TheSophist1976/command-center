## Context

The current `note: Option<String>` field holds one slug. The `note-task-link` spec documents the `n`-key picker, `g`-key navigation, and note indicator in the task list. All of these need to be extended for a list of slugs.

The `task note link <slug> <id>` and `task note unlink <id>` CLI commands currently set/clear a single note — they need append/remove semantics.

## Goals / Non-Goals

**Goals:**
- `notes: Vec<String>` replaces `note: Option<String>` on `Task`
- Serialized as `notes:slug1,slug2` (comma-separated); empty list → key omitted
- Legacy `note:slug` parsed as `notes: vec!["slug"]` for backward compat — no migration needed
- TUI `n` key: multi-note picker showing all attached notes + "Add note" + "Remove note" actions
- Task list: note count badge (e.g. `[2]`) replaces the current note indicator when multiple notes attached
- `g` key: if one note, open it directly; if multiple, show picker first
- `task note link <slug> <id>`: appends slug if not already present
- `task note unlink <task-id>`: removes a specific slug (prompted) or all if only one
- Instruction note detection: slug or title containing `instructions`, `how-to`, or `steps` → agents read these first
- `work-agent-tasks` skill: reads all attached notes before working; surfaces instruction notes separately

**Non-Goals:**
- Note ordering (list order = attachment order, no manual sort)
- Note types/categories in the data model (naming convention is sufficient)
- Migrating existing `note:` fields in tasks.md (parser handles backward compat transparently)

## Decisions

### `Vec<String>` not `Option<Vec<String>>`
An empty vec is cleaner than `None` for a list. Serialization omits the key when empty, so the on-disk format is unchanged for tasks with no notes.

### Comma-separated in metadata, consistent with `tags`
`notes:slug1,slug2` mirrors how `tags:frontend,api` works. Existing parser infrastructure handles this pattern.

### Backward compat: parse `note:` as single-element `notes`
Any existing `note:slug` in tasks.md is read as `notes: vec!["slug"]`. On next save it serializes as `notes:slug` (single-element list, no comma). This is invisible to users.

### Instruction note detection by naming convention, not a type field
Slugs or titles containing `instructions`, `how-to`, or `steps` are treated as instruction notes by agents. This is enforced in the skill and agent instructions, not in the data model — keeps the data model simple.

### `g` key: direct open when one note, picker when multiple
Preserves existing muscle memory for the common case (one note) while naturally extending to multiple.

## Risks / Trade-offs

- [Risk: Long notes list clutters metadata] → Slugs are short; a task with 3–4 notes is still readable. No truncation needed.
- [Risk: `task note unlink` ambiguity with multiple notes] → CLI prompts which slug to remove; if only one note, removes it without prompt (preserving existing behavior).

## Open Questions

None.
