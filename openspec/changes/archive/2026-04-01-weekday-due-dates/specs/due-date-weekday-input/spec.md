## ADDED Requirements

### Requirement: parse_due_date_input function
The system SHALL provide a `parse_due_date_input(s: &str, today: NaiveDate) -> Option<NaiveDate>` function in `src/parser.rs`. It SHALL first attempt to parse `s` as `%Y-%m-%d`. If that fails, it SHALL attempt to match `s` case-insensitively against the full English weekday names (`monday`…`sunday`) and three-letter abbreviations (`mon`…`sun`). On a weekday match, it SHALL return the next future date after `today` whose weekday equals the matched day. If neither parse succeeds, it SHALL return `None`.

#### Scenario: ISO date passes through
- **WHEN** `parse_due_date_input("2026-04-15", today)` is called
- **THEN** the result SHALL be `Some(NaiveDate::from_ymd(2026, 4, 15))`

#### Scenario: Full weekday name resolves to next occurrence
- **WHEN** `parse_due_date_input("Monday", today)` is called and today is Tuesday 2026-04-01
- **THEN** the result SHALL be `Some(NaiveDate::from_ymd(2026, 4, 6))` (the following Monday)

#### Scenario: Three-letter abbreviation resolves to next occurrence
- **WHEN** `parse_due_date_input("fri", today)` is called and today is Monday 2026-03-30
- **THEN** the result SHALL be `Some(NaiveDate::from_ymd(2026, 04, 03))` (that same week's Friday)

#### Scenario: Weekday name is case-insensitive
- **WHEN** `parse_due_date_input("TUESDAY", today)` is called
- **THEN** the result SHALL resolve identically to `"tuesday"` or `"Tuesday"`

#### Scenario: Same weekday as today resolves to next week
- **WHEN** `parse_due_date_input("wednesday", today)` is called and today is Wednesday 2026-04-01
- **THEN** the result SHALL be `Some(NaiveDate::from_ymd(2026, 4, 8))` (7 days later, never today)

#### Scenario: Unrecognized input returns None
- **WHEN** `parse_due_date_input("soon", today)` is called
- **THEN** the result SHALL be `None`

#### Scenario: Empty string returns None
- **WHEN** `parse_due_date_input("", today)` is called
- **THEN** the result SHALL be `None`

### Requirement: TUI detail edit panel accepts weekday input
The TUI detail edit panel SHALL use `parse_due_date_input` when validating and saving the due date field. If the input is non-empty and `parse_due_date_input` returns `None`, the panel SHALL display the error `"Invalid date (use YYYY-MM-DD or a weekday name)"` and remain in editing mode. If it returns `Some(date)`, the task's `due_date` SHALL be set to that date.

#### Scenario: Weekday accepted in edit panel
- **WHEN** the user types `"monday"` in the due date field and saves
- **THEN** the task due date SHALL be set to the next Monday after today

#### Scenario: Invalid input rejected with updated message
- **WHEN** the user types `"asap"` in the due date field and saves
- **THEN** the status message SHALL be `"Invalid date (use YYYY-MM-DD or a weekday name)"` and the task SHALL not be saved

#### Scenario: Empty due date clears the field
- **WHEN** the user clears the due date field and saves
- **THEN** the task's `due_date` SHALL be set to `None`

### Requirement: NLP set_fields due date path unchanged
The NLP response processing path (`set_fields.due_date`) SHALL continue to accept only `YYYY-MM-DD` strings, since Claude already resolves natural language to ISO dates before returning them. No weekday parsing is needed in this path.

#### Scenario: NLP YYYY-MM-DD still applied
- **WHEN** the NLP response sets `due_date` to `"2026-05-01"`
- **THEN** the task's `due_date` SHALL be set to `2026-05-01`

#### Scenario: NLP invalid date still shows error
- **WHEN** the NLP response sets `due_date` to an unparseable string
- **THEN** the status message SHALL indicate an invalid due date format
