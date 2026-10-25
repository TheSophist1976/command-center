---
name: task-manager
description: Read and edit the user's task list. Use this skill when the user wants to list, view, add, edit, complete, reopen, or delete tasks.
---

The user's tasks are stored in a SQLite database, `tasks.db`. The `task` CLI finds it for you: the `default-dir` setting in the config file (written by `task setup`), or the `TASK_FILE` environment variable, or the global `--file <path>` flag, in that order of increasing priority.

Use the `task` CLI for every task operation — never open or edit `tasks.db` directly, it is a database file, not a text file. For note operations, use the `task note` CLI subcommands documented below.

Every `task` command that writes is a single atomic transaction, so several agents can run `task` commands at the same time without losing each other's changes.

## Finding Your Tasks

Tasks can be assigned to specific AI agents via the `agent` field in task metadata. Agent names are matched ignoring case.

1. Make sure you know who you are: `TASK_AGENT=<profile-name>` is set in your environment, or you are running inside your profile's working directory. Check with `task whoami`.
2. Ask for your work queue:
   ```
   task next            # one line per task, in the order to work them
   task next --json     # full queue: your instructions, memory, tasks, review threads, linked notes
   ```
   `task next` already skips tasks that were handed back (`waiting-for-review`), tasks blocked on the human (`needs-input`), and recurring tasks that are not due yet. Each task in the JSON has a `pickup` of `new`, `continue`, `review_feedback` or `answered_question`, telling you why it is there.

Only work on tasks assigned to your agent profile. Tasks with `agent:human` are for the human. Tasks with no `agent` field are unassigned — do not work on these unless explicitly told to.

## Listing Tasks

```
task list [--status open|done] [--agent <name>] [--project <name>] [--tag <tag>] [--due-before <YYYY-MM-DD>]
```

With no flags, lists all tasks. Output is one line per task: status, id, priority, due date, title. Services are never listed here; use `task list --services` or `task service list`.

## Services

A service is a recurring agent job that needs no review (`kind: service`). It must have a recurrence and a due date, and it keeps one task id for good. When `task next` returns one with `pickup: service_run`, do the work and then record it:

```
task service run <id> --result ok|failed [--summary "<text>"]
```

This logs the run and moves the due date to the next scheduled date after today, skipping missed dates. A `failed` result sets `work_status` to `failing`, and the next `ok` run clears it. Never run `task done` (refused), set `waiting-for-review` or edit `--due` on a service. Other commands: `task service add "<title>" --recur <pattern> [--due <date>] [--agent <name>] [--instructions "<text>"]`, `task service list|show <id>|pause <id>|resume <id>|runs <id>`, and `task edit <id> --service|--no-service`.

## Viewing a Task

```
task show <id>
```

Prints full detail: title, priority, tags, due date, project, recurrence, notes, agent, effort, created/updated timestamps, and description, followed by a `## Instructions` section when the task has instructions.

## Adding a Task

```
task add "<title>" [--priority critical|high|medium|low] [--due <YYYY-MM-DD or weekday>] [--project <name>] [--tags <a,b,c>] [--agent <name>] [--description "<text>"] [--instructions "<text>"]
```

Only `<title>` is required; `--priority` defaults to `medium`. The id is assigned automatically and printed in the output (`Created task 12: <title>`).

`--instructions` is what an agent or person should do for this task, kept separate from `--description` (what the task is about).

## Editing a Task

```
task edit <id> [--title "<new title>"] [--priority <p>] [--due <date>] [--project <name>] [--tags <a,b,c>] [--agent <name>] [--description "<text>"] [--instructions "<text>"] [--effort high|medium|low] [--work-status <status>] [--recur <pattern>]
```

Only the fields you pass are changed; `updated` is set automatically. `--instructions "<text>"` replaces the task's instructions; `--instructions ""` clears them.

**`--work-status`** is separate from open/done status (see "Reporting Progress" below) — it does not mark the task done and does not affect `task done`/`task reopen`.

**`--recur`** sets or changes the task's recurrence pattern (see "Recurrence" under Valid Field Values below). When a recurring task is completed via `task done`, the newly spawned occurrence inherits the same `agent` and `instructions` as the completed task.

## Reporting Progress

Tasks have a `work_status` field, independent of open/done, so the human can see where an agent is on a task without the task being marked done:

```
task edit <id> --work-status in-progress
```

Valid values: `todo`, `in-progress`, `waiting-for-review`, `changes-requested`, `needs-input` (aliases `to-do`, `in_progress`/`inprogress`, `review`, `changes`, and `needs_input` are also accepted; `complete` and `done` are retired aliases for `waiting-for-review`).

**Every state change goes through the lifecycle rules, and an invalid move is refused with an error.** With `TASK_AGENT` set you are the agent:

