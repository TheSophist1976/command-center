## ADDED Requirements

### Requirement: By Agent view rendering
When `View::ByAgent` is active, the TUI SHALL display all open tasks grouped under agent section headers. Each unique `agent` value in the filtered task list SHALL produce one section header row. Tasks with no `agent` field SHALL be grouped under an "Unassigned" section placed after all named agent sections. Agent sections SHALL be sorted alphabetically by agent name. Within each section, tasks SHALL be sorted by the same due-date/priority order used in other views.

#### Scenario: Tasks grouped under agent headers
- **WHEN** the By Agent view is active and tasks have agents "command-center" and "human"
- **THEN** the task table SHALL render a header row for "command-center" followed by its tasks, then a header row for "human" followed by its tasks

#### Scenario: Unassigned tasks grouped last
- **WHEN** the By Agent view is active and some tasks have no `agent` field
- **THEN** those tasks SHALL appear under an "Unassigned" section after all named agent sections

#### Scenario: Agent sections sorted alphabetically
- **WHEN** the By Agent view is active and agents are "human", "AI-Assisted", and "command-center"
- **THEN** the sections SHALL appear in order: "AI-Assisted", "command-center", "human"

#### Scenario: Tasks within section sorted by due date then priority
- **WHEN** two tasks in the same agent section have different due dates
- **THEN** the task with the earlier due date SHALL appear first (tasks with no due date last), with priority as tiebreaker

#### Scenario: Empty view when no open tasks
- **WHEN** the By Agent view is active and there are no open tasks
- **THEN** the task table SHALL display the standard "No tasks match filter." message

### Requirement: By Agent view section header styling
Section header rows in the By Agent view SHALL be visually distinct from task rows. Each header SHALL display the agent name (or "Unassigned") with bold text and a dimmed/accented background. Header rows SHALL NOT be selectable — navigation with `j`/`k` SHALL skip header rows and land on task rows only.

#### Scenario: Header row is not selectable
- **WHEN** the user presses `j` from the last task in the "command-center" section
- **THEN** the selection SHALL move to the first task in the next section, skipping the section header row

#### Scenario: Header row styled distinctly
- **WHEN** the By Agent view renders a section header
- **THEN** the header row SHALL use bold text and a visually distinct background color different from task rows and the highlight color
