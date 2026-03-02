## ADDED Requirements

### Requirement: Detail panel inline editing
The TUI SHALL support inline editing of task fields within the detail panel. When the detail panel is visible and the user presses `Enter` on the selected task, the TUI SHALL enter `EditingDetailPanel` mode. The editable fields SHALL be: Title, Description, Priority, Status, Due Date, Project, and Tags. The user SHALL navigate between fields using `j`/`k` or `Tab`/`Shift-Tab`. Pressing `Esc` SHALL exit editing mode (with a dirty check if changes were made).

#### Scenario: Enter detail editing mode
- **WHEN** the detail panel is visible and the user presses `Enter` on a selected task
- **THEN** the TUI SHALL enter `EditingDetailPanel` mode, populate a draft from the current task, focus the first field (Title), and display the panel in edit layout with the focused field highlighted

#### Scenario: Navigate between fields
- **WHEN** the user is in `EditingDetailPanel` mode and presses `j`, `Down`, or `Tab`
- **THEN** the focus SHALL move to the next editable field (wrapping from Tags back to Title), saving the current input buffer value to the draft before moving

#### Scenario: Navigate fields backward
- **WHEN** the user is in `EditingDetailPanel` mode and presses `k`, `Up`, or `Shift-Tab`
- **THEN** the focus SHALL move to the previous editable field (wrapping from Title to Tags), saving the current input buffer value to the draft before moving

#### Scenario: Edit a text field
- **WHEN** a text field (Title, Description, Due Date, Project, Tags) is focused in `EditingDetailPanel` mode
- **THEN** the input buffer SHALL be loaded with the field's draft value, typing SHALL modify the buffer, and the panel SHALL render the current buffer with a cursor indicator

#### Scenario: Edit priority field
- **WHEN** the Priority field is focused and the user presses `c`, `h`, `m`, or `l`
- **THEN** the draft priority SHALL be set to critical, high, medium, or low respectively, and the display SHALL update immediately

#### Scenario: Toggle status field
- **WHEN** the Status field is focused and the user presses `Enter` or `Space`
- **THEN** the draft status SHALL toggle between open and done, and the display SHALL update immediately

#### Scenario: Exit editing with no changes
- **WHEN** the user presses `Esc` in `EditingDetailPanel` mode and the draft has no changes
- **THEN** the TUI SHALL exit to Normal mode immediately with no prompt

#### Scenario: Edit panel rendering
- **WHEN** the TUI is in `EditingDetailPanel` mode
- **THEN** the detail panel SHALL render each editable field on its own line with a label, the focused field SHALL be visually highlighted, and the footer SHALL show editing hints

### Requirement: Save-on-navigate confirmation
When the user attempts to leave the detail editing context with unsaved changes, the TUI SHALL prompt the user to save, discard, or cancel. This applies when pressing `Esc` to exit editing or when navigating to a different task while the draft is dirty.

#### Scenario: Dirty exit triggers confirmation
- **WHEN** the user presses `Esc` in `EditingDetailPanel` mode and the draft differs from the original task
- **THEN** the TUI SHALL enter `ConfirmingDetailSave` mode and display a footer prompt: "Unsaved changes. [s]ave  [d]iscard  [c]ancel"

#### Scenario: Save and exit
- **WHEN** the user presses `s` in `ConfirmingDetailSave` mode
- **THEN** the draft SHALL be applied to the task, the `updated` timestamp SHALL be set, the file SHALL be saved to disk, and the TUI SHALL exit to Normal mode

#### Scenario: Discard and exit
- **WHEN** the user presses `d` in `ConfirmingDetailSave` mode
- **THEN** the draft SHALL be discarded and the TUI SHALL exit to Normal mode with no changes persisted

#### Scenario: Cancel confirmation
- **WHEN** the user presses `c` or `Esc` in `ConfirmingDetailSave` mode
- **THEN** the TUI SHALL return to `EditingDetailPanel` mode with the draft intact

#### Scenario: Navigate away with dirty draft
- **WHEN** the detail panel is open with a dirty draft and the user presses `j` or `k` in Normal mode to navigate to a different task
- **THEN** the TUI SHALL enter `ConfirmingDetailSave` mode, storing the intended navigation direction. After save or discard, the navigation SHALL proceed. After cancel, the selection SHALL remain unchanged.

#### Scenario: Invalid due date on save
- **WHEN** the user saves and the due date field contains an invalid date string (not YYYY-MM-DD format and not empty)
- **THEN** the TUI SHALL display a status message indicating the invalid date, focus the due date field, and remain in `EditingDetailPanel` mode

## MODIFIED Requirements

### Requirement: Three-region layout
The TUI SHALL render a three-region layout in Normal mode: a header bar (1 line) showing the title and active filter summary, a scrollable task table filling the remaining space, and a footer bar (1 line) showing context-sensitive keybinding hints. In NlpChat mode, the TUI SHALL render a four-region layout: header (1 line), task table (top ~60%), chat panel (bottom ~40%), and input prompt (1 line).

#### Scenario: Default layout rendering
- **WHEN** the TUI is displayed with tasks loaded in Normal mode
- **THEN** the header SHALL show "task-manager" and the footer SHALL show keybinding hints including `j/k:nav  Enter:toggle  a:add  d:delete  f:filter  p:priority  e:edit  t:tags  r:desc  v:view  i:import  ::command  D:set-dir  T/W/M/Q:due  X:clr-due  Tab:details  q:quit`

#### Scenario: Footer hints with detail panel visible
- **WHEN** the detail panel is visible in Normal mode
- **THEN** the footer SHALL show `Enter:edit` instead of `Enter:toggle` to indicate that Enter enters detail editing mode

#### Scenario: Footer hints in detail editing mode
- **WHEN** the TUI is in `EditingDetailPanel` mode
- **THEN** the footer SHALL show hints for editing: `j/k:field  Enter:save  Esc:cancel` (or equivalent context-sensitive hints)

### Requirement: Toggle task completion
The user SHALL toggle a task between open and done by pressing `Enter` or `Space` on the selected task. The change SHALL be persisted to disk immediately. When the detail panel is visible, `Enter` SHALL enter detail editing mode instead of toggling completion; `Space` SHALL continue to toggle completion.

#### Scenario: Enter with detail panel visible
- **WHEN** the detail panel is visible and the user presses `Enter` on a selected task
- **THEN** the TUI SHALL enter `EditingDetailPanel` mode instead of toggling the task's completion status

#### Scenario: Space with detail panel visible
- **WHEN** the detail panel is visible and the user presses `Space` on a selected task
- **THEN** the task's status SHALL toggle between open and done as usual (not entering edit mode)
