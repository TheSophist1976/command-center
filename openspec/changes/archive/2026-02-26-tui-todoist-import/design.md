## Context

The CLI already has a complete Todoist import pipeline: `todoist::run_import()` handles fetching tasks, resolving projects, mapping fields, appending to the TaskFile, and labeling tasks as exported. The TUI already supports multiple modes (Adding, Filtering, EditingPriority, etc.) with a consistent pattern: a keybinding enters a mode, the footer shows status/prompts, and the mode resolves back to Normal.

Currently users must quit the TUI, run `task import todoist`, and relaunch the TUI to see imported tasks. The import can also be run with `--test` mode from the CLI.

## Goals / Non-Goals

**Goals:**
- Let users trigger a Todoist import from the TUI with a single keypress (`i`)
- Show import progress and results in the TUI footer
- Reload the task list after a successful import so new tasks appear immediately
- Handle errors gracefully (no token, API failures) with in-TUI feedback

**Non-Goals:**
- Async/non-blocking import — the existing `todoist::run_import` uses `reqwest::blocking`, and converting to async would require a larger refactor. A blocking call is acceptable since imports are fast (typically <2 seconds).
- Test mode from TUI — the `--test` flag is a CLI-specific debugging tool, not needed in the TUI
- Auth flow from TUI — users still run `task auth todoist` from the CLI to set up their token

## Decisions

### 1. Reuse `todoist::run_import` directly

**Choice**: Call the existing `todoist::run_import(token, &mut app.task_file, false)` from the TUI key handler.

**Why**: The function already takes a `&mut TaskFile` and returns `(imported, skipped)`. No new import logic needed. The TUI already holds `app.task_file` mutably.

**Alternative considered**: Extract a higher-level import function that also handles file I/O. Rejected because the TUI already manages its own save cycle, and `run_import` mutates the TaskFile in place which is exactly what we need.

### 2. Use a status message field instead of a new mode

**Choice**: Add a `status_message: Option<String>` field to `App` that displays in the footer during Normal mode. The import operation runs synchronously when `i` is pressed — the footer shows the result message on the next render, and the message clears on the next keypress.

**Why**: The import is a fire-and-forget action, not an interactive mode. Adding a full `Importing` mode (as initially proposed) would be over-engineering since there's no user interaction during the import. A transient status message is simpler and matches how the operation actually works.

**Alternative considered**: A dedicated `Importing` mode that shows "Importing..." then transitions to Normal. Rejected because the blocking call means the TUI freezes during import anyway — the mode would never actually render. A post-completion status message is more useful.

### 3. Token check before import

**Choice**: Check `auth::read_token()` before calling `run_import`. If no token, set the status message to indicate the user should run `task auth todoist` from the CLI.

**Why**: Fail fast with a clear message rather than making API calls that will return 401.

### 4. Save after import, then clamp selection

**Choice**: After `run_import` succeeds, call `app.save()` and `app.clamp_selection()` to persist and update the view.

**Why**: Follows the existing pattern used by all other mutating operations (add, delete, toggle, etc.).

## Risks / Trade-offs

- **Blocking UI during import** → Acceptable for now. Typical imports take <2s. The TUI will appear frozen during this time. If this becomes a problem, we can add async support later.
- **No progress indicator** → Since the call is blocking and the TUI can't render mid-call, we can't show a progress bar. The status message after completion is the feedback mechanism.
- **Token staleness** → A token that worked at TUI launch might expire during a session. The `run_import` error handling already returns a clear message for 401 errors, which we surface as a status message.
