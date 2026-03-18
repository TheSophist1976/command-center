---
name: task-manager
description: Read and edit the user's task list. Use this skill when the user wants to list, view, add, edit, complete, reopen, or delete tasks.
---

The user's tasks are stored in a markdown file at:

```
~/Documents/Mark-main/Tasks/tasks.md
```

Read and edit this file directly. The `task` CLI only supports `tui`, `auth`, and `config` — do NOT attempt to run `task list`, `task add`, or any other task CRUD commands.

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
