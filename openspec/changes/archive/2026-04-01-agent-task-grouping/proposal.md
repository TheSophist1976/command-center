## Why

Tasks are assigned to different agents (command-center, AI-Assisted, human, etc.) but the TUI has no way to see tasks organized by who owns them. A dedicated agent-grouped view makes it easy to review workload per agent at a glance.

## What Changes

- Add a new `View::ByAgent` to the TUI that displays all open tasks grouped under agent section headers
- Tasks with the same `agent` value are displayed together; tasks with no agent are grouped under "Unassigned"
- Agent groups are sorted alphabetically; tasks within each group retain the existing due-date/priority sort order
- Add `A` keybinding in Normal mode to switch to the By Agent view
- Add the view name to the footer hint bar

## Capabilities

### New Capabilities
- `tui-agent-view`: A grouped task display that renders section headers per agent, showing all open tasks organized by their assigned agent value

### Modified Capabilities
- `tui-views`: Add `ByAgent` variant to the `View` enum and its switching keybinding

## Impact

- `src/tui.rs`: add `View::ByAgent` variant, keybinding `A`, and the grouped rendering logic
- `openspec/specs/tui-views/spec.md`: delta spec adding ByAgent view requirements
