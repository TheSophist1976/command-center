## ADDED Requirements

### Requirement: agent metadata field on tasks
The task metadata comment SHALL support an optional `agent` key whose value is a profile name string (e.g., `agent:command-center`) or the literal `human`. When absent, the field SHALL default to `None`.

#### Scenario: Task with agent field serialized
- **WHEN** a task has `agent: Some("command-center")`
- **THEN** the metadata comment SHALL contain `agent:command-center`

#### Scenario: Task with human agent serialized
- **WHEN** a task has `agent: Some("human")`
- **THEN** the metadata comment SHALL contain `agent:human`

#### Scenario: Task without agent field serialized
- **WHEN** a task has `agent: None`
- **THEN** the metadata comment SHALL NOT contain an `agent` key

#### Scenario: Parse agent field
- **WHEN** a metadata comment contains `agent:command-center`
- **THEN** the parser SHALL set `agent` to `Some("command-center")`

#### Scenario: Parse human agent
- **WHEN** a metadata comment contains `agent:human`
- **THEN** the parser SHALL set `agent` to `Some("human")`

#### Scenario: Missing agent field parses as None
- **WHEN** a metadata comment does not contain an `agent` key
- **THEN** the parser SHALL set `agent` to `None` without error

#### Scenario: Existing files without agent field parse correctly
- **WHEN** a task file written before this change (no `agent` key in any metadata comment) is loaded
- **THEN** all tasks SHALL have `agent: None` and the file SHALL parse without error
