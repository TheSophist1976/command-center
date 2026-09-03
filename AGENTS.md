# Tasks File — AI Instructions

**Before working with tasks, read this file in full.**

This document describes how to work with tasks via the `task` CLI (tasks are stored in a SQLite database, `tasks.db`), and how to find the tasks assigned to you.

---

## Finding Your Agent Profile

Tasks can be assigned to specific AI agents using the `agent` field in the task metadata. To find tasks assigned to you:

1. **Read the config file** at `~/Library/Application Support/task-manager/config.md` (macOS) or `~/.config/task-manager/config.md` (Linux)
2. **Find all `agent-*` entries** — each defines a named agent profile and its working directory:
   ```
   agent-command-center: ~/code/command-center
   agent-itential: ~/code/itential
   ```
3. **Expand tildes** in directory paths (replace `~` with your home directory)
4. **Find the profile whose directory is a prefix of your current working directory** — use the longest match if multiple profiles match
5. **Filter tasks to those assigned to you** by running `task list --agent <your-profile-name>`

**Example**: If your CWD is `/Users/mark/code/command-center/src` and a profile exists with dir `/Users/mark/code/command-center`, your agent name is `command-center`. Work only on tasks with `agent:command-center` in their metadata.

**Tasks with `agent:human`** are for the human and should not be worked on by AI agents.

**Tasks with no `agent` field** are unassigned — do not work on these unless explicitly instructed.

---

## Reading Your Instructions

Each agent may have a set of operating instructions written by the human. **Read these at the start of every session before doing any work.**

Instructions are stored as a markdown note at:
```
<task-dir>/Notes/Agents/<your-agent-name>/instructions.md
```

Where `<task-dir>` is the directory containing `tasks.md` (resolved via `default-dir` in config or the file's parent directory).

**To read your instructions:**
```bash
task agent instructions <your-agent-name> show
```

Example (if your agent name is `command-center`):
```bash
task agent instructions command-center show
```

If no instructions file exists, the command prints "No instructions found." and you should proceed without them.

**The human can create or update your instructions with:**
```bash
task agent instructions <name> edit --title "My Agent Instructions" --body "Focus on..."
```

---

## Reading and Updating Memory

Each agent has a persistent memory file that accumulates learned patterns and preferences across sessions. **Read your memory at the start of every session after reading your instructions.**

Memory is stored at:
```
<task-dir>/Notes/Agents/<your-agent-name>/memory.md
```

**To read your memory:**
```bash
task agent memory <your-agent-name> show
```

If no memory file exists, the command prints "No memory found." and you should proceed without it — an empty memory is not an error.

**When to update memory** (after completing tasks):
- A preference or pattern has been observed at least twice
- A standing fact is established (recurring contact, project preference, known constraint)
- A past mistake is being corrected

**What NOT to store in memory:**
- One-off task details (those belong in the task note)
- Information specific to a single task that won't recur
- Anything likely to change within the current sprint

**To update your memory:**
```bash
task agent memory <name> edit --body "<updated content>"
```

**Suggested memory sections:** `## Preferences`, `## Patterns`, `## Standing Context`

---

## TUI Auto-Filter

When `task-tui` is launched from your project directory, it automatically applies a filter showing only tasks assigned to your agent. You will see `filter: agent:<name>` in the header. Press `Esc` to clear the filter and see all tasks.

---

## Working with Tasks

Tasks are stored in a SQLite database (`tasks.db` in your task directory), not a text file. Use the `task` CLI for every operation — do not attempt to open or edit `tasks.db` directly.

### Finding your tasks

```bash
task --file <task-dir>/tasks.db list --agent <your-agent-name> --status open
```

(Or, if you're running from inside your agent's working directory and it's already configured as `default-dir` for your profile, you can omit `--file`.)

### Adding a task

```bash
task add "Task title" --priority high --due 2026-03-15 --project Work --tags frontend,auth --agent command-center --description "Optional longer description"
```

Only `title` is required; `--priority` defaults to `medium`. The id is assigned automatically — it will be printed in the output (`Created task 12: Task title`).

### Editing a task

```bash
task edit <id> --priority critical --due 2026-04-01
```

Only the fields you pass are changed. `updated` is set automatically.

### Reporting your progress

Tasks have a `work_status` field, separate from open/done, so the human can track where you are on a task without you having to mark it done:

```bash
task edit <id> --work-status in-progress
```

Valid values: `todo`, `in-progress`, `waiting-for-review`, `complete` (aliases `to-do`, `in_progress`/`inprogress`, `review`, and `done` are also accepted).

**Update this as you work**, not just at the end — set `in-progress` when you start, `waiting-for-review` when you've finished and want the human to check your work before it's truly done.

**`work_status: complete` does NOT mark the task done.** It's purely informational — the human still explicitly runs `task done <id>` to close the task out (see below). Don't skip `task done` because you set `work_status` to `complete`.

### Completing a task

```bash
task done <id>
```

If the task has a recurrence set, completing it automatically creates the next occurrence as a new open task and reports its id.

### Reopening a task

```bash
task reopen <id>
```

### Deleting a task

```bash
task rm <id>
```

### Viewing full task detail

```bash
task show <id>
```

### Rules to Never Break

- **Never guess or hand-construct a task id** — always use the id the CLI reports back to you (from `add`, `list`, or `show`).
- **Never edit `tasks.db` with a text editor or by hand** — it's a SQLite database, not a text file; use the CLI for every read and write.
- **Never treat `--work-status complete` as equivalent to `task done`** — they're independent fields. Set `work_status` to keep the human informed; still run `task done <id>` yourself when appropriate, or leave that to the human if that's their preferred workflow.
