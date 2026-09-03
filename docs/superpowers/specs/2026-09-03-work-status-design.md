# Work Status Design

## Overview

Agents working on tasks currently have no way to signal their progress. The
task's only state is the binary `status` (`open`/`done`), which the human
sets by hand ("Mark done"). This spec adds a second, independent field —
`work_status` — that agents update via the CLI as they progress through a
task, and that the human tracks in the web UI. It never changes `status`
automatically; marking a task actually done remains a separate, human-driven
action.

## Decisions already made (confirmed with the user)

- **Independent field**, not a replacement for `status`/`Status::Open|Done`.
  Due-window filtering, `Mark done`/`Reopen`, and recurrence spawning are
  untouched.
- Setting `work_status` to `Complete` does **not** auto-mark the task done.
  It's purely informational; the human still explicitly marks the task done.
- **Web + CLI only** for this pass. The TUI keeps showing only `open`/`done`,
  unchanged. TUI parity is deliberately out of scope here — a candidate for a
  later pass if it turns out to matter.

## Data model

New enum in `src/task.rs`, next to `Effort`/`Priority`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkStatus {
    Todo,
    InProgress,
    WaitingForReview,
    Complete,
}

impl std::fmt::Display for WorkStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkStatus::Todo => write!(f, "todo"),
            WorkStatus::InProgress => write!(f, "in-progress"),
            WorkStatus::WaitingForReview => write!(f, "waiting-for-review"),
            WorkStatus::Complete => write!(f, "complete"),
        }
    }
}

impl std::str::FromStr for WorkStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "todo" | "to-do" => Ok(WorkStatus::Todo),
            "in-progress" | "in_progress" | "inprogress" => Ok(WorkStatus::InProgress),
            "waiting-for-review" | "review" => Ok(WorkStatus::WaitingForReview),
            "complete" | "done" => Ok(WorkStatus::Complete),
            _ => Err(format!(
                "Invalid work status: '{}'. Valid values: todo, in-progress, waiting-for-review, complete",
                s
            )),
        }
    }
}
```

`Task` gets one new field, mirroring `effort`'s pattern exactly:

```rust
#[serde(skip_serializing_if = "Option::is_none")]
pub work_status: Option<WorkStatus>,
```

`None` is the default for both existing and newly-created tasks — no work
status shows in the UI until an agent (or the human) explicitly sets one.

## Persistence (`src/db.rs`)

**This needs a real migration**, unlike some earlier additions. The current
schema uses `CREATE TABLE IF NOT EXISTS`, which does nothing to a table that
already exists — confirmed by checking the schema of the user's real
`tasks.db`, which was created fresh (via the markdown→SQLite migration) with
every current column already present, including `effort`. There is no
existing precedent in this codebase for adding a column to an
already-populated table.

Add, immediately after `execute_batch(SCHEMA_SQL)` in `open_conn`:

```rust
conn.execute("ALTER TABLE tasks ADD COLUMN work_status TEXT", []).ok();
```

`.ok()` (not `?`) because `ALTER TABLE ADD COLUMN` fails if the column
already exists, and SQLite has no `ADD COLUMN IF NOT EXISTS`. Discarding the
error here is safe *specifically* because the only expected failure mode is
"duplicate column" on a database that already has it — any other failure
(e.g. a locked or corrupt file) will surface immediately on the very next
query against `tasks`, so silently swallowing it here does not hide a real
problem for long. The implementer must verify this by testing against both
a fresh database and one seeded with the pre-migration schema.

`row_to_task`, `insert_task_row`, and the `SELECT`/`INSERT` column lists in
`db.rs` all get the new column, following exactly the same shape as
`effort`.

## CLI (`src/cli.rs`, `src/commands.rs`, `src/bin/task.rs`)

Extend the existing `task edit <id>` command rather than adding a new
subcommand — agents already call this command, and it's consistent with how
`--priority`/`--effort`/`--due` work:

```
task edit <id> --work-status <todo|in-progress|waiting-for-review|complete>
```

- `cli.rs`: add `#[arg(long)] work_status: Option<String>,` to the `Edit`
  variant.
