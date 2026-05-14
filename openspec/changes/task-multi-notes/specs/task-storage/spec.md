## MODIFIED Requirements

### Requirement: Task note field
The `Task` struct SHALL include a `notes: Vec<String>` field containing the slugs of all linked note files. The field SHALL be serialized as `notes:<slug1>,<slug2>,...` in the task metadata comment and omitted when the list is empty. The legacy `note:<slug>` key SHALL be parsed as a single-element `notes` list; on re-serialization it will be written as `notes:<slug>`.

#### Scenario: Task with one linked note serialized
- **WHEN** a task has `notes: vec!["meeting-notes"]`
- **THEN** the metadata comment SHALL contain `notes:meeting-notes`

#### Scenario: Task with multiple linked notes serialized
- **WHEN** a task has `notes: vec!["meeting-notes", "how-to-deploy"]`
- **THEN** the metadata comment SHALL contain `notes:meeting-notes,how-to-deploy`

#### Scenario: Task with no linked notes
- **WHEN** a task has `notes: vec![]`
- **THEN** the metadata comment SHALL NOT contain a `notes` key

#### Scenario: Legacy `note:` key parsed as single-element list
- **WHEN** a metadata comment contains `note:meeting-notes` (legacy format)
- **THEN** the parser SHALL set `notes` to `vec!["meeting-notes"]`

#### Scenario: `notes:` key parsed correctly
- **WHEN** a metadata comment contains `notes:slug1,slug2`
- **THEN** the parser SHALL set `notes` to `vec!["slug1", "slug2"]`

#### Scenario: Round-trip preserves all note slugs
- **WHEN** a task with `notes: vec!["a", "b", "c"]` is serialized and parsed
- **THEN** the parsed task SHALL have `notes: vec!["a", "b", "c"]`
