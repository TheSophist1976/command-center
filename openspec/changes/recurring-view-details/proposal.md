## Why

The recurring tasks view shows a "↻" icon to indicate a task recurs, but doesn't show the actual recurrence pattern. Users need to see whether a task repeats daily, weekly, monthly, etc. without opening task details.

## What Changes

- Add a "Pattern" column to the task table that displays the recurrence pattern text (e.g., "Weekly", "Monthly", "3rd Thu") when any filtered task has a recurrence
- Keep the existing "↻" indicator column unchanged

## Capabilities

### New Capabilities
_(none)_

### Modified Capabilities
- `tui`: Add recurrence pattern column to the task table

## Impact

- `src/tui.rs`: Add column header, cell rendering, and width constraint for the new pattern column
