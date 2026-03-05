## MODIFIED Requirements

### Requirement: Task table display
When displaying recurrence patterns in the Pattern column, the `format_recurrence_display` function SHALL show "Daily", "Weekly", "Monthly", "Yearly" for count-1 intervals (unchanged), and "Every N Days", "Every N Weeks", "Every N Months", "Every N Years" for count > 1 intervals.

#### Scenario: Display count-1 interval
- **WHEN** a task has `Interval { unit: Weekly, count: 1 }`
- **THEN** the Pattern column SHALL display "Weekly"

#### Scenario: Display count > 1 interval
- **WHEN** a task has `Interval { unit: Monthly, count: 3 }`
- **THEN** the Pattern column SHALL display "Every 3 Months"