- You may set `in-progress`, `needs-input` and `waiting-for-review` on tasks assigned to an agent. A task assigned to `human`, or to nobody, is the human's: you may not start, ask on or submit it.
- Submitting (`waiting-for-review`) needs a linked note.
- You may not run `task done` or `task reopen`, reset a task to `todo`, convert a task to or from a service, pause or resume a service, or reassign a task to `human`. `--force` is for humans only.
- `changes-requested` is set only by the human's feedback or answer, and `failing` only by `task service run <id> --result failed`.
- A service cannot be put in `needs-input` or `waiting-for-review`. If a service run cannot be done, record it with `task service run <id> --result failed --summary "<why>"`.

A work status is never the same as `task done`: only the human closes a task.

**`changes-requested`** means a human left feedback on the task's review thread (see "Review Threads" below) and it needs another round of work — this value is set automatically by the web UI when feedback is posted, not something you normally set by hand. It is also set when the human answers one of your questions (see "Asking a Question" below).

**`needs-input`** means you are blocked on a question for the human (see "Asking a Question" below).

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

**Work status:** `todo`, `in-progress`, `waiting-for-review`, `changes-requested`, `needs-input`, and `failing` (services only) — set via `--work-status` (see "Reporting Progress" above). Independent of open/done.

**Recurrence:** set via `--recur <pattern>`. Valid patterns: `daily`, `weekly`, `monthly`, `yearly` (every occurrence); `daily:N`, `weekly:N`, `monthly:N`, `yearly:N` (every N); `weekly:DAY` (e.g. `weekly:fri`); `weekly:N:DAY` (every N weeks on DAY); `monthly:N:DAY` (the Nth DAY of the month, e.g. `monthly:2:mon` for the 2nd Monday). `DAY` is a 3-letter abbreviation (`mon`, `tue`, `wed`, `thu`, `fri`, `sat`, `sun`).

## Writing Text for the Web UI

The web UI renders every free-text field as GitHub-flavoured markdown: task titles (inline only: bold, italics, `code`, links), descriptions, instructions, note bodies, question notes, review-thread messages and service run summaries. Write all of them in markdown:

- Keep it short and scannable: a one-line summary first, then short paragraphs or bullet lists.
- Use **bold** labels or `###` headings for sections. In review threads and question notes, never start a line with `## ` except the required `## Agent response — <date>` heading; the UI splits review threads into chat messages on `## ` lines.
- Put commands, paths, ids, branch names and slugs in `inline code`, and multi-line commands or output in fenced code blocks with a language (```` ```sh ````). The UI adds a copy button to every code block and to every description, instructions field and note.
- Write links as `[PR #42](https://github.com/...)` or paste the bare URL; both are clickable. Never wrap a URL in backticks, which makes it unclickable.
- Use `- [ ]` / `- [x]` task lists for checklists and tables only for real tabular data.

See `docs/markdown-style-guide.md` in the command-center repo for the full guide with examples.

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

## Asking a Question

When you are blocked on something only the human can answer, put the question where the web UI can show it, so it is not lost in a terminal nobody is watching:

```
task note show task-<id>-question 2>/dev/null   # does a question note already exist?
```

If it does not exist, create it. `task note add` derives the slug from the title, so the title must be exactly `Task <id> Question` for the slug to be `task-<id>-question`:

```
task note add "Task <id> Question" --task <id>
```

Then set the question text (`--body` replaces the whole body, which is right because only one question is active at a time) and flag the task:

```
task note edit task-<id>-question --body "<the question>"
task edit <id> --work-status needs-input
```

When the human answers from the web UI, an `## Answer — <YYYY-MM-DD>` section is appended to that note and the task's `work_status` becomes `changes-requested`. If you find a `changes-requested` task whose question note ends in an answer, read the note, treat it as the reply, set `--work-status in-progress`, and continue. Do not ask the same question again.

## Agent Instructions and Memory

Each agent profile can have standing instructions written by the human and a memory file it maintains itself. Read both at the start of a session, before working any task:

```
task agent instructions <agent-name> show
task agent memory <agent-name> show
```

Both print a "not found" message when empty; that is not an error. Update memory only for patterns seen at least twice, standing facts, or corrected mistakes — never for one-off task details:

```
task agent memory <agent-name> edit --body "<updated content>"
```

## Rules to Never Break

- **Never guess or hand-construct a task id** — always use the id the CLI reports back to you (from `add`, `list`, or `show`).
- **Never edit `tasks.db` with a text editor or by hand** — it's a SQLite database, not a text file; use the CLI for every read and write.
- **Never treat a work status as equivalent to `task done`** — they're independent fields. Marking a task done is the human's move, and the CLI refuses it when `TASK_AGENT` is set.
