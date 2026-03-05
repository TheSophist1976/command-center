### Requirement: WeeklyOn recurrence variant
The `Recurrence` enum SHALL include a `WeeklyOn { weekday: Weekday, every_n_weeks: u32 }` variant that represents a recurrence anchored to a specific day of the week, repeating every N weeks.

#### Scenario: Create weekly-on-Friday recurrence
- **WHEN** a `WeeklyOn` variant is created with `weekday: Fri` and `every_n_weeks: 1`
- **THEN** it SHALL represent "every week on Friday"

#### Scenario: Create biweekly-on-Monday recurrence
- **WHEN** a `WeeklyOn` variant is created with `weekday: Mon` and `every_n_weeks: 2`
- **THEN** it SHALL represent "every 2 weeks on Monday"

### Requirement: WeeklyOn parsing
The `Recurrence::from_str` implementation SHALL parse `weekly:DAY` as `WeeklyOn { weekday: DAY, every_n_weeks: 1 }` and `weekly:N:DAY` as `WeeklyOn { weekday: DAY, every_n_weeks: N }`. The weekday SHALL accept three-letter abbreviations (mon, tue, wed, thu, fri, sat, sun), case-insensitive. When the second part of a two-part `weekly:X` format is not a valid number, it SHALL be treated as a weekday name for the `WeeklyOn` variant. The count N SHALL be >= 1.

#### Scenario: Parse weekly:fri
- **WHEN** `"weekly:fri"` is parsed via `Recurrence::from_str`
- **THEN** the result SHALL be `WeeklyOn { weekday: Fri, every_n_weeks: 1 }`

#### Scenario: Parse weekly:2:mon
- **WHEN** `"weekly:2:mon"` is parsed via `Recurrence::from_str`
- **THEN** the result SHALL be `WeeklyOn { weekday: Mon, every_n_weeks: 2 }`

#### Scenario: Parse weekly:1:wed
- **WHEN** `"weekly:1:wed"` is parsed via `Recurrence::from_str`
- **THEN** the result SHALL be `WeeklyOn { weekday: Wed, every_n_weeks: 1 }`

#### Scenario: Existing weekly:2 still parses as Interval
- **WHEN** `"weekly:2"` is parsed via `Recurrence::from_str`
- **THEN** the result SHALL be `Interval { unit: Weekly, count: 2 }` (unchanged behavior)

#### Scenario: Invalid weekday in weekly:X rejected
- **WHEN** `"weekly:xyz"` is parsed via `Recurrence::from_str`
- **THEN** the result SHALL be an error

### Requirement: WeeklyOn display
The `Display` implementation for `WeeklyOn` SHALL serialize as `weekly:DAY` when `every_n_weeks` is 1, and `weekly:N:DAY` when `every_n_weeks` is greater than 1. The weekday SHALL be the lowercase three-letter abbreviation.

#### Scenario: Display weekly on Friday
- **WHEN** `WeeklyOn { weekday: Fri, every_n_weeks: 1 }` is formatted with `Display`
- **THEN** the output SHALL be `"weekly:fri"`

#### Scenario: Display biweekly on Monday
- **WHEN** `WeeklyOn { weekday: Mon, every_n_weeks: 2 }` is formatted with `Display`
- **THEN** the output SHALL be `"weekly:2:mon"`

### Requirement: WeeklyOn next due date calculation
The `next_due_date` function SHALL calculate the next occurrence for `WeeklyOn` as follows: if the base date is the target weekday, the next due date SHALL be `base + (7 * every_n_weeks)` days. If the base date is not the target weekday (e.g., initial scheduling with no prior due date), the next due date SHALL be the next occurrence of the target weekday after the base date.

#### Scenario: Next Friday from a Friday base
- **WHEN** `next_due_date` is called with `WeeklyOn { weekday: Fri, every_n_weeks: 1 }` and base `2026-03-06` (a Friday)
- **THEN** the result SHALL be `2026-03-13` (next Friday)

#### Scenario: Next Friday from a Wednesday base
- **WHEN** `next_due_date` is called with `WeeklyOn { weekday: Fri, every_n_weeks: 1 }` and no due date, with today being a Wednesday
- **THEN** the result SHALL be the next Friday (2 days later)

#### Scenario: Biweekly Monday from a Monday base
- **WHEN** `next_due_date` is called with `WeeklyOn { weekday: Mon, every_n_weeks: 2 }` and base `2026-03-02` (a Monday)
- **THEN** the result SHALL be `2026-03-16` (2 Mondays later)

### Requirement: WeeklyOn TUI display formatting
The `format_recurrence_display` function SHALL format `WeeklyOn` as `"Weekly (DAY)"` when `every_n_weeks` is 1, and `"Every N Weeks (DAY)"` when `every_n_weeks` is greater than 1, where DAY is the capitalized three-letter weekday abbreviation.

#### Scenario: Display weekly Friday in TUI
- **WHEN** `format_recurrence_display` is called with `WeeklyOn { weekday: Fri, every_n_weeks: 1 }`
- **THEN** the output SHALL be `"Weekly (Fri)"`

#### Scenario: Display biweekly Monday in TUI
- **WHEN** `format_recurrence_display` is called with `WeeklyOn { weekday: Mon, every_n_weeks: 2 }`
- **THEN** the output SHALL be `"Every 2 Weeks (Mon)"`

### Requirement: WeeklyOn metadata round-trip
The parser SHALL serialize `WeeklyOn` recurrence to task metadata using the `Display` format (`weekly:DAY` or `weekly:N:DAY`) and parse it back correctly. A task file with `WeeklyOn` metadata SHALL round-trip without data loss.

#### Scenario: Serialize and parse weekly:fri
- **WHEN** a task with `WeeklyOn { weekday: Fri, every_n_weeks: 1 }` is serialized and re-parsed
- **THEN** the recurrence SHALL be identical
