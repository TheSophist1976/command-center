## Why

The NLP update action currently supports changing `priority`, `status`, and `tags` on tasks, but not `due_date`. Users naturally ask things like "set overdue tasks to today" or "move task 5 to next week" and the model correctly reports it cannot do this. Adding `due_date` to the update action's set fields closes this gap.

## What Changes

- Add `due_date` to the `SetFields` struct in `src/nlp.rs` as an `Option<String>` (YYYY-MM-DD format, or null to clear)
- Update the system prompt's update action format to include `due_date` in the `set` field
- Update the TUI update execution to apply due date changes to matched tasks
- Update the TUI update preview to show due date changes (before → after)

## Capabilities

### New Capabilities
None

### Modified Capabilities
- `tui-nlp`: Update the NLP update action to support `due_date` in set fields, including confirmation preview and execution

## Impact

- `src/nlp.rs`: `SetFields` struct, system prompt, action summary formatting
- `src/tui.rs`: Update execution handler, update preview formatter
