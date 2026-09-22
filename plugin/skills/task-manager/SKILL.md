---
name: task-manager
description: Read and edit the user's task list. Use this skill when the user wants to list, view, add, edit, complete, reopen, or delete tasks.
---

The user's tasks are stored in a SQLite database at:

```
~/Documents/Mark-main/Tasks/tasks.db
```

Use the `task` CLI for every task operation — never open or edit `tasks.db` directly, it is a database file, not a text file. For note operations, use the `task note` CLI subcommands documented below.

## Finding Your Tasks

Tasks can be assigned to specific AI agents via the `agent` field in task metadata. To find tasks assigned to you:

1. Read the config file at `~/Library/Application Support/task-manager/config.md` (macOS) or `~/.config/task-manager/config.md` (Linux)
2. Find all lines starting with `agent-` — these define named agent profiles and their working directories, e.g. `agent-command-center: ~/code/command-center`
3. Expand tildes in directory paths (replace `~` with your home directory)
4. Find the profile whose directory is a prefix of your current working directory — use the longest match if multiple profiles match
5. Run `task list --agent <your-profile-name> --status open` to see the tasks assigned to you

Only work on tasks assigned to your agent profile. Tasks with `agent:human` are for the human. Tasks with no `agent` field are unassigned — do not work on these unless explicitly told to.

## Listing Tasks

```
task list [--status open|done] [--agent <name>] [--project <name>] [--tag <tag>] [--due-before <YYYY-MM-DD>]
```

With no flags, lists all tasks. Output is one line per task: status, id, priority, due date, title.

## Viewing a Task

```
task show <id>
```

Prints full detail: title, priority, tags, due date, project, recurrence, notes, agent, effort, created/updated timestamps, and description.

## Adding a Task

```
task add "<title>" [--priority critical|high|medium|low] [--due <YYYY-MM-DD or weekday>] [--project <name>] [--tags <a,b,c>] [--agent <name>] [--description "<text>"]
```

Only `<title>` is required; `--priority` defaults to `medium`. The id is assigned automatically and printed in the output (`Created task 12: <title>`).

## Editing a Task

```
task edit <id> [--title "<new title>"] [--priority <p>] [--due <date>] [--project <name>] [--tags <a,b,c>] [--agent <name>] [--description "<text>"] [--effort high|medium|low] [--work-status <status>] [--recur <pattern>]
```

Only the fields you pass are changed; `updated` is set automatically.

**`--work-status`** is separate from open/done status (see "Reporting Progress" below) — it does not mark the task done and does not affect `task done`/`task reopen`.

**`--recur`** sets or changes the task's recurrence pattern (see "Recurrence" under Valid Field Values below). When a recurring task is completed via `task done`, the newly spawned occurrence inherits the same `agent` as the completed task.

## Reporting Progress

Tasks have a `work_status` field, independent of open/done, so the human can see where an agent is on a task without the task being marked done:

```
task edit <id> --work-status in-progress
```

Valid values: `todo`, `in-progress`, `waiting-for-review`, `changes-requested`, `complete` (aliases `to-do`, `in_progress`/`inprogress`, `review`, `changes`, `done` are also accepted).

**Setting `work_status` to `complete` does NOT mark the task done.** It's purely informational. Never substitute it for `task done <id>` — see the `work-agent-tasks` skill for when to set each value while working assigned tasks.

**`changes-requested`** means a human left feedback on the task's review thread (see "Review Threads" below) and it needs another round of work — this value is set automatically by the web UI when feedback is posted, not something you normally set by hand.

## Completing a Task

```
task done <id>
```

