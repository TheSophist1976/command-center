## ADDED Requirements

### Requirement: columns config key
The `columns:` config key SHALL define which task table columns are shown and their display order, as a comma-separated list of column identifiers. Valid identifiers are: `id`, `status`, `priority`, `title`, `desc`, `due`, `project`, `agent`, `recur`, `note`, `tags`. If the key is absent or empty, the TUI SHALL fall back to the current auto-show logic (show a column only if at least one visible task has a non-empty value for it). The `id`, `status`, `priority`, and `title` columns are always shown regardless of this setting.

#### Scenario: Columns key controls visible columns
- **WHEN** config contains `columns: id,status,priority,title,due,agent`
- **THEN** the task table SHALL display exactly those columns in that order

#### Scenario: Unknown column identifiers ignored
- **WHEN** config contains `columns: id,status,priority,title,unknown-col`
- **THEN** the unknown identifier SHALL be silently ignored

#### Scenario: Absent key uses auto-show logic
- **WHEN** the config file does not contain a `columns:` key
- **THEN** the TUI SHALL show columns dynamically based on data presence (existing behavior)

### Requirement: group-by config key
The `group-by:` config key SHALL define the field by which the task table is grouped on startup. Valid values are: `none`, `agent`, `project`, `priority`. If absent or set to `none`, no grouping is applied. The TUI SHALL write this key whenever the user changes the active grouping via `:group` or `G`.

#### Scenario: group-by restored on startup
- **WHEN** config contains `group-by: agent`
- **THEN** the TUI SHALL start with agent grouping active

#### Scenario: group-by none disables grouping
- **WHEN** config contains `group-by: none`
- **THEN** the TUI SHALL start with no grouping

#### Scenario: Absent group-by defaults to none
- **WHEN** the config file does not contain a `group-by:` key
- **THEN** the TUI SHALL start with no grouping applied

### Requirement: TUI grouping commands
The user SHALL type `:group <field>` in normal mode to set the active grouping. Valid fields are `agent`, `project`, `priority`, and `none`. The new grouping SHALL take effect immediately, re-rendering the task table with section headers. The active grouping SHALL be saved to config. Pressing `G` in normal mode SHALL cycle through groupings in the order: none → project → agent → priority → none.

#### Scenario: Set grouping via command
- **WHEN** the user types `:group agent` and presses Enter
- **THEN** the task table SHALL render tasks grouped under agent section headers and `group-by: agent` SHALL be written to config

#### Scenario: Clear grouping via command
- **WHEN** the user types `:group none` and presses Enter
- **THEN** the task table SHALL render as a flat sorted list and `group-by: none` SHALL be written to config

#### Scenario: G cycles grouping
- **WHEN** the active grouping is `none` and the user presses `G`
- **THEN** the grouping SHALL change to `project`

#### Scenario: G wraps back to none
- **WHEN** the active grouping is `priority` and the user presses `G`
- **THEN** the grouping SHALL change to `none`

#### Scenario: Invalid group field shows error
- **WHEN** the user types `:group unknown`
- **THEN** the TUI SHALL display a status message "Unknown group field. Valid: agent, project, priority, none"