- `commands.rs`: add `work_status: Option<String>` to `EditArgs`, parse it
  with `WorkStatus::from_str` the same way `effort` is parsed, and set
  `t.work_status = Some(w)` in `edit()`.
- `bin/task.rs`: thread `work_status` through the `Command::Edit` match arm
  into `EditArgs`.
- `commands.rs`'s `show()`: print `work_status: {}` when set, in the same
  style as the existing `effort:` line, positioned right after `effort`.

The TUI (`bin/task_tui.rs`, `tui.rs`) is explicitly untouched — it has its
own separate edit flow that doesn't go through `commands::edit`, and per the
scope decision above, work status isn't exposed there in this pass.

## Server API (`src/server.rs`)

`EditTaskRequest` gets a new field:

```rust
pub work_status: Option<String>,
```

Parse and apply it in `edit_task` following the **exact same empty-string-
means-clear pattern** already used for `due` and `agent`:

```rust
let work_status: Option<Option<WorkStatus>> = req
    .work_status
    .as_deref()
    .map(|s| {
        if s.is_empty() {
            Ok(None)
        } else {
            WorkStatus::from_str(s).map(Some)
        }
    })
    .transpose()
    .map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?;
...
if let Some(w) = work_status {
    t.work_status = w;
}
```

This gives the web UI a real "clear work status" action (sending
`work_status: ""`), matching how due-date and agent clearing already work.

`Task`'s existing `#[derive(Serialize)]` already round-trips the new field
to JSON automatically (same as `effort`) — no other handler changes needed.

## Web UI (`web/src/`)

- **`types.ts`**: add `work_status?: 'todo' | 'in-progress' | 'waiting-for-review' | 'complete';`
  to the `Task` interface.
- **`api.ts`**: add `work_status` to `editTask`'s `changes` parameter type.
- **Table**: a new narrow column showing a colored pill/badge when
  `work_status` is set (nothing rendered when it's `undefined`). Suggested
  colors, distinct from the existing priority/status-icon palette already in
  use: Todo = `var(--fg-4)` (neutral), In Progress = `var(--citrine)`
  (matches the existing "agent waiting" accent color), Waiting for Review =
  a blue/cyan not yet used elsewhere in the palette, Complete =
  `var(--magenta)` or a green if one exists in `tokens.css` — implementer
  should check `web/src/tokens.css` for the actual available design tokens
  before hardcoding a hex value.
- **Inspector**: a new `FieldRow label="Work status"` using the same
  `EditableField type="select"` pattern already used for Priority/Effort
  (this field has no agent-name-collision risk like the Agent field did, so
  the native-select approach is fine here — no custom picker needed).
  Options: Todo / In Progress / Waiting for Review / Complete, plus an
  empty/"None" option so a human can clear it back to unset.
- **Group-by**: add `'work_status'` as a new `GroupBy` value and a
  corresponding entry in `GROUP_BY_OPTIONS`, following the exact existing
  pattern for `'priority'`/`'project'`/`'agent'` in `groupKey`/`groupTasks`
  in `App.tsx`. Tasks with no `work_status` group under a key like
  `'no status'`, matching the existing `'unassigned'`/`'no project'`
  convention for the other groupings.

## Testing

- `db.rs`: a test that seeds a database with the schema *as it exists before
  this change* (i.e. without the `work_status` column, replicating a real
  user's existing `tasks.db`), then opens it via `open_conn`/`load` and
  confirms the migration adds the column without error and without losing
  existing rows. Also a round-trip test (insert a task with `work_status`
  set, reload, confirm it comes back).
- `server.rs`: mirror the existing `test_edit_task_empty_due_clears_it` /
  `test_edit_task_empty_agent_clears_it` tests with a
  `test_edit_task_empty_work_status_clears_it`, plus a basic
  `test_edit_task_sets_work_status` and an invalid-value 400 test.
- `commands.rs`/CLI: a test that `task edit <id> --work-status in-progress`
  round-trips through `show`.

## Out of scope (explicitly, per user decisions above)

- TUI display/editing of `work_status`.
- Auto-completing a task when `work_status` reaches `Complete`.
- A dedicated CLI subcommand (e.g. `task status`) — folded into `edit`
  instead.
- Any change to `Status`/`open`/`done` semantics, due-window filtering, or
  recurrence logic.
