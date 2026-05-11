## Why

Tasks have no way to signal the cognitive load required to complete them. Priority tells you what matters most, but not whether you have the energy to act on it right now. An `effort` field lets users match tasks to their current mental state — picking high-focus work when sharp and low-effort tasks when drained.

## What Changes

- New optional `effort` metadata field on tasks with three values: `high`, `medium`, `low`
  - `high` — requires dedicated, uninterrupted time and full concentration
  - `medium` — requires dedicated time but low cognitive energy
  - `low` — can be done quickly while multitasking or context-switching
- Effort field serialized/parsed in task metadata comments (`effort:high`)
- `GroupBy::Effort` added to the TUI group-by cycle (`G` key)
- Effort displayed in the task list and detail panel
- Effort editing available in the TUI (inline picker)
- Per-view effort grouping persisted to config as `group-by.<view>: effort`

## Capabilities

### New Capabilities
- `task-effort-field`: Definition of the `effort` field — valid values, semantics, serialization, and parsing

### Modified Capabilities
- `task-storage`: Effort field added to task metadata (new optional `effort` key)
- `tui-views`: `GroupBy::Effort` added to grouping cycle; effort shown in list and detail
- `app-config`: `effort` added as a valid value for `group-by.<view>` config keys

## Impact

- `src/task.rs` — add `Effort` enum and `effort: Option<Effort>` field to `Task`
- `src/parser.rs` — parse and serialize `effort` key in metadata comments
- `src/tui.rs` — `GroupBy::Effort`, effort column display, effort picker for editing
- `AGENTS.md` — document the new `effort` field and its valid values
