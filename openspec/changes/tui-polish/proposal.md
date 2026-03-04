## Why

Two small UX issues: (1) The NLP "Thinking..." status message is static text — users can't tell if the app is still working during potentially slow API calls with tool-use loops. An animated indicator gives better feedback. (2) Tasks due today are being treated as overdue in the TUI row styling, which is incorrect — a task due today still has the rest of the day.

## What Changes

- Replace the static "Thinking..." status with an animated loading indicator (e.g., spinner or dots) that updates while waiting for the NLP API response
- Fix the overdue date comparison to use `<=` today (i.e., only dates strictly before today are overdue, not today itself)

## Capabilities

### New Capabilities
None

### Modified Capabilities
- `tui`: Animated loading indicator during NLP calls; fix overdue date boundary to exclude today

## Impact

- `src/tui.rs`: Loading animation logic, overdue comparison fix
