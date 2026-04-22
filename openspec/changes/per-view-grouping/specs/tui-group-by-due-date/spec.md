## ADDED Requirements

### Requirement: Group by due date
The TUI SHALL support `GroupBy::DueDate` as a grouping option. When active, tasks SHALL be grouped by their due date value formatted as `YYYY-MM-DD`. Tasks with no due date SHALL be grouped under a `"No Due Date"` section placed after all dated sections. Dated sections SHALL be sorted chronologically (ascending). The `G` key cycle SHALL include `DueDate` in its sequence: `none → project → agent → priority → due-date → none`.

#### Scenario: Tasks grouped by due date
- **WHEN** `GroupBy::DueDate` is active and tasks have due dates `2026-04-10`, `2026-04-12`, and one with no due date
- **THEN** the table SHALL render a section `Due Date : 2026-04-10`, then `Due Date : 2026-04-12`, then `Due Date : No Due Date`

#### Scenario: Dated sections sorted chronologically
- **WHEN** `GroupBy::DueDate` is active and due date groups include `2026-05-01` and `2026-04-15`
- **THEN** `2026-04-15` SHALL appear before `2026-05-01`

#### Scenario: No-due-date section appears last
- **WHEN** `GroupBy::DueDate` is active and some tasks have no due date
- **THEN** those tasks SHALL appear under `Due Date : No Due Date` after all dated sections

#### Scenario: G key includes due-date in cycle
- **WHEN** the user presses `G` repeatedly starting from `GroupBy::Priority`
- **THEN** the next grouping SHALL be `GroupBy::DueDate`, and the next after that SHALL be `GroupBy::None`
