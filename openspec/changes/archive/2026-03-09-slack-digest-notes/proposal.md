## Why

The existing Slack review flow extracts action items as individual tasks, but users lose the broader context of what was discussed. When reviewing Slack activity across multiple channels, users need both a summary of conversations (with links back to the original messages) and actionable tasks. A digest note captures the narrative — who said what, key decisions, important links — while task extraction handles the follow-ups. Together they provide a complete picture without requiring users to re-read every message.

## What Changes

- Add an AI-powered digest note generation step to the Slack review flow that summarizes messages per channel, including Slack message permalinks
- The digest note is created as a standard markdown note (using the existing `note.rs` infrastructure) with channel sections, message summaries, and deep links
- Extend the Slack NLP analysis to produce both a structured summary (for the note) and action items (for tasks) in a single AI call
- Add a TUI confirmation step where the user can preview the generated digest note and extracted tasks before committing them
- Generate Slack message permalinks using the channel ID and message timestamp

## Capabilities

### New Capabilities
- `slack-digest`: AI-powered digest generation from Slack messages — produces a markdown summary note with channel sections, message permalinks, and key discussion points, plus extracts action items as tasks. Covers the prompt design, response parsing, permalink construction, and note/task creation flow.

### Modified Capabilities
- `note-storage`: Add support for programmatic note creation with structured metadata (source: slack, generated timestamp, channel references) so digest notes can be identified and managed separately from manually-created notes.

## Impact

- **Modified files**: `src/slack.rs` (permalink generation, extended analysis prompt/response), `src/nlp.rs` (digest response type), `src/note.rs` (metadata support if needed), `src/tui.rs` (digest preview + confirmation UI)
- **External dependency**: Slack message permalink format uses `https://<workspace>.slack.com/archives/<channel_id>/p<timestamp>` — requires the workspace domain from auth or config
- **AI cost**: Single Claude API call per digest (replaces or extends the existing action-item-only call), returning both summary and action items
