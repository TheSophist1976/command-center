## REMOVED Requirements

### Requirement: Import Todoist tasks from TUI
**Reason**: Todoist import is CLI-only. Users should run `task import todoist` from the command line. Keeping an import trigger in the TUI created coupling between the TUI and Todoist API concerns.
**Migration**: Use `task import todoist` (or `task import todoist --test`) from the terminal to import tasks.

### Requirement: Status message display
**Reason**: Removed as part of the Todoist TUI import removal. The `status_message` field on `App` may be retained if other TUI features use it, but the Todoist-import-specific behavior is no longer required.
**Migration**: None — this was an internal TUI mechanism tied to the import feature.
