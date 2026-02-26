## Why

Tasks managed in Todoist can't be brought into the CLI's local task file, requiring manual re-entry. A one-way import command would let users migrate or snapshot their Todoist tasks into the local workflow, with both sides marked so the operation is auditable.

## What Changes

- Add a `task import todoist` subcommand that fetches all open tasks from the user's Todoist account; supports a `--test` flag that imports only the first 3 tasks and skips applying the `exported` label in Todoist (safe to run repeatedly)
- Add a `task auth todoist` subcommand (and supporting `task auth revoke` / `task auth status`) that authenticates via the Todoist OAuth 2.0 browser flow and stores the resulting token locally
- Imported tasks are tagged `imported` in the local task file
- Each imported Todoist task is labeled `exported` in Todoist after import (idempotent: tasks already labeled `exported` are skipped on subsequent runs)
- **BREAKING**: Add a fourth priority level `Critical` to the `Priority` enum. Todoist P1 maps to `critical`, P2 → `high`, P3 → `medium`, P4 → `low`. Existing tasks without an explicit priority continue to default to `medium`.
- Add a `due_date` field (`Option<NaiveDate>`) to `Task`. Imported from Todoist's `due.date`. Settable via `--due` on `task add` and `task edit`. Displayed in `task list` and `task show`.
- Add a `project` field (`Option<String>`) to `Task`. Resolved from Todoist's `project_id` → project name at import time via the Todoist projects API. Settable via `--project` on `task add` and `task edit`. Filterable with `--project` in `task list` and the TUI filter mode.
- **Bump storage format to version 2** to reflect the new fields. Existing format:1 files are auto-migrated in-memory on load (new fields default to absent); the upgraded format:2 header and new metadata keys are written on the next save. Add a `task migrate` subcommand that explicitly rewrites the file to format:2 without changing task data.

## Capabilities

### New Capabilities

- `todoist-import`: Fetch open Todoist tasks, resolve project names, map fields (title, description, priority, due_date, project, labels→tags), append to local task file, apply `imported` tag, and label tasks `exported` in Todoist; skip tasks already labeled `exported`; support `--test` flag to limit import to 3 tasks without labeling in Todoist
- `todoist-auth`: OAuth 2.0 browser flow — open browser to Todoist authorization URL, spin up a local HTTP callback server to capture the authorization code, exchange for an access token, and persist the token to a local config file; support revoke and status subcommands

### Modified Capabilities

- `task-lifecycle`: Add `critical` as a new priority level above `high` (**BREAKING**); add `due_date` (optional date) and `project` (optional string) as new optional fields on `Task`. Priority order: `critical > high > medium > low`. Default priority for new tasks remains `medium`.
- `task-storage`: Add `due_date` and `project` fields to the markdown task storage format. Fields are optional; existing tasks without them remain valid (no migration required).
- `cli-interface`: Add `import todoist` and `auth todoist` / `auth revoke` / `auth status` subcommands; add `critical` as a valid `--priority` value; add `--due <date>` and `--project <name>` flags to `add` and `edit`; add `--project <name>` filter to `list`.
- `tui`: Add display and editing support for `critical` priority (distinct color, e.g., bright magenta); include `critical` as a picker option in `EditingPriority` mode (`c` key); display `due_date` and `project` in the task table and/or `task show` output; add `project:<name>` as a supported filter expression.

## Impact

- **New dependencies**: `reqwest` (HTTP client for Todoist API), `oauth2` (OAuth 2.0 flow), `tiny_http` or similar (local callback server), `open` (launch browser), `serde_json` (API response deserialization), `dirs` (config directory for token storage), `chrono::NaiveDate` (already a transitive dep via `chrono`)
- **Code**: `src/task.rs` (Priority enum, Task struct), `src/cli.rs` (new subcommands + flags), `src/main.rs` (dispatch), `src/parser.rs` (priority + new field parsing), `src/output.rs` (display due_date + project), `src/tui.rs` (priority display + picker + new columns), new `src/todoist.rs` (API client + import logic), new `src/auth.rs` (OAuth flow)
- **Config file**: Token stored at `~/.config/task-manager/todoist_token` (or platform equivalent via `dirs`)
- **Breaking change**: `critical` priority is additive; existing task files remain valid. `due_date` and `project` are optional fields with no migration required.
