## Why

The TUI's current filter system (`f` key) requires exact syntax like `status:open priority:high tag:frontend`. Users must remember filter syntax and can only perform one operation at a time. A natural language interface powered by Claude would let users express complex queries and bulk operations conversationally (e.g., "show all tasks related to the FLOW AI project", "mark all frontend tasks due this week as high priority").

## What Changes

- Add a new TUI mode (`NlpInput`) triggered by `:` key for entering natural language commands
- Integrate with the Claude API to interpret user intent and translate it into task operations
- Support two categories of NLP commands:
  - **Queries/views**: Natural language filters applied to the task list (e.g., "show tasks for project Work due this month")
  - **Bulk updates**: Modify multiple tasks at once based on criteria (e.g., "set priority high on all tasks tagged urgent")
- Display LLM-interpreted results in the TUI with a confirmation step before applying bulk changes
- Require a Claude API key stored via a new `task auth claude` CLI command

## Capabilities

### New Capabilities

- `tui-nlp`: NLP input mode, Claude API integration for intent parsing, query execution, bulk update execution with confirmation, and API key management
- `claude-auth`: Storage and retrieval of Claude API key, `task auth claude` CLI subcommand

### Modified Capabilities

- `tui`: Add `NlpInput` mode variant and `:` keybinding to mode-based input handling; update footer hints to include `::command`
- `cli-interface`: Add `task auth claude` subcommand for API key management

## Impact

- **Code**: `src/tui.rs` (new mode, rendering), new `src/nlp.rs` module (Claude API client, intent parsing, command execution), `src/auth.rs` (Claude key storage), `src/cli.rs` (new auth subcommand)
- **Dependencies**: New crate dependency on `serde_json` (already present) for Claude API request/response handling; uses existing `reqwest` for HTTP calls
- **External**: Requires Claude API key and network access to `api.anthropic.com`
- **UX**: New `:` key in normal mode; confirmation prompt before bulk updates to prevent accidental changes
