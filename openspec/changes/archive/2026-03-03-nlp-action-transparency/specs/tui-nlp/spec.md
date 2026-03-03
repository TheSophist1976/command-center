<!-- MODIFIED: tui-nlp -->

### ADDED Requirement: Action summary in chat

After parsing an NLP response, the chat panel SHALL display a human-readable summary of the interpreted action before executing it.

#### Scenario: Filter action summary
- **WHEN** the NLP returns a filter action
- **THEN** the chat panel SHALL show a message like `Filtering: status=open, priority=high` listing the non-null filter criteria

#### Scenario: Update action summary
- **WHEN** the NLP returns an update action
- **THEN** the chat panel SHALL show a message like `Updating: match {tag=frontend} → set {priority=high}` describing match criteria and fields to set

#### Scenario: Null fields omitted
- **WHEN** an action has null criteria fields
- **THEN** those fields SHALL NOT appear in the summary

### MODIFIED Requirement: Update confirmation detail

The update confirmation flow SHALL show per-task before→after changes so the user can make an informed approval.

#### Scenario: Task changes listed in chat
- **WHEN** the NLP returns an update action and matching tasks are found
- **THEN** the chat panel SHALL list each affected task with its field changes (e.g., `#3 "Fix bug": priority Medium → High`)

#### Scenario: Only changed fields shown
- **WHEN** a set_field value matches the task's current value
- **THEN** that field SHALL NOT be shown in the change preview for that task

#### Scenario: Large match set truncated
- **WHEN** more than 10 tasks match the update criteria
- **THEN** the chat panel SHALL show the first 10 tasks and a line `... and N more tasks`
