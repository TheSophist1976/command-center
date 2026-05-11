## Context

The task data model (`src/task.rs`) uses an enum-per-field pattern: `Priority` is already an enum (`Critical`, `High`, `Medium`, `Low`) with `Option<Priority>` on the `Task` struct. The parser (`src/parser.rs`) handles serialization and deserialization of all metadata fields. The TUI (`src/tui.rs`) already has a `GroupBy` enum with `None`, `Project`, `Agent`, `Priority`, `DueDate` and a cycle through them via the `G` key. Grouping values are persisted per-view in config as `group-by.<view>: <value>`.

The `agent` field added an inline picker (a small overlay list) for editing from the TUI. Effort editing will follow the same pattern.

## Goals / Non-Goals

**Goals:**
- Add `effort: Option<Effort>` to the `Task` struct with `High`, `Medium`, `Low` variants
- Parse and serialize `effort` in metadata comments
- Add `GroupBy::Effort` to the TUI cycle
- Display effort in the task list (as a column) and detail panel
- Support inline editing via a picker overlay (matching the agent picker pattern)
- Persist effort grouping to config

**Non-Goals:**
- Filtering by effort (a separate future feature)
- Auto-suggesting effort based on title or description
- Migration tooling — effort is optional and defaults to `None`; existing files parse without change

## Decisions

### Effort as a dedicated enum (not reusing Priority)
Effort is conceptually orthogonal to priority. A task can be `priority:critical, effort:low` (urgent but quick) or `priority:low, effort:high` (low stakes but mentally taxing). Reusing Priority would lose this distinction. A dedicated `Effort` enum is the correct model.

### Three values: High / Medium / Low
Matches the user's described semantics exactly. More granularity (e.g. five levels) adds cognitive load without meaningfully expanding the model. Three levels maps to the common "energy zones" mental model.

### Optional field (no default)
Forcing a default value would pollute existing tasks with meaningless data. `None` means "unspecified" — the TUI groups unspecified tasks under `(none)` when effort grouping is active, consistent with how `GroupBy::Project` and `GroupBy::Agent` handle absent values today.

### Serialization: `effort:high` / `effort:medium` / `effort:low`
Consistent with all other metadata keys. Lowercase matches the existing convention (`priority:high`, `agent:command-center`).

### Effort picker for TUI editing (matching agent picker pattern)
The agent picker is already a proven pattern for this type of enum field. Reusing the same overlay approach keeps the UX consistent and minimizes new code paths.

### GroupBy cycle order: insert after Priority
The current cycle is `None → Project → Agent → Priority → DueDate → None`. Effort is most similar to Priority (both classify by task attribute), so inserting it after Priority gives: `None → Project → Agent → Priority → Effort → DueDate → None`.

## Risks / Trade-offs

- [Risk: Column width in narrow terminals] Adding an effort column may truncate task titles on small screens. → Mitigation: display effort as a short abbreviation (`H`/`M`/`L`) in the list column, full label in the detail panel.
- [Risk: Effort picker key conflict] The `A` key is used for the agent picker. Need to pick a non-conflicting key for effort editing. → Use `E` key (currently unbound in task detail mode).

## Migration Plan

No migration required. The `effort` field is optional. Existing `tasks.md` files parse correctly with all tasks defaulting to `effort: None`. No format version bump needed.

## Open Questions

None — the design is fully specified.
