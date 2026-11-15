# Project Metadata Design

## Overview

A project is currently just a name: the `projects` table has a single `name`
column and tasks reference it by string (`tasks.project`). This spec turns
projects into real entities with metadata, editable from the CLI and web UI,
without changing how tasks reference them.

## Decisions already made (confirmed with the user)

- **Fields in v1:** `description`, `status`, `color`, `default_agent`, `due`.
- **Archive/done only hides the project.** It disappears from the picker and
  default filters. Its tasks are untouched: they stay in the queue, in
  `task next`, and in search.
- **Rename is supported** and cascades atomically to every task using the old
  name (same transactional pattern as `delete_name`).
- **Name stays the key.** Tasks keep referencing projects by name; no task
  schema change, and `--project` keeps working everywhere.

Out of scope: per-project dashboard, vault/working-directory links,
hierarchy/sub-projects, TUI parity.

## Data model

Migration adds columns to `projects` (`ALTER TABLE ... ADD COLUMN`, guarded so
it is idempotent, in `open_migrated`):

| Column | Type | Notes |
|---|---|---|
| `description` | TEXT, nullable | Free text |
| `status` | TEXT NOT NULL DEFAULT `'active'` | `active`, `on-hold`, `done`, `archived` |
| `color` | TEXT, nullable | One of a fixed palette name, or `#rrggbb` |
| `default_agent` | TEXT, nullable | Matched ignoring case, like task agents |
| `due` | TEXT, nullable | Strict `YYYY-MM-DD` (weekday names and past-date rejection from task due dates do not apply) |
| `created` / `updated` | TEXT | Set on create / every edit |

Existing rows backfill to `status = 'active'`, other fields null.

New `ProjectStatus` enum in `src/task.rs` (Display/FromStr, kebab-case serde),
modelled on `WorkStatus`. New `Project` struct in `src/db.rs`.
`Registry::Tag` stays name-only; project functions (`list_projects`,
`get_project`, `update_project`, `rename_project`) sit beside the
`Registry`-based name helpers. Every write bumps the data version
(`PRAGMA user_version`) as `create_name` does.

## API

- `GET /api/projects` unchanged: array of name strings (compatible with the
  picker and tests). Archived projects are excluded unless `?all=1`.
- `GET /api/projects?detail=1` returns full `Project` objects.
- `GET /api/projects/:name` returns one project.
- `POST /api/projects` accepts `{name, ...optional metadata}`.
- `PATCH /api/projects/:name` updates any metadata field; `name` in the body
  renames and cascades to tasks.
- `DELETE /api/projects/:name` unchanged.

Validation errors return 400 with the message (invalid status, color, date).

## CLI

New `task project` group in `src/cli.rs` / `src/commands.rs`:

```
task project list [--all] [--json]
task project show <name>
task project add <name> [--description ..] [--status ..] [--color ..] [--agent ..] [--due ..]
task project edit <name> [same flags] [--rename <new>]
task project delete <name>
```

`task add --project X` on an unregistered name keeps auto-registering it with
defaults. When `--agent` is omitted and the project has a `default_agent`,
`task add` uses it. `task edit` never overrides an agent already set.

## Web UI

- `ProjectPicker` receives project objects: color dot per row, archived hidden
  (with a "show archived" toggle), status badge for `on-hold`/`done`.
- An "Edit project" panel opened from the picker edits all fields, including
  rename and archive.
- The queue's project chip and the group headers (when grouped by project) show the color dot. `filters.ts` still groups by name.
- Picking a project for a task that has no agent assigns the project's `default_agent`; an agent already set is kept.
- `api.ts` / `types.ts` gain a `Project` type and detail/patch calls;
  `fakeApi.ts` is updated to match.

## Docs

Per `CLAUDE.md`, update `AGENTS.md` for the new `task project` commands, the
`default_agent` behaviour, and the fact that archived projects still surface
their tasks to agents.

## Testing

- `db.rs`: migration on an old schema, update, rename cascade (tasks follow),
  case-insensitive collisions on rename, status/date/color validation.
- `commands.rs`: each `project` subcommand, default-agent fill on `add`.
- `server.rs`: GET (names, detail, `all`), POST, PATCH, rename, 400s.
- Web: picker hides archived, shows color, default-agent prefill
  (`vitest`, following `NextQueue.test.tsx`).

## Resolved

- Colors: a fixed palette of names plus free `#rrggbb`.
- A project is marked `done` manually only.
