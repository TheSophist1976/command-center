---
name: task-manager
description: Read and edit the user's task list. Use this skill when the user wants to list, view, add, edit, complete, reopen, or delete tasks.
---

The user's tasks are stored in a markdown file at:

```
~/Documents/Mark-main/Tasks/tasks.md
```

Read and edit this file directly for task operations. For note operations, use the `task note` CLI subcommands documented below.

## File Format

Each task is a single line with a GitHub-style checkbox:

```
- [ ] id:42 priority:high [tag1,tag2] due:2026-03-25 project:Work | Task title here | created:2026-01-01T00:00:00Z updated:2026-01-01T00:00:00Z
```

- `[ ]` = open task, `[x]` = done
- Fields before `|`: `id:`, `priority:`, tags in `[...]`, `due:`, `project:` (all optional except `id:`)
- Title follows the first ` | `
- Timestamps follow the second ` | `

## Reading Tasks

Read the file and parse each `- [ ]` or `- [x]` line. Present ID, status, priority, title, and due date.

## Adding a Task

Append a new line:
```
- [ ] id:<next_id> priority:medium | <title> | created:<ISO timestamp>
```
Use the highest existing `id:` + 1 for `<next_id>`.

## Editing a Task

Find the line with the matching `id:` and update the relevant field in place. Always update the `updated:` timestamp.

## Completing a Task

Change `[ ]` to `[x]` on the matching line. Update `updated:` timestamp.

## Reopening a Task

Change `[x]` to `[ ]` on the matching line. Update `updated:` timestamp.

## Deleting a Task

Remove the line with the matching `id:` entirely.

## Valid Field Values

**Priority:** `critical`, `high`, `medium` (default), `low`

**Tags:** comma-separated inside `[...]`, lowercase alphanumeric and hyphens — e.g., `[frontend,api-v2]`

**Due date:** `YYYY-MM-DD` — e.g., `due:2026-03-25`

**Timestamps:** ISO 8601 — e.g., `2026-03-18T12:00:00Z`

## Notes

Notes are markdown files stored in the same directory as `tasks.md`. Each note has a slug (derived from its title) and is stored as `<slug>.md`. Use the `task note` CLI subcommands to manage notes.

### Commands

| Command | Description | Output |
|---------|-------------|--------|
| `task note list` | List all notes | `<slug>  <title>` per line, sorted by slug |
| `task note add "<title>"` | Create a new note with empty body | File path of created note |
| `task note add "<title>" --task <id>` | Create a note and link it to a task | File path (links `note` field on the task) |
| `task note show <slug>` | Print the note's title and body | Raw markdown content |
| `task note edit <slug> --title "<new title>"` | Update the note's title | File path |
| `task note edit <slug> --body "<new body>"` | Replace the note's body | File path |
| `task note edit <slug> --title "..." --body "..."` | Update both title and body | File path |
| `task note rm <slug>` | Delete the note file | Confirmation message |
| `task note link <slug> <task-id>` | Link an existing note to a task | Confirmation message |
| `task note unlink <task-id>` | Remove the note link from a task | Confirmation message |

### Notes

- `task note edit` requires at least one of `--title` or `--body`; omitting both is an error
- `task note add --task <id>` creates the note even if the task is not found, but exits with code 1 and prints a warning
- `task note rm` does not automatically clear the `note` field on tasks that referenced the deleted note
- `task note unlink` is idempotent — succeeds even if the task has no note linked
- The `--file` flag (global) can be used to target a different task file: `task --file /path/to/tasks.md note list`
