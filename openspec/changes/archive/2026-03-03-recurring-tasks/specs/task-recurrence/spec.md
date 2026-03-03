## ADDED Requirements

### Requirement: Recurrence data model
The system SHALL define a `Recurrence` enum with two variants: `Interval(IntervalUnit)` for simple repeating intervals (daily, weekly, monthly, yearly), and `NthWeekday { n: u8, weekday: chrono::Weekday }` for nth-weekday-of-month patterns. The `IntervalUnit` enum SHALL have variants `Daily`, `Weekly`, `Monthly`, and `Yearly`. Both `Recurrence` and `IntervalUnit` SHALL derive `Debug`, `Clone`, `PartialEq`, and implement `Display` and `FromStr`.

#### Scenario: Parse simple interval from string
- **WHEN** `"weekly"` is parsed via `Recurrence::from_str`
- **THEN** the result SHALL be `Recurrence::Interval(IntervalUnit::Weekly)`

#### Scenario: Parse nth weekday from string
- **WHEN** `"monthly:3:thu"` is parsed via `Recurrence::from_str`
- **THEN** the result SHALL be `Recurrence::NthWeekday { n: 3, weekday: Weekday::Thu }`

#### Scenario: Display simple interval
- **WHEN** `Recurrence::Interval(IntervalUnit::Daily)` is formatted with `Display`
- **THEN** the output SHALL be `"daily"`

#### Scenario: Display nth weekday
- **WHEN** `Recurrence::NthWeekday { n: 3, weekday: Weekday::Thu }` is formatted with `Display`
- **THEN** the output SHALL be `"monthly:3:thu"`

#### Scenario: Invalid recurrence string
- **WHEN** `"biweekly"` is parsed via `Recurrence::from_str`
- **THEN** the result SHALL be an error

#### Scenario: Parse weekday abbreviations
- **WHEN** any three-letter weekday abbreviation (mon, tue, wed, thu, fri, sat, sun) is used in the nth-weekday format
- **THEN** the parser SHALL correctly map it to the corresponding `chrono::Weekday`

### Requirement: Next occurrence calculation
The system SHALL provide a `next_due_date` function that takes a `Recurrence` and an optional `NaiveDate` (the current due date) and returns the next `NaiveDate`. For `Interval` variants: if a current due date is provided, add 1 day/week/month/year to it; if no due date, calculate from today. For `NthWeekday`: find the nth occurrence of the weekday in the next month after the reference date. If the nth weekday does not exist in a month (e.g., 5th Friday), the system SHALL skip to the next month that contains it.

#### Scenario: Weekly interval from due date
- **WHEN** `next_due_date` is called with `Recurrence::Interval(Weekly)` and due date `2026-03-02`
- **THEN** the result SHALL be `2026-03-09`

#### Scenario: Monthly interval from due date
- **WHEN** `next_due_date` is called with `Recurrence::Interval(Monthly)` and due date `2026-01-31`
- **THEN** the result SHALL be `2026-02-28` (clamped to month end)

#### Scenario: Yearly interval from due date
- **WHEN** `next_due_date` is called with `Recurrence::Interval(Yearly)` and due date `2024-02-29`
- **THEN** the result SHALL be `2025-02-28` (clamped to month end for leap year edge case)

#### Scenario: No due date falls back to today
- **WHEN** `next_due_date` is called with `Recurrence::Interval(Daily)` and no due date
- **THEN** the result SHALL be tomorrow (today + 1 day)

#### Scenario: Nth weekday calculation
- **WHEN** `next_due_date` is called with `NthWeekday { n: 3, weekday: Thu }` and due date `2026-03-19` (3rd Thu of March)
- **THEN** the result SHALL be `2026-04-16` (3rd Thu of April)

#### Scenario: Nth weekday skip month
- **WHEN** `next_due_date` is called with `NthWeekday { n: 5, weekday: Fri }` and due date `2026-01-30` (5th Fri of January)
- **THEN** the result SHALL skip February (which has no 5th Friday) and return the 5th Friday of the next month that has one

### Requirement: Recurrence metadata serialization
The parser SHALL support a `recur` metadata key in the task HTML comment. Simple intervals SHALL be serialized as `recur:daily`, `recur:weekly`, `recur:monthly`, `recur:yearly`. Nth-weekday patterns SHALL be serialized as `recur:monthly:N:DAY` (e.g., `recur:monthly:3:thu`). The parser SHALL round-trip the recurrence field correctly.

#### Scenario: Parse recurrence from metadata
- **WHEN** a task metadata comment contains `recur:weekly`
- **THEN** the parsed task SHALL have `recurrence: Some(Recurrence::Interval(Weekly))`

#### Scenario: Parse nth weekday from metadata
- **WHEN** a task metadata comment contains `recur:monthly:3:thu`
- **THEN** the parsed task SHALL have `recurrence: Some(Recurrence::NthWeekday { n: 3, weekday: Thu })`

#### Scenario: Serialize recurrence to metadata
- **WHEN** a task with `recurrence: Some(Recurrence::Interval(Weekly))` is serialized
- **THEN** the metadata comment SHALL contain `recur:weekly`

#### Scenario: No recurrence omitted from metadata
- **WHEN** a task with `recurrence: None` is serialized
- **THEN** the metadata comment SHALL NOT contain a `recur` key

#### Scenario: Round-trip preserves recurrence
- **WHEN** a task file with recurrence metadata is parsed and re-serialized
- **THEN** the recurrence values SHALL be identical
