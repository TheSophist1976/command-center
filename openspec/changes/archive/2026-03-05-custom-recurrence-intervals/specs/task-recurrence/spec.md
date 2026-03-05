## MODIFIED Requirements

### Requirement: Recurrence data model
The system SHALL define a `Recurrence` enum with two variants: `Interval { unit: IntervalUnit, count: u32 }` for repeating intervals with a multiplier (e.g., every 3 months), and `NthWeekday { n: u8, weekday: chrono::Weekday }` for nth-weekday-of-month patterns. The `IntervalUnit` enum SHALL have variants `Daily`, `Weekly`, `Monthly`, and `Yearly`. The `count` field SHALL be a positive integer >= 1, where 1 represents the base interval (every day, every week, etc.) and higher values represent multiples (every N days, every N weeks, etc.).

#### Scenario: Parse simple interval from string (backward compat)
- **WHEN** `"weekly"` is parsed via `Recurrence::from_str`
- **THEN** the result SHALL be `Recurrence::Interval { unit: IntervalUnit::Weekly, count: 1 }`

#### Scenario: Parse interval with count from string
- **WHEN** `"monthly:3"` is parsed via `Recurrence::from_str`
- **THEN** the result SHALL be `Recurrence::Interval { unit: IntervalUnit::Monthly, count: 3 }`

#### Scenario: Parse nth weekday from string (unchanged)
- **WHEN** `"monthly:3:thu"` is parsed via `Recurrence::from_str`
- **THEN** the result SHALL be `Recurrence::NthWeekday { n: 3, weekday: Weekday::Thu }`

#### Scenario: Display interval with count 1
- **WHEN** `Recurrence::Interval { unit: IntervalUnit::Daily, count: 1 }` is formatted with `Display`
- **THEN** the output SHALL be `"daily"` (no count suffix)

#### Scenario: Display interval with count > 1
- **WHEN** `Recurrence::Interval { unit: IntervalUnit::Monthly, count: 3 }` is formatted with `Display`
- **THEN** the output SHALL be `"monthly:3"`

#### Scenario: Invalid count of 0
- **WHEN** `"daily:0"` is parsed via `Recurrence::from_str`
- **THEN** the result SHALL be an error

### Requirement: Next occurrence calculation
The system SHALL provide a `next_due_date` function that takes a `Recurrence` and an optional `NaiveDate` (the current due date) and returns the next `NaiveDate`. For `Interval` variants: multiply the base interval by `count` — add `count` days/weeks/months/years. If no due date, calculate from today. For `NthWeekday`: unchanged.

#### Scenario: Weekly interval with count 1
- **WHEN** `next_due_date` is called with `Interval { unit: Weekly, count: 1 }` and due date `2026-03-02`
- **THEN** the result SHALL be `2026-03-09`

#### Scenario: Monthly interval with count 3
- **WHEN** `next_due_date` is called with `Interval { unit: Monthly, count: 3 }` and due date `2026-01-15`
- **THEN** the result SHALL be `2026-04-15`

#### Scenario: Daily interval with count 5
- **WHEN** `next_due_date` is called with `Interval { unit: Daily, count: 5 }` and due date `2026-03-01`
- **THEN** the result SHALL be `2026-03-06`

### Requirement: Recurrence metadata serialization
The parser SHALL support a `recur` metadata key in the task HTML comment. Simple intervals with count 1 SHALL be serialized as `recur:daily`, `recur:weekly`, `recur:monthly`, `recur:yearly` (unchanged). Intervals with count > 1 SHALL be serialized as `recur:daily:3`, `recur:weekly:2`, `recur:monthly:3`, etc. Existing task files with count-1 format SHALL parse correctly.

#### Scenario: Serialize interval with count 1
- **WHEN** a task with `Interval { unit: Weekly, count: 1 }` is serialized
- **THEN** the metadata comment SHALL contain `recur:weekly`

#### Scenario: Serialize interval with count > 1
- **WHEN** a task with `Interval { unit: Monthly, count: 3 }` is serialized
- **THEN** the metadata comment SHALL contain `recur:monthly:3`

#### Scenario: Parse existing file without count
- **WHEN** a task metadata comment contains `recur:weekly`
- **THEN** the parsed task SHALL have `recurrence: Some(Interval { unit: Weekly, count: 1 })`
