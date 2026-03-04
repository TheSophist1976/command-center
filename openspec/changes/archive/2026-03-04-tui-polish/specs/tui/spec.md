## MODIFIED Requirements

### Requirement: NLP loading indicator
While waiting for an NLP API response, the TUI SHALL display an animated loading indicator in the status bar. The indicator SHALL cycle through "Thinking", "Thinking.", "Thinking..", "Thinking..." at approximately 200ms intervals. The NLP call SHALL run on a background thread so the main event loop can continue redrawing.

#### Scenario: Loading animation starts
- **WHEN** the user submits an NLP query
- **THEN** the TUI SHALL spawn the NLP call on a background thread and begin showing the animated "Thinking" indicator

#### Scenario: Loading animation cycles
- **WHEN** the NLP call is in progress
- **THEN** the status message SHALL cycle through dot variants on each redraw tick

#### Scenario: Loading animation stops on result
- **WHEN** the NLP call completes (success or error)
- **THEN** the animated indicator SHALL stop and the result SHALL be processed normally

### Requirement: Overdue task styling
A task SHALL be styled as overdue (red text, `[!]` marker) only if it is open AND its due date is strictly before today's date. Tasks due today SHALL NOT be considered overdue.

#### Scenario: Task due today is not overdue
- **WHEN** an open task has a due date equal to today
- **THEN** it SHALL be displayed with normal styling, not overdue styling

#### Scenario: Task due yesterday is overdue
- **WHEN** an open task has a due date before today
- **THEN** it SHALL be displayed with overdue styling (red text, `[!]` marker)
