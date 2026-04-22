## ADDED Requirements

### Requirement: TUI auto-filter by agent on launch
When `task-tui` launches, the TUI SHALL check whether the current working directory matches any agent profile in config (using longest-prefix matching). If a match is found, the TUI SHALL automatically apply an `agent:<name>` filter so only that agent's tasks are visible. The filter SHALL be displayed in the header and clearable by pressing `Esc` in Normal mode.

#### Scenario: CWD matches an agent profile
- **WHEN** `task-tui` is launched from `/Users/mark/code/command-center/src` and config contains `agent-command-center: ~/code/command-center`
- **THEN** the TUI SHALL launch with filter `agent:command-center` active, showing only tasks where `agent:command-center` in metadata

#### Scenario: No matching agent profile
- **WHEN** `task-tui` is launched from a directory that does not match any agent profile
- **THEN** the TUI SHALL launch with no filter applied (same as current behaviour)

#### Scenario: Multiple profiles, longest match wins
- **WHEN** config contains `agent-root: ~/code` and `agent-app: ~/code/myapp` and CWD is `~/code/myapp/src`
- **THEN** the TUI SHALL apply filter `agent:app` (the longer match)

#### Scenario: Auto-filter visible in header
- **WHEN** the auto-filter is active
- **THEN** the header SHALL display the active filter expression (same as any manually applied filter)

#### Scenario: User can clear auto-filter
- **WHEN** the auto-filter is active and the user presses `Esc`
- **THEN** the filter SHALL be cleared and all tasks SHALL be visible
