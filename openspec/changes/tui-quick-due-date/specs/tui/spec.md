## ADDED Requirements

### Requirement: Quick due date assignment
The user SHALL set a task's due date directly from Normal mode using Shift-letter keybindings. Pressing `T` SHALL set the due date to today, `W` to one week from today (+7 days), `M` to one month from today, `Q` to three months from today (next quarter), and `X` SHALL clear the due date (set to None). The change SHALL be persisted to disk immediately. A status message SHALL confirm the new date or clearance. All due date keys SHALL be no-ops when no task is selected.

#### Scenario: Set due date to today
- **WHEN** the user presses `T` on a selected task
- **THEN** the task's `due_date` SHALL be set to today's date, the `updated` timestamp SHALL be set, the file SHALL be saved, and a status message "Due: YYYY-MM-DD" SHALL be displayed

#### Scenario: Set due date to next week
- **WHEN** the user presses `W` on a selected task
- **THEN** the task's `due_date` SHALL be set to 7 days from today, the file SHALL be saved, and a status message SHALL confirm the date

#### Scenario: Set due date to next month
- **WHEN** the user presses `M` on a selected task
- **THEN** the task's `due_date` SHALL be set to one calendar month from today (clamped to month-end if needed), the file SHALL be saved, and a status message SHALL confirm the date

#### Scenario: Set due date to next quarter
- **WHEN** the user presses `Q` on a selected task
- **THEN** the task's `due_date` SHALL be set to three calendar months from today, the file SHALL be saved, and a status message SHALL confirm the date

#### Scenario: Clear due date
- **WHEN** the user presses `X` on a selected task that has a due date
- **THEN** the task's `due_date` SHALL be set to None, the file SHALL be saved, and a status message "Due date cleared" SHALL be displayed

#### Scenario: No-op when list is empty
- **WHEN** the user presses `T`, `W`, `M`, `Q`, or `X` and no task is selected (empty or fully filtered list)
- **THEN** the TUI SHALL remain in Normal mode with no change

## MODIFIED Requirements

### Requirement: Three-region layout
The TUI SHALL render a three-region layout in Normal mode: a header bar (1 line) showing the title and active filter summary, a scrollable task table filling the remaining space, and a footer bar (1 line) showing context-sensitive keybinding hints. In NlpChat mode, the TUI SHALL render a four-region layout: header (1 line), task table (top ~60%), chat panel (bottom ~40%), and input prompt (1 line).

#### Scenario: Default layout rendering
- **WHEN** the TUI is displayed with tasks loaded in Normal mode
- **THEN** the header SHALL show "task-manager" and the footer SHALL show keybinding hints including `j/k:nav  Enter:toggle  a:add  d:delete  f:filter  p:priority  e:edit  t:tags  r:desc  v:view  i:import  ::command  D:set-dir  T/W/M/Q:due  X:clr-due  q:quit`

#### Scenario: Filter active in header
- **WHEN** a filter is active (e.g., status:open)
- **THEN** the header SHALL display the active filter expression alongside the title

#### Scenario: Status message in footer
- **WHEN** a status message is set and the TUI is in Normal mode
- **THEN** the footer SHALL display the status message text instead of keybinding hints

#### Scenario: NlpChat split layout
- **WHEN** the TUI is in NlpChat mode
- **THEN** the layout SHALL split into header, task table (top portion), chat panel (bottom portion), and an input prompt line at the bottom
