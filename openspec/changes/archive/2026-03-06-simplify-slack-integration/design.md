## Context

The current Slack integration fetches unread messages, sends them to Claude's API for NLP extraction of "action items," then presents suggestions in a review UI where the user accepts/skips/edits each one to create tasks. This requires a Claude API key, adds latency and cost, and the extracted suggestions often miss context.

The app already has: Slack OAuth token storage (`auth.rs`), channel/message fetching with pagination and HWM tracking (`slack.rs`), user name resolution with caching, and a channel picker TUI mode. Storage elsewhere uses markdown files (`tasks.md`, notes).

## Goals / Non-Goals

**Goals:**
- Display unread Slack messages directly in the TUI, grouped by channel
- Provide deep links back to the original Slack conversation
- Allow replying to messages from within the TUI
- Allow marking messages as "done" so they disappear from the inbox
- Use markdown files for storing Slack inbox state (consistent with rest of app)
- Remove NLP/Claude API dependency from the Slack flow

**Non-Goals:**
- Thread support (display/reply to threads) — keep it flat for now
- Real-time message streaming / websocket — still poll-based on user action
- Slack reactions or emoji support
- Rich text / attachment rendering — plain text only
- Migrating existing HWM data — fresh start is fine

## Decisions

### 1. Storage format: Single markdown file per workspace

**Decision:** Store Slack inbox state in `~/.config/task-manager/slack/inbox.md` as a flat markdown file with one section per message.

**Format:**
```markdown
<!-- slack-inbox -->
<!-- workspace: T0123WORKSPACE -->
<!-- last-sync: 2026-03-06T14:30:00Z -->

## [#general] Alice: Can you review the deploy script?
<!-- ts:1709654321.000100 channel:C0123ABC user:U456DEF status:open -->
<!-- link: https://myworkspace.slack.com/archives/C0123ABC/p1709654321000100 -->

## [DM] Bob: Hey, are we still meeting at 3?
<!-- ts:1709654322.000200 channel:D0789XYZ user:U111AAA status:done -->
<!-- link: https://myworkspace.slack.com/archives/D0789XYZ/p1709654322000200 -->
```

**Why over alternatives:**
- *Single file vs per-channel files:* Simpler to load/save, easier to browse outside the app. Message count is bounded by unread window.
- *Markdown vs JSON:* Consistent with tasks.md and notes. Human-readable and editable.
- *Status field in metadata:* `open` or `done`. Done messages stay in file for a configurable retention period (default 7 days), then get pruned on next sync.

### 2. Deep links: Construct from workspace + channel + ts

**Decision:** Build Slack deep links using the format `https://{workspace}.slack.com/archives/{channel_id}/p{ts_without_dot}`.

The workspace name needs to be stored once (during `task auth slack` or first sync). Store as `slack-workspace` in config.md. The message ts `1709654321.000100` becomes `p1709654321000100` (remove the dot).

### 3. Reply mechanism: `chat.postMessage` API

**Decision:** Use `chat.postMessage` to send replies. The reply is posted as a new message in the channel (not threaded). The bot token already has `chat:write` scope from the existing OAuth flow.

**UX:** User presses `r` on a message, enters reply text in an input bar, presses Enter to send. Esc to cancel.

### 4. TUI mode: Replace SlackReview with SlackInbox

**Decision:** Replace the three existing modes (`SlackReview`, `SlackReviewEditing`, `SlackChannelPicker`) with two new modes:
- `SlackInbox` — main message list, grouped by channel
- `SlackReplying` — input mode for composing a reply

The channel picker remains but is simplified — it only needs to set which channels to sync (already stored in config as `slack-channels`).

**Key bindings in SlackInbox:**
- `j/k` or arrows: navigate messages
- `Enter` or `d`: mark message as done (hide from inbox)
- `r`: reply to message
- `o`: open deep link in browser (`open` command on macOS)
- `S`: sync (re-fetch) messages
- `Esc`: return to normal mode

### 5. Sync strategy: HWM-based, on-demand

**Decision:** Keep the existing high-water-mark approach but store HWM in the inbox.md metadata rather than a separate JSON file. On sync:
1. Fetch new messages since last HWM per channel
2. Append new messages to inbox.md with `status:open`
3. Update HWM per channel
4. Prune messages with `status:done` older than 7 days

Sync happens when user enters SlackInbox mode or presses `S`.

### 6. Remove NLP code

**Decision:** Delete from `slack.rs`: `SlackSuggestion`, `RawSuggestion`, `build_slack_analysis_prompt`, `analyze_slack_messages`, `parse_slack_suggestions`, and all related tests. Remove the Claude API call path from the Slack flow in `tui.rs`.

The `chrono::NaiveDate` and `crate::task::Priority` imports in slack.rs are no longer needed for the Slack module.

## Risks / Trade-offs

**[Bot token scope]** → `chat:write` is needed for replies. If the user's bot token doesn't have this scope, replies will fail gracefully with an error message. The existing OAuth flow should already request it, but we'll validate on first reply attempt.

**[Message volume]** → Channels with high traffic could produce a large inbox.md. Mitigated by: (a) only syncing configured channels, (b) limiting fetch to 200 messages per channel per sync, (c) pruning done messages after 7 days.

**[No thread context]** → Messages shown without thread context may lose meaning. Acceptable for v1 — the deep link lets the user jump to Slack for full context.

**[Workspace name requirement]** → Need the workspace name for deep links. Will attempt to fetch via `auth.test` API on first sync, which returns the workspace URL. Cache in config.
