## Why

The Slack inbox currently shows messages as a flat table with 60-character snippets, making it hard to read longer messages without opening them in Slack. The reply experience is minimal — a single-line input in the status bar with no visible context of what you're replying to. These limitations push users back to the Slack app for basic reading and replying, reducing the value of the integration.

## What Changes

- Add a message preview pane below the inbox table that shows the full text of the selected message, including sender, channel, and timestamp
- Replace the status-bar reply input with a dedicated reply panel that shows the original message above a multi-line text input area
- Add word-wrapping for long messages in the preview pane
- Add `p` keybinding to toggle the preview pane on/off (default: on)
- Support cursor movement within the reply input (left/right arrows, Home/End)

## Capabilities

### New Capabilities
- `slack-message-preview`: A split-pane view in the Slack inbox that displays the full content of the currently selected message below the message list, with word-wrapping and metadata (sender, channel, time)
- `slack-reply-panel`: An improved reply composition experience with the original message shown as context above a multi-line text input that supports basic cursor movement and word-wrapping

### Modified Capabilities
- `slack-inbox`: Update the inbox layout to support the split-pane view with a toggleable preview pane, and update keybinding footer to reflect new controls

## Impact

- **Modified files**: `src/tui.rs` (layout changes for split pane, new draw functions, updated key handling for reply panel and preview toggle)
- **No external dependencies**: All changes are TUI-internal, no new API calls or crate dependencies
- **No breaking changes**: Existing keybindings remain unchanged; new features are additive
