## Context

Command Center is a Rust TUI task manager that integrates with Todoist (import) and Claude (NLP chat). The app stores tasks in a markdown file, credentials in `~/.config/task-manager/` (or platform equivalent via `dirs::config_dir()`), and uses `reqwest::blocking::Client` for HTTP calls. The existing auth pattern stores tokens as plaintext files with 0600 permissions (e.g., `todoist_token`, `claude_api_key`). The TUI uses a mode-based state machine (`Mode` enum) with per-mode key handlers and rendering functions.

The Slack integration needs to fit naturally into this architecture: same auth pattern, same HTTP client, same TUI mode system, same NLP engine.

## Goals / Non-Goals

**Goals:**
- Let users press `S` in the TUI to fetch new Slack messages from configured channels
- Use Claude to analyze messages and suggest action items (task title, priority, optional due date)
- Present suggestions in a review UI where users accept, skip, or edit before creating tasks
- Track "last read" per channel so subsequent fetches only return new messages
- Integrate with `deploy.sh` for guided Slack token setup

**Non-Goals:**
- Sending messages back to Slack (read-only integration)
- Real-time/websocket streaming (poll-on-demand only)
- OAuth web flow — users provide a pre-created Bot User OAuth Token directly
- Slack workspace administration or app installation flow
- Thread/reply fetching (top-level messages only for v1)
- Reacting to or marking messages as read in Slack

## Decisions

### 1. Single module `src/slack.rs` for both auth and client

**Decision:** Combine Slack auth helpers and API client in one module, similar to how `src/todoist.rs` handles both API types and fetch logic.

**Rationale:** The Slack integration is self-contained — auth functions (path, read, write) are only used by the Slack client. Splitting into `slack_auth.rs` + `slack_client.rs` would add file overhead for ~200 lines total. The Todoist module follows this same pattern.

**Alternative considered:** Separate `src/slack_auth.rs` — rejected as premature given the scope.

### 2. High-water-mark stored in a JSON state file

**Decision:** Store per-channel last-read timestamps in `~/.config/task-manager/slack_state.json` as a JSON object: `{ "C1234": "1709654321.000100", ... }`. Use Slack's message `ts` field (string timestamp) as the marker.

**Rationale:** The `conversations.history` API accepts an `oldest` parameter (Unix timestamp string). Storing the latest `ts` from each fetch lets us pass it as `oldest` on the next call to get only new messages. JSON is simple to read/write and doesn't need a new dependency (already using `serde_json`).

**Alternative considered:** Store in the config file as `slack-hwm-CHANNEL:TIMESTAMP` lines — rejected because it pollutes the user-facing config with internal state, and the existing config format doesn't handle dynamic keys well.

### 3. Channel selection stored in config

**Decision:** Users configure monitored channels via `task config set slack-channels C1234,C5678` (comma-separated channel IDs). The TUI's Slack flow reads this config value to know which channels to fetch. A channel picker mode (using `conversations.list`) lets users browse and select channels interactively.

**Rationale:** Channel IDs are stable (names can change). Storing in the existing config system keeps setup simple. The interactive picker is a convenience that writes the same config value.

**Alternative considered:** Separate `slack_channels.json` file — rejected to avoid yet another state file when the config system already supports key-value storage.

### 4. Batch NLP analysis with a dedicated prompt

**Decision:** Send all new messages (across channels) to Claude in a single NLP call with a purpose-built system prompt. The prompt instructs Claude to return a JSON array of suggested actions, each with: `title`, `priority`, `due_date` (optional), `source_channel`, `source_text`. Messages that aren't actionable are excluded from suggestions.

**Rationale:** A single call is cheaper and faster than per-message calls. The NLP engine already supports structured JSON responses. Batching lets Claude see context across messages (e.g., a thread of related requests).

**Alternative considered:** Per-message NLP calls — rejected due to latency and API cost. Streaming/incremental suggestions — rejected as over-engineering for v1.

### 5. TUI review flow as a new `Mode::SlackReview`

**Decision:** Add `Mode::SlackReview` to the TUI mode enum. When triggered by `S`, the flow is:
1. Fetch new messages from all configured channels (show spinner)
2. Send to NLP for analysis (show spinner)
3. Display suggestions in a list — each shows: suggested title, priority, source snippet
4. User navigates with `j`/`k`, then: `Enter` = accept (create task), `e` = edit title before accepting, `s` = skip, `Esc` = exit review

**Rationale:** Follows the existing TUI pattern (dedicated mode with handler + renderer). The review-before-create model matches the Todoist import UX — users stay in control of what becomes a task.

**Alternative considered:** Reuse the NLP chat mode with a special command — rejected because the review-and-accept UX is fundamentally different from conversational chat.

### 6. Use `reqwest::blocking::Client` for Slack API calls

**Decision:** Use `reqwest::blocking` for Slack HTTP calls, matching the Todoist module's approach.

**Rationale:** The app is single-threaded with blocking I/O throughout. Adding async would require tokio and restructuring. The Slack calls happen on user action (press `S`), not continuously, so blocking is acceptable.

### 7. Deploy script integration

**Decision:** Add a Slack token setup section to `deploy.sh` between the Claude and summary sections, following the same pattern: check if token file exists, prompt if not, save with 0600 permissions.

**Rationale:** Consistent with the existing Todoist and Claude setup flow. Users running `deploy.sh` for the first time get prompted for all three integrations in sequence.

## Risks / Trade-offs

**[Rate limiting]** → Slack's `conversations.history` is rate-limited (1 req/min for new apps on commercial plans, higher for internal apps). Mitigation: fetch on-demand only (user presses `S`), not automatically. Show clear error if rate-limited.

**[Large message volumes]** → A busy channel could have hundreds of new messages. Mitigation: cap at 200 messages per channel per fetch (Slack's default page size is 100, fetch at most 2 pages). Warn the user if messages were truncated.

**[Bot token setup complexity]** → Users must create a Slack App, install it to their workspace, and copy the Bot User OAuth Token. Mitigation: `deploy.sh` and `task auth slack` print clear instructions with a link to https://api.slack.com/apps. Document required scopes: `channels:history`, `channels:read`.

**[NLP cost]** → Each Slack review sends messages to Claude. Mitigation: batch into a single API call; users control when to trigger (not automatic).

**[Channel ID vs name]** → Users see channel names but we store IDs. Mitigation: the channel picker shows `#name (ID)` and stores the ID. The review UI resolves IDs to names via the cached channel list.
