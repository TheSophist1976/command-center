## ADDED Requirements

### Requirement: Auto-create next occurrence on completion
When a recurring task is marked as done (via CLI `done` command, TUI toggle, or NLP update), the system SHALL automatically create a new open task that copies the completed task's title, priority, tags, project, description, and recurrence. The new task's due date SHALL be calculated using `next_due_date` from the completed task's due date and recurrence. The new task SHALL receive a fresh ID from `next_id` and `created` set to the current timestamp.

#### Scenario: Complete recurring task via CLI
- **WHEN** the user runs `task done 5` and task 5 has `recurrence: Some(Weekly)` and `due_date: 2026-03-02`
- **THEN** task 5 SHALL be marked done, and a new open task SHALL be created with the same title, tags, project, priority, recurrence, and `due_date: 2026-03-09`

#### Scenario: Complete recurring task via TUI toggle
- **WHEN** the user presses `Enter` or `Space` on a recurring open task in the TUI
- **THEN** the task SHALL be marked done and a new occurrence SHALL be created with the next due date

#### Scenario: Complete non-recurring task
- **WHEN** the user completes a task that has `recurrence: None`
- **THEN** no new task SHALL be created (existing behavior preserved)

#### Scenario: Recurring task with no due date
- **WHEN** a recurring task with no due date is completed
- **THEN** the new occurrence SHALL have a due date calculated from today

#### Scenario: Status message after recurrence creation
- **WHEN** a recurring task is completed and a new occurrence is created
- **THEN** the system SHALL display a status message indicating the new task was created (e.g., "Completed task 5. Next occurrence: task 12, due 2026-03-09")

#### Scenario: Undo does not remove created occurrence
- **WHEN** a recurring task is completed (creating a new occurrence) and then reopened via `undo`
- **THEN** the created occurrence SHALL remain in the task list (it is a separate task)
