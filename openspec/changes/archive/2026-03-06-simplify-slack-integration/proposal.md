## Why

The current Slack integration pipes messages through Claude's API to extract "action items," which adds latency, cost, and complexity. Most of the time I just need to see my unread messages, quickly respond or dismiss them, and link back to the original Slack thread. The NLP analysis layer is overkill for this workflow. Simplifying to a direct message reader with markdown-file storage makes the feature faster, cheaper, and easier to maintain.

## What Changes

- **BREAKING**: Remove NLP/AI analysis of Slack messages (`analyze_slack_messages`, `SlackSuggestion`, Claude API call). No longer requires a Claude API key for Slack features.
- **BREAKING**: Remove the "suggestion review" flow (`SlackReview`/`SlackReviewEditing` modes) that converts AI suggestions into tasks.
- Add a new **Slack inbox** TUI view that displays unread messages grouped by channel, showing sender, timestamp, and message text.
- Each message includes a deep link back to the original Slack conversation (using workspace URL + channel ID + message ts).
- Add ability to **reply to a message** directly from the TUI (via `chat.postMessage` API).
- Add ability to **mark messages as done/completed** so they no longer appear in the inbox.
- Use **markdown files** as the storage mechanism for Slack message state (inbox items, completed items) instead of the current JSON high-water-mark approach.
- Retain existing Slack API client code (channel fetching, message fetching, user resolution) — only the processing/display layer changes.

## Capabilities

### New Capabilities
- `slack-inbox`: TUI view for browsing unread Slack messages with links, reply, and mark-done actions
- `slack-message-storage`: Markdown-file based storage for Slack message state (inbox + completed tracking)
- `slack-reply`: Send replies to Slack messages from within the TUI

### Modified Capabilities
- `tui`: New Slack inbox mode replacing the old SlackReview/SlackReviewEditing/SlackChannelPicker modes

## Impact

- **Code removed**: `SlackSuggestion`, `RawSuggestion`, `build_slack_analysis_prompt`, `analyze_slack_messages`, `parse_slack_suggestions` from `src/slack.rs`. `SlackReview`/`SlackReviewEditing` mode handlers and `accept_slack_suggestion` from `src/tui.rs`. Related tests.
- **Code modified**: `src/tui.rs` (new inbox mode), `src/slack.rs` (add reply API, remove NLP), `src/main.rs`/`src/cli.rs` (remove Claude key requirement for Slack)
- **New storage**: Markdown files for Slack inbox state (e.g., `~/.config/task-manager/slack/inbox.md`, per-channel or unified)
- **Dependencies**: No new crates needed. Can remove the Claude API dependency from the Slack flow path.
- **Config**: `slack-channels` config remains. HWM JSON state file migrates to or is replaced by MD-based tracking.
