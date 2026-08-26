# SQLite Backend for Task Storage

## Problem

Tasks currently live in a hand-rolled Markdown file (`tasks.md`) with a custom
metadata-comment format (documented in `AGENTS.md`), parsed and serialized by
`src/parser.rs` and read/written by `src/storage.rs`. This works but has
sharp edges:

- Concurrency is handled by an advisory file lock (`fs2`) around a
  write-whole-file-and-rename cycle; there's no real transactional safety.
- ID assignment relies on a manually maintained `next-id` counter in the file
  header that every writer must read, increment, and persist correctly.
- Every operation that touches tasks (TUI, CLI `add`, Todoist sync) goes
  through a full parse-mutate-serialize-write cycle of the entire file.
- AI agents currently work tasks by directly editing `tasks.md` text
  per rules in `AGENTS.md` — fragile, and doesn't scale to a binary store.

This change replaces the task storage backend with SQLite. Notes remain
Markdown files on disk, unchanged.

## Goals

- Replace `tasks.md` as the source of truth with a SQLite database
  (`tasks.db`), while keeping `Task`/`TaskFile` (`src/task.rs`) as the
  in-memory model unchanged.
- Give SQLite ownership of concurrency (WAL mode) and ID assignment
  (`AUTOINCREMENT`), removing the manual file-lock and `next-id` counter.
- Add the missing non-interactive CLI operations (`list`, `show`, `edit`,
  `done`, `reopen`, `rm`) so AI agents can work tasks via the CLI instead of
  editing a file directly.
- One-time, automatic migration of an existing `tasks.md` into `tasks.db` on
  first run, with the old file left in place afterward.
- Update `AGENTS.md` to document the CLI workflow, replacing the file-format
  documentation.
- Show the running build's version in the TUI header (the CLI already
  exposes `--version` via clap).

## Non-goals

- No `task export`/`task import` round-trip tooling between SQLite and
  Markdown — SQLite is the sole source of truth going forward.
- No change to how Notes are stored (`src/note.rs` is untouched).
- No schema versioning system beyond what's needed to detect "not yet
  migrated" vs. "already on SQLite" (a single migration step, not a general
  migrations framework — YAGNI until a second schema change is needed).
- No multi-directory / multi-tenant database features beyond what already
  exists (one DB per `default-dir`, same as one `tasks.md` per `default-dir`
  today).

## Design

### Data model

