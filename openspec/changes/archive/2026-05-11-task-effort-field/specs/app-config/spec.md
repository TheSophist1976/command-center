## MODIFIED Requirements

### Requirement: Per-view grouping config keys
The application SHALL store each view's grouping independently using the key pattern `group-by.<view>`, where `<view>` is the view's config name: `due`, `no-due-date`, `recurring`, `notes`. Valid values for each key are: `none`, `agent`, `project`, `priority`, `effort`, `due-date`. On launch, each view's grouping SHALL be restored from its key, defaulting to `none` if absent. The legacy global `group-by` key SHALL be ignored.

#### Scenario: Per-view grouping persisted
- **WHEN** the user sets grouping to `agent` while in the Due view
- **THEN** the config SHALL contain `group-by.due: agent`

#### Scenario: Different views have independent groupings
- **WHEN** the config contains `group-by.due: agent` and `group-by.recurring: priority`
- **THEN** the Due view SHALL use `agent` grouping and the Recurring view SHALL use `priority` grouping

#### Scenario: Missing key defaults to none
- **WHEN** the config does not contain `group-by.no-due-date`
- **THEN** the NoDueDate view SHALL start with `GroupBy::None`

#### Scenario: Legacy group-by key ignored
- **WHEN** the config contains `group-by: agent` (the old global key)
- **THEN** the application SHALL ignore it and use per-view defaults

#### Scenario: Effort grouping persisted and restored
- **WHEN** the user sets grouping to `effort` while in the Recurring view
- **THEN** the config SHALL contain `group-by.recurring: effort` and on next launch the Recurring view SHALL restore `GroupBy::Effort`