If the task has a recurrence set, this automatically creates the next occurrence as a new open task and reports its id. Running `done` on an already-done task is a no-op (prints a message, doesn't error).

## Reopening a Task

```
task reopen <id>
```

## Deleting a Task

```
task rm <id>
```

## Valid Field Values

**Priority:** `critical`, `high`, `medium` (default), `low`

**Effort:** `high`, `medium`, `low`

**Tags:** comma-separated, e.g. `--tags frontend,api-v2`

**Due date:** `YYYY-MM-DD`, or a weekday name/abbreviation (resolves to the next future occurrence) — e.g. `--due 2026-03-25` or `--due friday`

**Work status:** `todo`, `in-progress`, `waiting-for-review`, `changes-requested`, `complete` — set via `--work-status` (see "Reporting Progress" above). Independent of open/done.

**Recurrence:** set via `--recur <pattern>`. Valid patterns: `daily`, `weekly`, `monthly`, `yearly` (every occurrence); `daily:N`, `weekly:N`, `monthly:N`, `yearly:N` (every N); `weekly:DAY` (e.g. `weekly:fri`); `weekly:N:DAY` (every N weeks on DAY); `monthly:N:DAY` (the Nth DAY of the month, e.g. `monthly:2:mon` for the 2nd Monday). `DAY` is a 3-letter abbreviation (`mon`, `tue`, `wed`, `thu`, `fri`, `sat`, `sun`).

## Notes

Notes are markdown files stored in the same directory as `tasks.db`. Each note has a slug (derived from its title) and is stored as `<slug>.md`. Notes are unaffected by the SQLite migration — they remain plain `.md` files, managed with the `task note` CLI subcommands below.

### Commands

| Command | Description | Output |
|---------|-------------|--------|
| `task note list` | List all notes | `<slug>  <title>` per line, sorted by slug |
| `task note add "<title>"` | Create a new note with empty body | File path of created note |
| `task note add "<title>" --task <id>` | Create a note and link it to a task | File path (links `notes` field on the task) |
| `task note show <slug>` | Print the note's title and body | Raw markdown content |
| `task note edit <slug> --title "<new title>"` | Update the note's title | File path |
| `task note edit <slug> --body "<new body>"` | Replace the note's body | File path |
| `task note edit <slug> --title "..." --body "..."` | Update both title and body | File path |
| `task note append <slug> --body "<text>"` | Append a section to the note's existing body (blank-line separated) | File path |
| `task note rm <slug>` | Delete the note file | Confirmation message |
| `task note link <slug> <task-id>` | Link an existing note to a task | Confirmation message |
| `task note unlink <task-id>` | Remove the note link from a task | Confirmation message |

### Notes

- `task note append` requires the note to already exist — it never creates one (errors if the slug isn't found)
- `task note edit` requires at least one of `--title` or `--body`; omitting both is an error
- `task note add --task <id>` creates the note even if the task is not found, but exits with code 1 and prints a warning
- `task note rm` does not automatically clear the `notes` field on tasks that referenced the deleted note
- `task note unlink` is idempotent — succeeds even if the task has no note linked
- The `--file` flag (global) can be used to target a different task database: `task --file /path/to/tasks.db note list`

## Review Threads

When a human gives feedback on a task you completed, it arrives as a note titled `Review — <task title>` with a deterministic slug: `task-<id>-review-thread`. This note is a running, append-only markdown log — the human's feedback and your responses interleave in it as a single conversation, oldest first.

**To read the full history before acting on feedback:**

```
task note show task-<id>-review-thread
```

Always read the whole thread, not just the newest section — earlier rounds carry context (what was already tried, what was already explained) that matters for getting the next round right.

**To respond after doing the requested work:**

```
task note append task-<id>-review-thread --body "## Agent response — <YYYY-MM-DD>

<summary of what changed>"
```

Then set `task edit <id> --work-status waiting-for-review` again — see `work-agent-tasks` for the full loop.

You'll only ever see a `task-<id>-review-thread` note on a task whose `work_status` is `changes-requested`; the note itself is created automatically by the web UI the first time a human leaves feedback, not by you.

## Rules to Never Break

- **Never guess or hand-construct a task id** — always use the id the CLI reports back to you (from `add`, `list`, or `show`).
- **Never edit `tasks.db` with a text editor or by hand** — it's a SQLite database, not a text file; use the CLI for every read and write.
- **Never treat `--work-status complete` as equivalent to `task done`** — they're independent fields. Setting `work_status` to `complete` does not close the task.
