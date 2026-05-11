## ADDED Requirements

### Requirement: effort metadata key
The task metadata comment SHALL support an optional `effort` key with values `high`, `medium`, or `low`. When absent, the field SHALL default to `None`.

#### Scenario: Task with high effort serialized
- **WHEN** a task has `effort: Some(Effort::High)`
- **THEN** the metadata comment SHALL contain `effort:high`

#### Scenario: Task with medium effort serialized
- **WHEN** a task has `effort: Some(Effort::Medium)`
- **THEN** the metadata comment SHALL contain `effort:medium`

#### Scenario: Task with low effort serialized
- **WHEN** a task has `effort: Some(Effort::Low)`
- **THEN** the metadata comment SHALL contain `effort:low`

#### Scenario: Task without effort not serialized
- **WHEN** a task has `effort: None`
- **THEN** the metadata comment SHALL NOT contain an `effort` key

#### Scenario: Parse effort:high
- **WHEN** a metadata comment contains `effort:high`
- **THEN** the parser SHALL set `effort` to `Some(Effort::High)`

#### Scenario: Parse effort:medium
- **WHEN** a metadata comment contains `effort:medium`
- **THEN** the parser SHALL set `effort` to `Some(Effort::Medium)`

#### Scenario: Parse effort:low
- **WHEN** a metadata comment contains `effort:low`
- **THEN** the parser SHALL set `effort` to `Some(Effort::Low)`

#### Scenario: Missing effort key parses as None
- **WHEN** a metadata comment does not contain an `effort` key
- **THEN** the parser SHALL set `effort` to `None` without error

#### Scenario: Unknown effort value tolerantly ignored
- **WHEN** a metadata comment contains `effort:unknown`
- **THEN** the parser SHALL set `effort` to `None` without error in tolerant mode

#### Scenario: Effort round-trips correctly
- **WHEN** a task with `effort: Some(Effort::Medium)` is serialized and then parsed
- **THEN** the parsed task SHALL have `effort: Some(Effort::Medium)`
