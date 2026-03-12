## MODIFIED Requirements

### Requirement: Note slug naming
Note filenames SHALL be derived from the title using a slugification algorithm: lowercase, replace spaces with hyphens, remove non-alphanumeric characters (except hyphens), collapse consecutive hyphens. The file SHALL be named `<slug>.md`. When a specific slug is provided programmatically (e.g., `slack-digest-2026-03-05`), the system SHALL use that slug directly instead of deriving it from the title.

#### Scenario: Title to slug conversion
- **WHEN** a note is created with title "My Project Plan!"
- **THEN** the filename SHALL be `my-project-plan.md`

#### Scenario: Slug collision avoidance
- **WHEN** a note is created with a title that produces a slug matching an existing note file
- **THEN** the system SHALL append a numeric suffix (e.g., `my-note-2.md`) to produce a unique filename

#### Scenario: Programmatic slug override
- **WHEN** a note is created with title "Slack Digest 2026-03-05" and an explicit slug `slack-digest-2026-03-05`
- **THEN** the filename SHALL be `slack-digest-2026-03-05.md` using the provided slug directly
