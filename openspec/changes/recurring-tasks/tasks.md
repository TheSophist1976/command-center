## Tasks

- [x] Add `Recurrence` enum and `IntervalUnit` enum to `src/task.rs` with `Display`, `FromStr`, `Debug`, `Clone`, `PartialEq`. Add `recurrence: Option<Recurrence>` field to `Task` struct. Add `next_due_date` function that computes the next occurrence date. Update `make_task` test helper to include the new field.
- [x] Update `src/parser.rs` to parse `recur:<value>` from metadata comments and serialize it back. Add `recur` to the `Metadata` struct. Handle both simple intervals (`recur:weekly`) and nth-weekday patterns (`recur:monthly:3:thu`). Update round-trip tests.
- [x] Update `src/main.rs` `Done` command: after marking a recurring task done, create a new open task with the next due date, same title/tags/project/priority/recurrence. Print status message about the new occurrence. Add `--recur` flag to `Add` and `Edit` commands in `src/cli.rs`.
- [x] Update `src/tui.rs` task table to show `↻` recurrence indicator column when any visible task has recurrence. Show column conditionally (like description/due/project columns).
- [x] Update `src/tui.rs` detail panel to display recurrence field (e.g., "Recurrence: Weekly" or "Recurrence: Monthly (3rd Thu)" or "Recurrence: -").
- [x] Update `src/tui.rs` TUI toggle (Enter/Space): when completing a recurring task, call the recurrence creation logic and display a status message about the new occurrence.
- [x] Add `EditingRecurrence` mode to `src/tui.rs`. `R` key in Normal mode enters the mode with a text input prompt. On Enter, send input to NLP with a focused recurrence-parsing prompt. On success, set/clear recurrence on selected task and save. On Esc, cancel.
- [x] Add `SetRecurrence` variant to `NlpAction` in `src/nlp.rs`. Update `RawAction` deserialization, `parse_response`, system prompt (add action format #5 for set_recurrence), and `TaskSummary` to include recurrence field. Handle `SetRecurrence` in `src/tui.rs` NLP response processing.
- [x] Add focused recurrence-parsing NLP function in `src/nlp.rs` for the `R` keybinding. This is a simpler prompt that only parses recurrence patterns (not full task commands). Returns a `Recurrence` or error string.
