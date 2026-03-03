## Why

Some tasks repeat on a regular schedule — weekly reviews, monthly reports, daily standups. Currently the user must manually re-create these tasks each time. A recurrence system lets the user define a repeat pattern on a task, and when the task is marked done, the system automatically creates a new open copy with an updated due date.

## Capabilities

### New Capabilities

- **task-recurrence**: Define recurrence rules on tasks (daily, weekly, monthly, yearly, nth-weekday) and auto-generate the next occurrence when a recurring task is completed.

### Modified Capabilities

- **task-lifecycle**: The Task struct gains an optional `recurrence` field. Completing a recurring task creates a new task with the next due date.
- **tui**: The TUI displays a recurrence indicator in the task table and detail panel. A quick keybinding (`r`) in Normal mode opens an inline prompt to set recurrence on the selected task using natural language (e.g., "every third thursday", "daily", "monthly").
- **tui-nlp**: The NLP system understands recurrence-related commands. Users can set, change, or remove recurrence via the NLP chat (e.g., "make task 5 repeat every third thursday"). The NLP system prompt includes recurrence info for each task.
- **cli-interface**: The `add` and `edit` commands gain a `--recur` flag.

## Impact

- `src/task.rs`: Add `Recurrence` enum and `recurrence: Option<Recurrence>` field to Task. Recurrence supports simple intervals (daily, weekly, monthly, yearly) and nth-weekday patterns (e.g., 3rd Thursday).
- `src/parser.rs`: Parse and serialize the `recur` metadata field (e.g., `recur: weekly`, `recur: monthly:3:thu`)
- `src/main.rs`: Handle `--recur` flag on add/edit commands; create next occurrence on `done`
- `src/tui.rs`: Show recurrence indicator (↻) in table; `r` keybinding opens inline NLP prompt for quick recurrence setting; detail panel shows and edits recurrence
- `src/nlp.rs`: Add `SetRecurrence` action variant. Include recurrence info in task context. Parse natural language recurrence patterns (e.g., "every third thursday" → monthly:3:thu). Support recurrence commands in NLP chat.
- `src/storage.rs`: Serialization of the new field (handled by parser changes)