`Task`, `Status`, `Priority`, `Effort`, `Recurrence`, and `TaskFile` in
`src/task.rs` are unchanged, **including** `TaskFile.next_id` and
`TaskFile.format_version`. This is a deliberate revision from the original
brainstormed design (which proposed dropping `next_id` in favor of SQLite
`AUTOINCREMENT`): `next_id` is read and incremented in-place at several
call sites (`tui.rs`'s new-task and recurrence-spawn flows, `todoist.rs`'s
bulk import, both `add` command handlers), not just at load time. Making
IDs autoincrement-assigned would require touching every one of those call
sites to learn the ID only after insert. Instead, `db::load` computes
`next_id` transiently as `MAX(id) + 1` (or `1` if the table is empty) every
time it loads — the value is never persisted in the DB, matching how it's
never truly "stored" today either (the markdown header is just a cache of
this same derivation, per `parser.rs`'s "always ensure next_id > max
existing id" comment). `format_version` stays for parity with `TaskFile`'s
current shape but is unused by the SQLite path — schema identity is just
"does `tasks.db` exist." All existing callers that do
`let id = task_file.next_id; task_file.next_id += 1;` then push a `Task`
and call `save` keep working unmodified.

### Schema

Single table:

```sql
CREATE TABLE tasks (
    id          INTEGER PRIMARY KEY,   -- explicit id always supplied by callers; see Data model
    title       TEXT NOT NULL,
    status      TEXT NOT NULL,           -- 'open' | 'done'
    priority    TEXT NOT NULL,           -- 'critical' | 'high' | 'medium' | 'low'
    tags        TEXT NOT NULL DEFAULT '', -- comma-separated, same encoding as today
    created     TEXT NOT NULL,           -- RFC 3339
    updated     TEXT,                    -- RFC 3339, nullable
    description TEXT,
    due_date    TEXT,                    -- YYYY-MM-DD, nullable
    project     TEXT,
    recurrence  TEXT,                    -- via Recurrence's Display/FromStr, nullable
    notes       TEXT NOT NULL DEFAULT '', -- comma-separated slugs
    agent       TEXT,
    effort      TEXT                     -- 'high' | 'medium' | 'low', nullable
);
```

No separate tables for tags/notes — they stay as delimited text columns,
matching the current in-file encoding and avoiding join complexity that
nothing in the app currently needs (`TaskFile` is always loaded/saved in
full, never queried piecemeal at the SQL level beyond simple filters).

### Storage module (`src/db.rs`, replacing most of `src/storage.rs`)

- `resolve_file_path` keeps its existing resolution logic (`--file` flag →
  `TASK_FILE` env → `default-dir` config → `./tasks.md`) but the default
  filename becomes `tasks.db`, and callers passing an explicit `--file`/
  `TASK_FILE` now point at a `.db` path directly. (The flag/env var names are
  unchanged — only what they resolve to changes.)
- `load(path) -> Result<TaskFile, String>`: opens the SQLite connection
  (WAL mode, busy timeout), runs the migration check (below), `SELECT *`
  the whole table into a `Vec<Task>`, wraps it in `TaskFile`.
- `save(path, &TaskFile) -> Result<(), String>`: today's whole-file
  rewrite becomes a transaction — `DELETE FROM tasks; INSERT ...` for each
  task in `task_file.tasks`, one transaction, always supplying the
  in-memory `id` explicitly. This keeps `load`/`save` semantics identical
  to today from the callers' point of view (mutate `TaskFile` in memory —
  including assigning new tasks an id from `task_file.next_id` — then call
  `save`), so `tui.rs`, `bin/task.rs`, `bin/task_tui.rs`, and `todoist.rs`
  need no changes to their id-assignment logic, only to the module path
  (`storage::` → `db::`) and dropping the now-removed `strict` parameter
  from `load` calls.
- `backup_daily(path)`: replace `fs::copy` with `VACUUM INTO
  <backup_dir>/tasks-<date>.db` run against the live connection — safe
  under WAL, produces one consistent file. Same 7-backup pruning logic,
  same `.backups/` directory, just a `.db` extension.

### Migration

On `load`, if the resolved `.db` path doesn't exist:

1. Check for a `tasks.md` in the same directory.
2. If present, run the existing `parser::parse` against it (this code path
   is kept solely for migration — no longer used for normal load/save).
3. Create `tasks.db`, insert all parsed tasks with their existing `id`
   values preserved (so cross-references — e.g. notes' `task:<id>` links,
   if any exist — stay valid). No sequence-seeding is needed since ids are
   always supplied explicitly, never DB-assigned (see Data model above).
4. Leave `tasks.md` on disk, untouched, going forward.
5. If neither file exists, create an empty `tasks.db` (today's "start with
   an empty `TaskFile`" behavior).

This runs once per directory, transparently, the first time any binary
(`task`, `task-tui`) touches that directory after the upgrade.

### CLI additions (`src/cli.rs`, `src/bin/task.rs`)

New subcommands, all operating through the same `db::load`/mutate/
`db::save` pattern the existing `add` command uses:

- `task list [--status open|done] [--agent NAME] [--project NAME] [--tag TAG] [--due-before DATE]`
  — prints one line per matching task (id, priority, title, due date if
  set), sorted the same way the TUI's default view sorts (priority, then
  due date).
- `task show <id>` — prints full task detail (all fields, description body).
- `task edit <id> [--title T] [--priority P] [--due D] [--project P] [--tags T] [--agent A] [--description D] [--effort E]`
  — updates only the fields whose flag was passed, sets `updated`, mirroring
  the "Editing a task" rules from the current `AGENTS.md`.
