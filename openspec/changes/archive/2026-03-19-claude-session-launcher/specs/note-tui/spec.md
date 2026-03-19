## ADDED Requirements

### Requirement: Session launch keybinding in Notes view
The user SHALL press `C` in the Notes view (with a note selected) to open the directory picker and initiate a Claude session with the selected note's title and body as context. The footer in the Notes view SHALL include `C:claude` in its keybinding hints.

#### Scenario: Launch session from Notes view
- **WHEN** the user presses `C` with a note selected in the Notes view
- **THEN** the TUI SHALL enter Mode::SessionDirectoryPicker with the selected note's title and body queued as session context

#### Scenario: Footer hint in Notes view
- **WHEN** the Notes view is active
- **THEN** the footer SHALL display `a:new  Enter:edit  d:delete  v:view  C:claude  q:quit`
