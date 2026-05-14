## ADDED Requirements

### Requirement: Open note in external editor via `$EDITOR`
When a note is opened for editing and `$EDITOR` is set, the TUI SHALL suspend (disable raw mode, leave alternate screen), spawn `$EDITOR <note-file-path>` as a child process, wait for it to exit, then resume the TUI (enter alternate screen, enable raw mode, redraw). If `$EDITOR` is not set, the system SHALL fall back to checking for Obsidian configuration. If neither is configured, the system SHALL display an error status message.

#### Scenario: Open note with `$EDITOR` set
- **WHEN** the user opens a note and `$EDITOR` is set to `vim`
- **THEN** the TUI SHALL suspend, `vim <note-path>` SHALL run in the foreground, and the TUI SHALL resume after vim exits

#### Scenario: `$EDITOR` not set, Obsidian not configured
- **WHEN** the user opens a note and `$EDITOR` is unset and `obsidian-vault` is not in config
- **THEN** the TUI SHALL display the status message `"No editor configured. Set $EDITOR or add obsidian-vault to config."`

#### Scenario: TUI resumes after editor exits
- **WHEN** the editor process exits (any exit code)
- **THEN** the TUI SHALL re-enter raw mode, re-enter the alternate screen, and redraw the full interface

### Requirement: Open note in Obsidian via URI
When `obsidian-vault` is set in config, opening a note SHALL launch `open "obsidian://open?vault=<vault>&file=<file>"` (macOS). If `obsidian-notes-dir` is also set, the file path SHALL be `<obsidian-notes-dir>/<slug>`. Otherwise the file path SHALL be just `<slug>`. The Obsidian launch SHALL NOT suspend the TUI — it runs asynchronously in the background.

#### Scenario: Open note with Obsidian configured
- **WHEN** `obsidian-vault: MyVault` is in config and the user opens a note with slug `meeting-notes`
- **THEN** the system SHALL run `open "obsidian://open?vault=MyVault&file=meeting-notes"` without suspending the TUI

#### Scenario: Open note with Obsidian and notes dir configured
- **WHEN** `obsidian-vault: MyVault` and `obsidian-notes-dir: Tasks/Notes` are in config and slug is `meeting-notes`
- **THEN** the system SHALL run `open "obsidian://open?vault=MyVault&file=Tasks/Notes/meeting-notes"`

#### Scenario: Obsidian takes priority over `$EDITOR`
- **WHEN** both `obsidian-vault` is configured and `$EDITOR` is set
- **THEN** the system SHALL use Obsidian (not `$EDITOR`)

### Requirement: View note with glow
When `note-viewer: glow` is set in config, the TUI SHALL provide a separate view action that suspends the TUI, runs `glow <note-path>`, waits for it to exit, and resumes the TUI. This is distinct from the edit action.

#### Scenario: View note with glow
- **WHEN** `note-viewer: glow` is in config and the user triggers the view action
- **THEN** the TUI SHALL suspend, `glow <note-path>` SHALL run in the foreground, and the TUI SHALL resume after glow exits

#### Scenario: View action without glow configured
- **WHEN** `note-viewer` is not set and the user triggers the view action
- **THEN** the TUI SHALL fall through to the edit action (open in `$EDITOR` or Obsidian)
