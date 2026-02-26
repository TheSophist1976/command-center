## MODIFIED Requirements

### Requirement: Mode-based input handling
The TUI SHALL operate in distinct modes: Normal, Adding, Filtering, Confirming, EditingPriority, EditingTitle, EditingTags, EditingDescription, EditingDefaultDir, NlpInput, and ConfirmingNlp. Keyboard input SHALL be interpreted according to the current mode. Only Normal mode SHALL process navigation and action keys. The `:` key in Normal mode SHALL enter NLP input mode. The `i` key in Normal mode SHALL trigger a Todoist import (handled outside the mode system via the status message pattern).

#### Scenario: Input in add mode
- **WHEN** the TUI is in Adding mode and the user presses `j`
- **THEN** the character `j` SHALL be appended to the input buffer (not interpreted as navigation)

#### Scenario: Escape returns to normal
- **WHEN** the TUI is in any non-Normal mode and the user presses `Esc`
- **THEN** the TUI SHALL return to Normal mode and discard any in-progress input

#### Scenario: Import key in normal mode
- **WHEN** the TUI is in Normal mode and the user presses `i`
- **THEN** the system SHALL initiate a Todoist import as specified in the tui-todoist-import capability

#### Scenario: NLP key in normal mode
- **WHEN** the TUI is in Normal mode and the user presses `:`
- **THEN** the TUI SHALL enter NlpInput mode

### Requirement: Three-region layout
The TUI SHALL render a three-region layout: a header bar (1 line) showing the title and active filter summary, a scrollable task table filling the remaining space, and a footer bar (1 line) showing context-sensitive keybinding hints.

#### Scenario: Default layout rendering
- **WHEN** the TUI is displayed with tasks loaded
- **THEN** the header SHALL show "task-manager" and the footer SHALL show keybinding hints including `j/k:nav  Enter:toggle  a:add  d:delete  f:filter  p:priority  e:edit  t:tags  r:desc  v:view  i:import  ::command  D:set-dir  q:quit`

#### Scenario: Filter active in header
- **WHEN** a filter is active (e.g., status:open)
- **THEN** the header SHALL display the active filter expression alongside the title

#### Scenario: Status message in footer
- **WHEN** a status message is set and the TUI is in Normal mode
- **THEN** the footer SHALL display the status message text instead of keybinding hints

#### Scenario: NLP input mode footer
- **WHEN** the TUI is in NlpInput mode
- **THEN** the footer SHALL display ` > {input}_ ` with the user's typed text

#### Scenario: NLP confirmation mode footer
- **WHEN** the TUI is in ConfirmingNlp mode with a pending bulk update
- **THEN** the footer SHALL display the action description and "y/n" prompt
