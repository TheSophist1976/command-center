## MODIFIED Requirements

### Requirement: Inbox layout
The SlackInbox mode SHALL render a table with columns: Channel, Sender, Message, and Time. The layout SHALL include a header showing "Slack Inbox -- N messages" and a footer with keybinding hints. When the preview pane is visible, the footer SHALL show `j/k:nav  Enter/d:done  r:reply  o:open  p:preview  t:task  S:sync  Esc:back`. When the preview pane is hidden, the footer SHALL show `j/k:nav  Enter/d:done  r:reply  o:open  p:preview  t:task  S:sync  Esc:back`. During SlackReplying mode, the footer SHALL show `Enter:send  Esc:cancel  ←→:move cursor  Home/End:jump`. During SlackCreatingTask mode, the footer SHALL show `Enter:confirm  Esc:cancel`.

#### Scenario: Inbox layout with task keybinding visible
- **WHEN** the SlackInbox mode is active
- **THEN** the footer SHALL include the `t:task` keybinding hint
