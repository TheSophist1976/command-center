## MODIFIED Requirements

### Requirement: Three-region layout
The TUI SHALL render a three-region layout in Normal mode: a header bar (1 line) showing the title and active filter summary, a scrollable task table filling the remaining space, and a footer bar (1 line) showing context-sensitive keybinding hints. In NlpChat mode, the TUI SHALL render a four-region layout: header (1 line), task table (top ~60%), chat panel (bottom ~40%), and input prompt (1 line).

#### Scenario: Default layout rendering
- **WHEN** the TUI is displayed with tasks loaded in Normal mode
- **THEN** the header SHALL use `theme::BAR_FG` foreground and `theme::BAR_BG` background, and the footer SHALL show keybinding hints with the same theme colors. The header SHALL show "task-manager" and the footer SHALL show keybinding hints including `j/k:nav  Enter:toggle  a:add  d:delete  f:filter  p:priority  e:edit  t:tags  r:desc  v:view  i:import  ::command  D:set-dir  S:slack  T/W/M/Q:due  X:clr-due  R:recur  Tab:details  ^r:reload  q:quit`

#### Scenario: Footer hints with detail panel visible
- **WHEN** the TUI is in Normal mode with the detail panel open
- **THEN** the footer SHALL show `j/k:nav  Enter:edit  s:save  d:discard  c:cancel  Tab:close`

#### Scenario: Footer hints in detail editing mode
- **WHEN** the TUI is in `EditingDetailPanel` mode
- **THEN** the footer SHALL show hints for editing: `j/k:field  Enter:save  Esc:cancel` (or equivalent context-sensitive hints)

#### Scenario: Filter active in header
- **WHEN** a filter is active (e.g., status:open)
- **THEN** the header SHALL display the active filter expression alongside the title

#### Scenario: Status message in footer
- **WHEN** a status message is set and the TUI is in Normal mode
- **THEN** the footer SHALL display the status message text instead of keybinding hints

#### Scenario: NlpChat split layout
- **WHEN** the TUI is in NlpChat mode
- **THEN** the layout SHALL split into header, task table (top portion), chat panel (bottom portion), and an input prompt line at the bottom
