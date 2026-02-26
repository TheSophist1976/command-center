## Context

The CLI task manager stores tasks locally in a Markdown file (`tasks.md`). Users who already manage work in Todoist have no path to migrate or snapshot those tasks into the local workflow without manual re-entry.

This change adds:

1. **OAuth 2.0 authentication** (`task auth todoist`) — browser-based, token persisted locally
2. **One-way import** (`task import todoist`) — fetches open Todoist tasks, maps fields, appends to local file, marks tasks in both systems to prevent duplicate imports
3. **New fields on `Task`** — `due_date` (optional date) and `project` (optional string), usable independently of the import path
4. **Critical priority level** — a fourth priority above `high`, required to faithfully map Todoist's P1
5. **Storage format v2** — captures the new fields; existing v1 files are auto-migrated on load

The current codebase is a single Rust binary with: `task.rs` (data model), `parser.rs` (parse + serialize), `storage.rs` (file I/O with locking), `cli.rs` (clap commands), `main.rs` (dispatch), `output.rs` (display), `tui.rs` (terminal UI).

## Goals / Non-Goals

**Goals:**

- Import all open Todoist tasks once; skip any already labeled `exported` in Todoist
- Authenticate via OAuth 2.0 browser flow; persist token in platform config dir
- Map Todoist fields to local `Task`: title, description, priority (P1–P4 → critical/high/medium/low), labels → tags, due date, project name
- Add `due_date` and `project` to `Task`; support `--due` / `--project` on `add` and `edit`; filter by `--project` in `list`; display in `show` and `tui`
- Add `critical` priority to `Priority` enum and all display / filter / TUI paths
- Auto-migrate format:1 files to format:2 on load; provide `task migrate` for explicit upgrade

**Non-Goals:**

- Two-way sync (no write-back of local changes to Todoist)
- Importing completed Todoist tasks
- Importing sub-tasks, sections, comments, attachments, or assignees
- OAuth refresh-token rotation (access tokens are long-lived for Todoist personal use)
- Importing from multiple Todoist accounts simultaneously

## Decisions

### 1. New module layout: `src/todoist.rs` and `src/auth.rs`

All Todoist API calls and field mapping live in `src/todoist.rs`. OAuth flow lives in `src/auth.rs`. This keeps the core data model and storage layer untouched by integration concerns.

*Alternative*: inline logic in `main.rs`. Rejected — it's already dispatch-heavy and would become unreadable.

### 2. OAuth callback server: `tiny_http`

The OAuth flow requires a local redirect URI. We spin up a `tiny_http` listener on `127.0.0.1:7777` for one request, capture the `?code=` parameter, and shut down immediately.

`tiny_http` adds one small dependency and avoids pulling in a full async runtime. A tokio/axum approach was considered but is disproportionate for a single one-shot request.

*Redirect URI*: `http://127.0.0.1:7777/callback` — must be registered in the Todoist OAuth app settings.

### 3. Token storage: `dirs::config_dir()`

Token stored at `~/.config/task-manager/todoist_token` (Linux/macOS) or the platform equivalent via `dirs`. Plain text file, 0600 permissions set at write time.

*Alternative*: system keychain. Rejected for this iteration — adds OS-specific dependencies and complexity; users can protect the file themselves.

### 4. HTTP client: `reqwest` (blocking feature)

Todoist API calls are synchronous from the CLI's perspective. The blocking `reqwest` client avoids introducing an async runtime. Two calls per import run: `GET /tasks` (open tasks) and `GET /projects` (project id → name map). After import, one `POST /tasks/{id}/close` per task is replaced by a label-based approach: `POST /tasks/{id}` to add the `exported` label, which is idempotent and non-destructive.

### 5. Test mode: `--test` flag on `task import todoist`

`task import todoist --test` imports the first 3 tasks from Todoist but does NOT apply the `exported` label back to Todoist. This makes the flag safe to run repeatedly against a real account without side effects. The tasks are written to the local file normally (with the `imported` tag), so the user can inspect them and delete them if needed.

The limit of 3 is fixed and not configurable — the goal is a quick sanity check, not a partial import.

*Alternative*: `--dry-run` (no writes at all). Rejected — writing 3 real tasks is more useful for verifying the full round-trip including storage, field mapping, and display.

### 6. Idempotency: `exported` label in Todoist

On each import run, tasks already carrying the `exported` label are skipped. After a task is successfully appended to the local file, we add the `exported` label via the Todoist API. If the CLI crashes between writing the local file and labeling in Todoist, re-running will re-import that task (duplicate); the user can deduplicate manually. This is acceptable for a one-way migration tool.

