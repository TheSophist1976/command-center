## Context

The `note` module (`src/note.rs`) is fully implemented with functions for slugifying, reading, writing, discovering, and deleting notes. Notes are stored as `.md` files alongside `tasks.md`. The TUI can manage notes, but no CLI subcommands exist. The CLI is built with `clap` in `src/cli.rs` and dispatched in `src/main.rs`.

## Goals / Non-Goals

**Goals:**
- Expose `list`, `add`, `show`, `edit`, `rm` operations as `task note <sub>` subcommands
- Reuse existing `note` module functions with zero changes to `note.rs`
- Follow the existing `auth`/`config` subcommand pattern exactly

**Non-Goals:**
- Opening an editor for body input (body is passed as a flag value)
- Stdin piping for body content
- Note search or filtering

## Decisions

**Add `Note` variant to `Command` enum with a `NoteCommand` sub-enum**
Following the same pattern as `Auth` and `Config` — a top-level variant holding a `#[command(subcommand)]` of a new `NoteCommand` enum. This keeps CLI structure consistent and clap handles help/validation automatically.

**Pass body as a `--body` flag (not stdin or editor)**
The primary consumer is Cowork (Claude running shell commands). Flag-based input is unambiguous, composable, and doesn't require TTY. Alternative (opening `$EDITOR`) would block non-interactive use entirely.

**`note edit` requires at least one of `--title` or `--body`**
If neither is provided, emit a usage error rather than a no-op. This surfaces mistakes immediately.

**`note add` creates an empty-body note**
The `add` command only requires a title; the user can populate the body with `note edit --body` afterward. This keeps `add` simple and aligns with the existing `note.rs` write path.

**Output format for `note list`: `<slug>  <title>` (two spaces)**
Parseable by scripts; consistent with the tab-separated-ish pattern used elsewhere. Sorted alphabetically by slug (already guaranteed by `discover_notes`).

**Attaching a note to a task via `task note add --task <id>` and `task note link <slug> <task-id>`**
The `note` link lives on the `Task` struct (serialized as `note:<slug>` in the task metadata comment). Attaching a note therefore requires loading the task file, mutating the matching task's `note` field, and writing it back — a cross-module operation (`storage::load` + `storage::save` + `note` module). Two entry points cover the use cases:
- `task note add <title> --task <id>`: create a new note and immediately link it to the given task in one command
- `task note link <slug> <task-id>`: attach an existing note to a task; also serves as re-link (overwrites any existing link on that task)

A `task note unlink <task-id>` command clears the `note` field (sets to `None`), so Cowork can detach without deleting the note.

Alternative considered: put linking in a separate `task edit --note <slug>` command. Rejected — `task edit` is not in scope for this change and note linking belongs with the `note` subcommand group.

## Risks / Trade-offs

`note edit --body` replaces the entire body — there is no append mode. → Acceptable for now; users can `note show` first to copy the body.

`note add` does not open an editor, so creating a note with content requires two commands. → Mitigated by Cowork being the primary user; it will chain `add` + `edit`.

`note link` / `note unlink` mutate the task file — this is a cross-module write from within the `note` subcommand. → Use the same `storage::load` / `storage::save` path already used everywhere; no new locking complexity.

Deleting a note with `note rm` does not automatically clear the `note` field on any tasks that reference it — the task will have a dangling slug. → Acceptable; no referential integrity enforcement is in scope for this change.

## Migration Plan

Additive change only. No existing commands change behavior. No data migration needed.

## Open Questions

None.
