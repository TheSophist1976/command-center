## ADDED Requirements

### Requirement: Effort enum
The system SHALL define an `Effort` enum with three variants: `High`, `Medium`, and `Low`. Each variant represents the cognitive load required to complete a task.

- `High` — requires dedicated, uninterrupted time and full concentration
- `Medium` — requires dedicated time but low cognitive energy  
- `Low` — can be completed quickly while multitasking or context-switching

#### Scenario: Effort variants exist
- **WHEN** the Effort enum is defined
- **THEN** it SHALL have exactly three variants: High, Medium, Low

### Requirement: Effort field on Task
The `Task` struct SHALL include an optional `effort` field of type `Option<Effort>`. When absent, it SHALL default to `None`.

#### Scenario: New task defaults to no effort
- **WHEN** a task is created without specifying effort
- **THEN** `task.effort` SHALL be `None`

#### Scenario: Task with effort set
- **WHEN** a task is created with `effort: Some(Effort::High)`
- **THEN** `task.effort` SHALL equal `Some(Effort::High)`
