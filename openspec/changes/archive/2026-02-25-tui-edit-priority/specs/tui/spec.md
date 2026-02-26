## ADDED Requirements

### Requirement: Edit task priority
The user SHALL press `p` in normal mode to enter priority-editing mode for the selected task. The footer SHALL display a picker prompt showing the three available priorities. The user SHALL press `h`, `m`, or `l` to set the priority to high, medium, or low respectively. The change SHALL be persisted to disk immediately. Pressing `Esc` or any other key SHALL cancel without changing the task. The `p` key SHALL be a no-op when no task is selected.

#### Scenario: Set priority to high
- **WHEN** the user presses `p` on a selected task and then presses `h`
- **THEN** the task's priority SHALL be set to high, the display SHALL update, and the file SHALL be saved

#### Scenario: Set priority to medium
- **WHEN** the user presses `p` on a selected task and then presses `m`
- **THEN** the task's priority SHALL be set to medium, the display SHALL update, and the file SHALL be saved

#### Scenario: Set priority to low
- **WHEN** the user presses `p` on a selected task and then presses `l`
- **THEN** the task's priority SHALL be set to low, the display SHALL update, and the file SHALL be saved

#### Scenario: Cancel priority edit
- **WHEN** the user presses `p` and then presses `Esc` or any key other than `h`, `m`, or `l`
- **THEN** the task's priority SHALL remain unchanged and the TUI SHALL return to normal mode

#### Scenario: No-op when list is empty
- **WHEN** the user presses `p` and no task is selected (empty or fully filtered list)
- **THEN** the TUI SHALL remain in normal mode with no change

### Requirement: Edit task title
The user SHALL press `e` in normal mode to enter title-editing mode for the selected task. The footer SHALL display a text input pre-populated with the task's current title. The user SHALL edit the text and press `Enter` to confirm or `Esc` to cancel. A confirmed empty title SHALL be rejected and the TUI SHALL remain in editing mode. The change SHALL be persisted to disk immediately on confirmation. The `e` key SHALL be a no-op when no task is selected.

#### Scenario: Edit title to new value
- **WHEN** the user presses `e`, modifies the pre-populated title text, and presses `Enter`
- **THEN** the task's title SHALL be updated to the new value, the display SHALL update, and the file SHALL be saved

#### Scenario: Cancel title edit
- **WHEN** the user presses `e` and then presses `Esc`
- **THEN** the task's title SHALL remain unchanged and the TUI SHALL return to normal mode

#### Scenario: Reject empty title
- **WHEN** the user presses `e`, clears the input buffer, and presses `Enter`
- **THEN** the title SHALL NOT be updated and the TUI SHALL remain in title-editing mode

#### Scenario: No-op when list is empty
- **WHEN** the user presses `e` and no task is selected
- **THEN** the TUI SHALL remain in normal mode with no change

### Requirement: Edit task tags
The user SHALL press `t` in normal mode to enter tag-editing mode for the selected task. The footer SHALL display a text input pre-populated with the task's current tags as a space-separated string. The user SHALL edit the text and press `Enter` to confirm or `Esc` to cancel. An empty confirmed input SHALL clear all tags. The change SHALL be persisted to disk immediately on confirmation. The `t` key SHALL be a no-op when no task is selected.

#### Scenario: Edit tags to new values
- **WHEN** the user presses `t`, modifies the pre-populated tag string, and presses `Enter`
- **THEN** the task's tags SHALL be updated to the whitespace-split tokens of the new input, the display SHALL update, and the file SHALL be saved

#### Scenario: Clear all tags
- **WHEN** the user presses `t`, clears the input buffer, and presses `Enter`
- **THEN** the task's tags SHALL be set to an empty list and the file SHALL be saved

#### Scenario: Cancel tag edit
- **WHEN** the user presses `t` and then presses `Esc`
- **THEN** the task's tags SHALL remain unchanged and the TUI SHALL return to normal mode

#### Scenario: No-op when list is empty
- **WHEN** the user presses `t` and no task is selected
- **THEN** the TUI SHALL remain in normal mode with no change

### Requirement: Edit task description
The user SHALL press `r` in normal mode to enter description-editing mode for the selected task. The footer SHALL display a text input pre-populated with the task's current description (empty string if none). The user SHALL edit the text and press `Enter` to confirm or `Esc` to cancel. A confirmed non-empty value SHALL set the description; a confirmed empty value SHALL clear it (set to none). The change SHALL be persisted to disk immediately on confirmation. The `r` key SHALL be a no-op when no task is selected.

#### Scenario: Set description
- **WHEN** the user presses `r`, types a description, and presses `Enter`
- **THEN** the task's description SHALL be set to the entered text, and the file SHALL be saved

#### Scenario: Clear description
- **WHEN** the user presses `r`, clears the input buffer, and presses `Enter`
- **THEN** the task's description SHALL be set to none and the file SHALL be saved

#### Scenario: Cancel description edit
- **WHEN** the user presses `r` and then presses `Esc`
- **THEN** the task's description SHALL remain unchanged and the TUI SHALL return to normal mode

#### Scenario: No-op when list is empty
- **WHEN** the user presses `r` and no task is selected
- **THEN** the TUI SHALL remain in normal mode with no change

## MODIFIED Requirements

### Requirement: Three-region layout
The TUI SHALL render a three-region layout: a header bar (1 line) showing the title and active filter summary, a scrollable task table filling the remaining space, and a footer bar (1 line) showing context-sensitive keybinding hints.

#### Scenario: Default layout rendering
- **WHEN** the TUI is displayed with tasks loaded
- **THEN** the header SHALL show "task-manager" and the footer SHALL show keybinding hints including `j/k:nav  Enter:toggle  a:add  d:delete  f:filter  p:priority  e:edit  t:tags  r:desc  q:quit`

#### Scenario: Filter active in header
- **WHEN** a filter is active (e.g., status:open)
- **THEN** the header SHALL display the active filter expression alongside the title

### Requirement: Mode-based input handling
The TUI SHALL operate in distinct modes: Normal, Adding, Filtering, Confirming, EditingPriority, EditingTitle, EditingTags, and EditingDescription. Keyboard input SHALL be interpreted according to the current mode. Only Normal mode SHALL process navigation and action keys.

#### Scenario: Input in add mode
- **WHEN** the TUI is in Adding mode and the user presses `j`
- **THEN** the character `j` SHALL be appended to the input buffer (not interpreted as navigation)

#### Scenario: Escape returns to normal
- **WHEN** the TUI is in any non-Normal mode and the user presses `Esc`
- **THEN** the TUI SHALL return to Normal mode and discard any in-progress input
