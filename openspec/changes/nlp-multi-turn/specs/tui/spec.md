## MODIFIED Requirements

### Requirement: Mode-based input handling
The TUI SHALL operate in distinct modes: Normal, Adding, Filtering, Confirming, EditingPriority, EditingTitle, EditingTags, EditingDescription, EditingDefaultDir, NlpChat, and ConfirmingNlp. Keyboard input SHALL be interpreted according to the current mode. Only Normal mode SHALL process navigation and action keys. The `i` key in Normal mode SHALL trigger a Todoist import (handled outside the mode system via the status message pattern).

#### Scenario: Input in add mode
- **WHEN** the TUI is in Adding mode and the user presses `j`
- **THEN** the character `j` SHALL be appended to the input buffer (not interpreted as navigation)

#### Scenario: Escape returns to normal
- **WHEN** the TUI is in any non-Normal mode and the user presses `Esc`
- **THEN** the TUI SHALL return to Normal mode and discard any in-progress input

#### Scenario: Import key in normal mode
- **WHEN** the TUI is in Normal mode and the user presses `i`
- **THEN** the system SHALL initiate a Todoist import as specified in the tui-todoist-import capability

#### Scenario: Colon key enters NlpChat mode
- **WHEN** the TUI is in Normal mode and the user presses `:`
- **THEN** the TUI SHALL enter NlpChat mode with an empty conversation history and display the split layout

### Requirement: Three-region layout
The TUI SHALL render a three-region layout in Normal mode: a header bar (1 line) showing the title and active filter summary, a scrollable task table filling the remaining space, and a footer bar (1 line) showing context-sensitive keybinding hints. In NlpChat mode, the TUI SHALL render a four-region layout: header (1 line), task table (top ~60%), chat panel (bottom ~40%), and input prompt (1 line).

#### Scenario: Default layout rendering
- **WHEN** the TUI is displayed with tasks loaded in Normal mode
- **THEN** the header SHALL show "task-manager" and the footer SHALL show keybinding hints including `j/k:nav  Enter:toggle  a:add  d:delete  f:filter  p:priority  e:edit  t:tags  r:desc  v:view  i:import  ::command  D:set-dir  q:quit`

#### Scenario: Filter active in header
- **WHEN** a filter is active (e.g., status:open)
- **THEN** the header SHALL display the active filter expression alongside the title

#### Scenario: Status message in footer
- **WHEN** a status message is set and the TUI is in Normal mode
- **THEN** the footer SHALL display the status message text instead of keybinding hints

#### Scenario: NlpChat split layout
- **WHEN** the TUI is in NlpChat mode
- **THEN** the layout SHALL split into header, task table (top portion), chat panel (bottom portion), and an input prompt line at the bottom

### Requirement: NLP message responses
When the NLP model determines that the user's query is unclear, conversational, does not map to a filter, update, or show_tasks action, or is a question about the user's tasks, the system SHALL return a `Message(String)` action containing the model's plain-text response. The model SHALL use the task context (all fields: id, title, status, priority, tags, due_date, project) to answer task queries. The TUI SHALL display this message text in the chat panel and remain in NlpChat mode.

#### Scenario: Ambiguous query returns message
- **WHEN** the user enters NLP mode and types an ambiguous query like "hello"
- **THEN** the NLP module SHALL return `NlpAction::Message` with a helpful text response, and the TUI SHALL display that text in the chat panel

#### Scenario: Task count query
- **WHEN** the user enters NLP mode and types "how many high-priority tasks do I have?"
- **THEN** the NLP module SHALL return `NlpAction::Message` with the count derived from the task context

#### Scenario: Task field query
- **WHEN** the user enters NLP mode and types "what projects do I have tasks in?"
- **THEN** the NLP module SHALL return `NlpAction::Message` listing the distinct project names from the task data

#### Scenario: Task summary query
- **WHEN** the user enters NLP mode and types "what's my oldest open task?"
- **THEN** the NLP module SHALL return `NlpAction::Message` with the answer based on task creation dates and status

#### Scenario: Unsupported action returns message
- **WHEN** the user enters NLP mode and requests an action the system cannot perform (e.g., "email my tasks to Alice")
- **THEN** the NLP module SHALL return `NlpAction::Message` explaining that the action is not supported

#### Scenario: Message display stays in NlpChat mode
- **WHEN** the TUI receives an `NlpAction::Message` response while in NlpChat mode
- **THEN** the TUI SHALL append the message to the chat panel, clear the input buffer, and remain in NlpChat mode for follow-up input

## ADDED Requirements

### Requirement: Chat panel display
The TUI SHALL render a chat panel in NlpChat mode that displays the conversation history. User messages SHALL be prefixed with `> ` and visually distinguished from assistant messages. The chat panel SHALL auto-scroll to the most recent message when new messages are added.

#### Scenario: User message displayed
- **WHEN** the user submits a query in NlpChat mode
- **THEN** the chat panel SHALL display the user's message prefixed with `> `

#### Scenario: Assistant message displayed
- **WHEN** the model returns a Message or ShowTasks response
- **THEN** the chat panel SHALL display the assistant's text response below the user's message

#### Scenario: Auto-scroll on new message
- **WHEN** a new message is added to the conversation and the chat panel content exceeds the visible area
- **THEN** the chat panel SHALL scroll to show the most recent message

### Requirement: ShowTasks display in chat panel
When the model returns a `ShowTasks` action, the TUI SHALL display the accompanying text message followed by a compact task list in the chat panel. Each task SHALL be rendered with its ID, title, status, and priority. Task IDs that do not exist in the current task list SHALL be silently skipped.

#### Scenario: ShowTasks renders task list
- **WHEN** the model returns `ShowTasks` with task IDs `[1, 3, 7]` and text "Here are your overdue tasks:"
- **THEN** the chat panel SHALL display the text followed by a list showing each task's ID, title, status, and priority

#### Scenario: ShowTasks with invalid task IDs
- **WHEN** the model returns `ShowTasks` with task IDs `[1, 999]` and task 999 does not exist
- **THEN** the chat panel SHALL display task 1's details and silently skip task 999

#### Scenario: ShowTasks does not modify table filter
- **WHEN** the model returns a `ShowTasks` action
- **THEN** the main task table filter SHALL NOT be modified and the TUI SHALL remain in NlpChat mode

### Requirement: NlpChat conversation lifecycle
The conversation state SHALL be initialized when entering NlpChat mode and cleared when exiting. Pressing `Esc` in NlpChat mode SHALL clear the conversation history, restore the standard three-region layout, and return to Normal mode.

#### Scenario: Enter NlpChat mode
- **WHEN** the user presses `:` in Normal mode
- **THEN** the TUI SHALL enter NlpChat mode with empty conversation history and display the split layout with input prompt

#### Scenario: Stay in NlpChat after response
- **WHEN** the model returns any NlpAction (Filter, Update confirmation, Message, or ShowTasks)
- **THEN** the TUI SHALL remain in NlpChat mode with the input buffer cleared, ready for follow-up input

#### Scenario: Exit NlpChat mode
- **WHEN** the user presses `Esc` in NlpChat mode
- **THEN** the TUI SHALL clear conversation history, restore the three-region layout, and return to Normal mode

#### Scenario: NlpChat update confirmation
- **WHEN** the model returns an Update action while in NlpChat mode
- **THEN** the TUI SHALL enter ConfirmingNlp mode. After confirmation or cancellation, the TUI SHALL return to NlpChat mode (not Normal mode)
