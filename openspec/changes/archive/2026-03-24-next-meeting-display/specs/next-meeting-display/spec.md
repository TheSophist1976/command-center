## ADDED Requirements

### Requirement: Fetch next Outlook meeting via osascript
On macOS, the system SHALL query Microsoft Outlook for the next upcoming calendar event by running `osascript` with an AppleScript that returns the event subject and start time. The result SHALL be formatted as `"HH:MM AM/PM — <title>"` (title truncated to 40 characters). This requirement is macOS-only and compiled out on other platforms.

#### Scenario: Outlook running with upcoming event
- **WHEN** Outlook is running and has at least one calendar event scheduled after the current time
- **THEN** `fetch_next_meeting()` SHALL return `Some("10:30 AM — Weekly Standup")` (example)

#### Scenario: Outlook not running or no upcoming events
- **WHEN** osascript returns an error or empty output
- **THEN** `fetch_next_meeting()` SHALL return `None`

#### Scenario: Long meeting title truncated
- **WHEN** the meeting title exceeds 40 characters
- **THEN** the displayed title SHALL be truncated to 40 characters with `…` appended

### Requirement: Async background fetch with periodic refresh
The TUI SHALL fetch meeting data in a background thread at startup and refresh it every 5 minutes. The fetch SHALL NOT block TUI startup or the event loop. Results SHALL be delivered via an `mpsc::channel` checked on each event loop tick.

#### Scenario: Initial fetch completes after startup
- **WHEN** the TUI starts
- **THEN** the header SHALL initially render without a meeting line, and update to show the meeting once the first fetch completes

#### Scenario: Periodic refresh
- **WHEN** 5 minutes have elapsed since the last fetch
- **THEN** the background thread SHALL re-fetch and send the updated meeting data

### Requirement: Meeting displayed in TUI header on all views
The TUI header SHALL show the next meeting string on all views (Today, All, Weekly, Monthly, Yearly, NoDueDate, Recurring, Notes) when `next_meeting` is `Some`. The meeting string SHALL appear below the view name line, visually distinct (dimmer color). When `next_meeting` is `None`, the header SHALL render as it did before (no empty line added).

#### Scenario: Meeting shown on Today view
- **WHEN** `next_meeting` is `Some("2:00 PM — Design Review")` and the Today view is active
- **THEN** the header SHALL show the meeting string below the view name

#### Scenario: Meeting shown on Notes view
- **WHEN** `next_meeting` is `Some(...)` and the Notes view is active
- **THEN** the header SHALL show the meeting string

#### Scenario: No meeting shown when None
- **WHEN** `next_meeting` is `None`
- **THEN** the header SHALL render without a meeting line and the header height SHALL remain 1 row

### Requirement: Header height adjusts for meeting line
When `next_meeting` is `Some`, the header area SHALL expand from 1 row to 2 rows to accommodate the meeting line. All other layout constraints SHALL adjust accordingly.

#### Scenario: Header height is 2 when meeting present
- **WHEN** `next_meeting` is `Some`
- **THEN** the header layout constraint SHALL be `Constraint::Length(2)`

#### Scenario: Header height is 1 when no meeting
- **WHEN** `next_meeting` is `None`
- **THEN** the header layout constraint SHALL be `Constraint::Length(1)`
