## ADDED Requirements

### Requirement: `note-viewer` config key
The config file MAY contain a `note-viewer` key specifying a read-only note viewer command. The only supported value is `glow`. If set to `glow`, the TUI SHALL offer a glow-based view action for notes. Unrecognised values SHALL be silently ignored.

#### Scenario: glow viewer configured
- **WHEN** config contains `note-viewer: glow`
- **THEN** the TUI SHALL launch `glow <note-path>` when the view action is triggered

#### Scenario: Unknown viewer ignored
- **WHEN** config contains `note-viewer: bat`
- **THEN** the system SHALL ignore the value and treat it as unconfigured

### Requirement: `obsidian-vault` config key
The config file MAY contain an `obsidian-vault` key specifying the name of an Obsidian vault. When present, opening a note SHALL launch the Obsidian URI `obsidian://open?vault=<value>&file=<file>` using the macOS `open` command.

#### Scenario: Obsidian vault configured
- **WHEN** config contains `obsidian-vault: MyVault`
- **THEN** note open SHALL use `open "obsidian://open?vault=MyVault&file=<slug>"`

### Requirement: `obsidian-notes-dir` config key
The config file MAY contain an `obsidian-notes-dir` key specifying the relative path prefix within the Obsidian vault where task notes are stored. When present, this prefix is prepended to the note slug in the Obsidian URI file parameter.

#### Scenario: Notes dir prefix applied
- **WHEN** config contains `obsidian-vault: MyVault` and `obsidian-notes-dir: Tasks/Notes` and slug is `sprint-retro`
- **THEN** the URI SHALL be `obsidian://open?vault=MyVault&file=Tasks/Notes/sprint-retro`

#### Scenario: Notes dir absent uses slug only
- **WHEN** `obsidian-notes-dir` is not set and slug is `sprint-retro`
- **THEN** the URI SHALL be `obsidian://open?vault=MyVault&file=sprint-retro`