*Alternative*: track imported Todoist IDs in a local state file. Rejected — adds complexity; the Todoist label approach is visible to the user in Todoist itself.

### 7. Priority mapping

| Todoist | Local |
|---------|-------|
| P1 (urgent) | `critical` |
| P2 (high) | `high` |
| P3 (medium) | `medium` |
| P4 (low — default) | `low` |

`Critical` is inserted above `High` in the `Priority` enum. Display order everywhere is `critical > high > medium > low`.

### 8. `due_date` as `NaiveDate`, not `DateTime`

Todoist's `due.date` is a plain date string (`YYYY-MM-DD`) when no time component is present. `chrono::NaiveDate` is the right type. Todoist also supports datetime dues but we map only the date portion to keep the local model simple. `chrono` is already a dependency.

### 9. Project resolution at import time

At import, we call `GET /projects` once to build a `HashMap<project_id, project_name>`. Each task's `project_id` is resolved to a name string before storage. We do not store the Todoist project ID locally — the `project` field is a plain `Option<String>` (name only).

### 10. Storage format v2

Format:2 adds two optional keys to the metadata comment:

```
<!-- id:3 priority:high tags:frontend due:2025-06-01 project:Work created:... -->
```

The parser already ignores unknown keys (tolerant by default), so format:1 files load correctly with `due` and `project` defaulting to `None`. On next save, the file is written with `<!-- format:2 -->` and the new keys serialized when present. No in-place rewriting is needed on load — the upgrade happens naturally on the next mutation.

`task migrate` forces a save without any task mutation, triggering the format upgrade explicitly.

*Alternative*: require explicit migration before use. Rejected — auto-migration on next save is transparent and zero-friction for existing users.

### 11. `task auth` and `task import` as top-level subcommand groups

```
task auth todoist    # start OAuth flow
task auth status     # print token status
task auth revoke     # delete stored token

task import todoist  # run import
```

Both use nested `Subcommand` enums in `cli.rs`. This keeps the command surface clean and leaves room for future integrations.

## Risks / Trade-offs

- **Todoist OAuth app registration required**: Users must create a Todoist OAuth application and provide a client ID + client secret. These can be supplied via environment variables (`TODOIST_CLIENT_ID`, `TODOIST_CLIENT_SECRET`) or prompted at first run.
  → *Mitigation*: Document the setup steps clearly; consider bundling a default app ID for single-user tools if Todoist's terms allow.

- **Token stored in plaintext**: The access token gives full Todoist read/write access.
  → *Mitigation*: Set file permissions to 0600 at write time; document the risk; defer keychain integration to a future iteration.

- **Import collision if Todoist title matches existing local task**: We do not deduplicate by title — two tasks with the same title are both valid locally.
  → *Mitigation*: The `imported` tag distinguishes imported tasks; users can review with `task list --tag imported`.

- **Rate limiting**: Todoist REST API v2 has rate limits. A typical user has hundreds of tasks, well within limits for a one-shot import.
  → *Mitigation*: No special handling needed for typical use; errors from the API are surfaced as CLI errors.

- **`critical` is a breaking parse change**: Existing serialized files do not contain `critical` and will parse fine. New files with `priority:critical` will fail to parse on older binary versions.
  → *Mitigation*: Format version bump to 2 signals incompatibility; older binaries will emit a warning for format:2 files (current behavior: unsupported version warning in strict mode, silent in tolerant mode).

## Migration Plan

1. **Parser**: update `Priority::from_str` to accept `"critical"`; update `serialize` to emit `"critical"`; update format version check to accept both `1` and `2`; add `due` and `project` keys to `parse_metadata_comment`; add them to `serialize`.
2. **Task struct**: add `due_date: Option<NaiveDate>` and `project: Option<String>`; add `Critical` variant to `Priority`.
3. **`task migrate`**: loads the file, bumps `format_version` to 2, saves — no task data changes.
4. **CLI / output / TUI**: add `critical` everywhere `Priority` is matched; add `due_date` / `project` display and filter paths.
5. **New modules**: `src/auth.rs` and `src/todoist.rs`.
6. **Rollback**: downgrade binary; format:2 files will load silently in tolerant mode on older binaries (unknown priority values fall back to `medium`; unknown metadata keys are ignored).

## Open Questions

- Should `TODOIST_CLIENT_ID` / `TODOIST_CLIENT_SECRET` be baked in as defaults (for personal-use distribution) or always require user-supplied values? → Defer to implementation; start with env-var-only.
- Should `task list` display `due_date` and `project` columns by default, or only when at least one task has those fields set? → Default to showing them when non-empty; always show in `task show`.
- Port for the OAuth callback server: `7777` — confirm no common conflict before finalizing.
