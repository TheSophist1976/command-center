## Context

The TUI loads `tasks.md` once at startup via `storage::load()` and holds the result as `app.task_file: TaskFile` in memory. External writers (AI agents, CLI commands) can modify `tasks.md` while the TUI is open. Currently there is no way to pick up those changes without restarting.

The event loop already ticks every 200 ms to service NLP and Claude session polling — there is no need for a new polling mechanism. `r` is taken (`r:desc` — edit description) and `R` is taken (`R:recur`), so `ctrl+r` is used for reload.

## Goals / Non-Goals

**Goals:**
- Allow the user to manually reload `tasks.md` from disk with `ctrl+r` in Normal mode
- Preserve the current selection as closely as possible after reload
- Block reload when editing is in progress and surface a clear warning

**Non-Goals:**
- Automatic / background reload (no file-system watcher, no polling)
- Merge or diff between in-memory state and disk state
- Conflict resolution when TUI has pending writes

## Decisions

### Use `ctrl+r` as the keybinding
`r` maps to description editing and `R` maps to recurrence editing. `ctrl+r` is a widely-recognised "refresh" shortcut and has no current binding in the TUI.

### Block reload in any non-Normal mode
Any mode other than `Normal` implies an in-progress edit (`Adding`, `EditingTitle`, `EditingDescription`, etc.). Reloading in those modes would silently discard draft input. Blocking and showing a status message (`"Cannot reload: finish editing first"`) is the safest behaviour — the user chose option (a) explicitly.

### Replace `app.task_file` in place
Call `storage::load(&app.file_path, false)` and overwrite `app.task_file`. Then call `app.clamp_selection()` to keep the cursor in bounds. No structural changes to `App` are needed.

### Show status message on success and failure
On success: `"Reloaded N tasks from disk"`. On load error: display the error string via `app.status_message`.

## Risks / Trade-offs

- **Race condition** — If an agent is mid-write when the user hits `ctrl+r`, the reload may read a partially-written file. Mitigation: `storage::save()` uses an atomic rename, so `storage::load()` will either read the old or the new complete file, never a partial one.
- **Selection drift** — After reload, the selected task index may point to a different task if tasks were added/removed above the cursor. Mitigation: `clamp_selection()` keeps the index in-bounds; acceptable UX for a manual action.

## Migration Plan

No migration needed. Pure additive change to `src/tui.rs`.