- `task done <id>` / `task reopen <id>` — flip status, set `updated`; `done`
  also handles recurrence (compute next due date and either update the
  existing task or leave completion as-is — matching whatever the TUI's
  current completion behavior is for recurring tasks, verified during
  implementation by reading `tui.rs`).
- `task rm <id>` — deletes the row.

Errors (task not found, invalid enum value) use the same
`Result<(), (i32, String)>` convention already used in `bin/task.rs`.

`src/bin/task.rs` and `src/bin/task_tui.rs` are near-duplicates today (the
only difference is what `None | Some(Command::Tui)` does — print a message
vs. launch the TUI). Adding six new subcommands to both independently would
double the new surface area for no reason. This change extracts the task
CRUD command bodies (`add`, `list`, `show`, `edit`, `done`, `reopen`, `rm`)
into a new `src/commands.rs` module as plain functions
(`pub fn add(path: &Path, ...) -> Result<String, (i32, String)>`, etc., one
per subcommand, returning the stdout text to print on success), called
identically from both binaries. This only touches the commands being added
in this change — the existing `Auth`/`Config`/`Note`/`Agent` duplication in
the two binaries is left as-is (out of scope).

### Version display

Both `task` and `task-tui` gain a way to see the running build's version.
`task --version` already works via clap's `version` attribute on `Cli`
(`src/cli.rs`); this is unchanged. The TUI's header bar (`draw_header` in
`src/tui.rs`) is extended to include the crate version
(`env!("CARGO_PKG_VERSION")`) so it's visible while the TUI is running,
e.g. `task-manager v3.4.0  |  <view>  |  <filter>`.

### `AGENTS.md` update

Per the CLAUDE.md instruction to keep `AGENTS.md` in sync, it gets rewritten
to:

- Drop the "File Format" / "Header" / "Task block" / "Metadata fields" /
  "Rules for Each Operation" sections describing `tasks.md` text structure.
- Replace them with CLI usage: how to find your tasks (`task list --agent
  <name>`), and how to add/edit/complete/reopen/delete via the new
  subcommands, with the same semantics (never reuse an id — now automatic;
  set `updated` on edit — now automatic).
- Keep the sections that are unaffected: "Finding Your Tasks" (config
  lookup), "Reading Your Instructions", "Reading and Updating Memory", "TUI
  Auto-Filter" — these don't depend on the file format.

### Dependencies

Add `rusqlite = { version = "0.32", features = ["bundled"] }` (bundled
avoids a system SQLite dependency). Remove `fs2` once the manual file lock
is deleted (SQLite's own locking replaces it) — keep it if anything else in
the codebase still uses it (`note.rs` writes don't currently lock; leaving
note storage as-is means `fs2` may become unused and should be dropped from
`Cargo.toml` if so).

### Testing

- `src/db.rs` unit tests replace `src/storage.rs` tests: same test names/
  shapes (load nonexistent, load empty, save+load roundtrip, backup
  creation/pruning) but against a temp-directory SQLite file instead of a
  temp Markdown file.
- A new migration test: write a `tasks.md` with known tasks into a temp
  dir, call `load`, assert `tasks.db` now exists with matching rows and
  `tasks.md` is untouched (still present, same content).
- New CLI integration tests in `tests/integration.rs` for `list`/`show`/
  `edit`/`done`/`reopen`/`rm`, following the existing patterns for `add`.
- `parser.rs`'s Markdown parse/serialize tests are kept only for the
  functions still used by migration (`parse`) and by `add`'s due-date input
  parsing (`parse_due_date_input`); the serialize-to-Markdown path and its
  tests are deleted since nothing calls it after migration.

## Open questions for implementation time

None outstanding — resolved during brainstorming:
- Agent access model: CLI (not a generated file export).
- Old `tasks.md` handling post-migration: left in place, untouched.
- Export/import tooling: not built.
