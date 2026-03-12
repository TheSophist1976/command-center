## ADDED Requirements

### Requirement: Reply panel layout
When the user enters `SlackReplying` mode, the preview pane area SHALL transform into a reply panel. The reply panel SHALL display the original message (quoted) in the upper portion and a text input area in the lower portion. The message table SHALL remain visible above the reply panel.

#### Scenario: Reply panel replaces preview pane
- **WHEN** the user presses `r` on a selected message
- **THEN** the preview pane area SHALL show the original message text prefixed with `>` as a quote block, followed by a bordered text input area labeled "Reply to #channel-name:"

#### Scenario: Original message context visible
- **WHEN** the reply panel is active for a message from `Alice` in `#general` saying "Can you review the deploy script?"
- **THEN** the upper portion SHALL display `Alice · #general` and the quoted message text

### Requirement: Reply text input with cursor
The reply input area SHALL support character-by-character input with a visible cursor position. The cursor SHALL be displayed as a highlighted character or underscore at the current position.

#### Scenario: Type characters
- **WHEN** the user types "Sure, I'll take a look"
- **THEN** the input area SHALL display the text with the cursor at the end

#### Scenario: Cursor visibility
- **WHEN** the reply input is active
- **THEN** a cursor indicator SHALL be visible at the current insertion point

### Requirement: Reply cursor movement
The reply input SHALL support cursor movement using Left/Right arrow keys and Home/End keys. The cursor SHALL move by character boundaries (respecting multi-byte characters).

#### Scenario: Move cursor left
- **WHEN** the user presses Left arrow with cursor at position 5
- **THEN** the cursor SHALL move to position 4 (the previous character boundary)

#### Scenario: Move cursor right
- **WHEN** the user presses Right arrow with cursor not at the end of the text
- **THEN** the cursor SHALL move to the next character boundary

#### Scenario: Home key
- **WHEN** the user presses Home
- **THEN** the cursor SHALL move to the beginning of the input (position 0)

#### Scenario: End key
- **WHEN** the user presses End
- **THEN** the cursor SHALL move to the end of the input text

#### Scenario: Insert at cursor position
- **WHEN** the user types a character with the cursor at position 3 of "hello"
- **THEN** the character SHALL be inserted at position 3 and the cursor SHALL advance by one

#### Scenario: Backspace at cursor position
- **WHEN** the user presses Backspace with the cursor at position 3 of "hello"
- **THEN** the character at position 2 SHALL be deleted and the cursor SHALL move to position 2

### Requirement: Reply input word wrapping
The reply input text SHALL wrap visually within the input area using `Paragraph` with `Wrap { trim: false }`. The text SHALL be sent as a single message regardless of visual line breaks.

#### Scenario: Long reply wraps visually
- **WHEN** the user types a reply longer than the input area width
- **THEN** the text SHALL wrap at word boundaries within the input area

### Requirement: Reply send and cancel
The user SHALL press `Enter` to send the reply and `Esc` to cancel. Sending SHALL post the message via `chat.postMessage` and return to `SlackInbox` mode. Canceling SHALL discard the input and return to `SlackInbox` mode. Empty replies SHALL be rejected.

#### Scenario: Send reply
- **WHEN** the user presses Enter with non-empty input text
- **THEN** the system SHALL send the message via `chat.postMessage`, display "Reply sent" in the status bar, clear the input, and return to SlackInbox mode

#### Scenario: Cancel reply
- **WHEN** the user presses Esc during reply composition
- **THEN** the input SHALL be discarded and the TUI SHALL return to SlackInbox mode with the preview pane restored

#### Scenario: Empty reply rejected
- **WHEN** the user presses Enter with an empty or whitespace-only input
- **THEN** no message SHALL be sent and the reply panel SHALL remain active
