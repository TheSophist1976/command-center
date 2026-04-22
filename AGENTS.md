# Tasks File — AI Instructions

**Before modifying `tasks.md`, read this file in full.**

This document describes the exact format of `tasks.md`, the rules for safely reading and modifying it, and how to find the tasks assigned to you.

---

## Finding Your Tasks

Tasks can be assigned to specific AI agents using the `agent` field in the task metadata. To find tasks assigned to you:

1. **Read the config file** at `~/Library/Application Support/task-manager/config.md` (macOS) or `~/.config/task-manager/config.md` (Linux)
2. **Find all `agent-*` entries** — each defines a named agent profile and its working directory:
   ```
   agent-command-center: ~/code/command-center
   agent-itential: ~/code/itential
   ```
3. **Expand tildes** in directory paths (replace `~` with your home directory)
4. **Find the profile whose directory is a prefix of your current working directory** — use the longest match if multiple profiles match
5. **Filter `tasks.md`** to tasks where `agent:<your-profile-name>` appears in the metadata comment

**Example**: If your CWD is `/Users/mark/code/command-center/src` and a profile exists with dir `/Users/mark/code/command-center`, your agent name is `command-center`. Work only on tasks with `agent:command-center` in their metadata.

**Tasks with `agent:human`** are for the human and should not be worked on by AI agents.

**Tasks with no `agent` field** are unassigned — do not work on these unless explicitly instructed.

---

## Reading Your Instructions

Each agent may have a set of operating instructions written by the human. **Read these at the start of every session before doing any work.**

Instructions are stored as a markdown note at:
```
<task-dir>/Notes/Instructions/<your-agent-name>.md
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

## TUI Auto-Filter

When `task-tui` is launched from your project directory, it automatically applies a filter showing only tasks assigned to your agent. You will see `filter: agent:<name>` in the header. Press `Esc` to clear the filter and see all tasks.

---

---

## Step 1: Always Read First

Before making any changes, read the current contents of `tasks.md`. You need:

- The current `<!-- next-id:N -->` value to assign IDs to new tasks
- The existing task list to avoid ID conflicts and to find the task you want to modify

---

## File Format

`tasks.md` is a structured Markdown file. Do not add, remove, or reorder structural elements outside of task blocks.

### Header (first two lines)

```
<!-- format:2 -->
<!-- next-id:N -->
```

- `format:2` — fixed; do not change
- `next-id:N` — the next available task ID; **increment this when adding a task**

### Section heading

```

# Tasks
```

A single blank line precedes `# Tasks`. Do not change this heading.

### Task block

Each task occupies one heading, one metadata comment, and an optional description:

```
## [ ] Task title here
<!-- id:3 priority:high tags:frontend,auth due:2026-03-15 project:Work recur:weekly note:my-note created:2026-01-15T10:00:00+00:00 updated:2026-01-20T12:00:00+00:00 -->

Optional description paragraph(s) here.

```

- `## [ ]` = open task, `## [x]` = done task
- The metadata comment immediately follows the heading (blank lines between are tolerated but not written)
- Description is any text after the metadata comment and before the next `##` heading — may be omitted

### Metadata fields

| Field | Required | Format | Example |
|-------|----------|--------|---------|
| `id` | yes | integer | `id:3` |
| `priority` | yes | see below | `priority:high` |
| `tags` | no | comma-separated | `tags:frontend,auth` |
| `due` | no | `YYYY-MM-DD` | `due:2026-03-15` |
| `project` | no | URL-encoded string | `project:Work` |
| `recur` | no | see below | `recur:weekly` |
| `note` | no | note slug | `note:my-note` |
| `agent` | no | profile name or `human` | `agent:command-center` |
| `created` | yes | ISO 8601 | `created:2026-01-15T10:00:00+00:00` |
| `updated` | no | ISO 8601 | `updated:2026-01-20T12:00:00+00:00` |

**Priority values:** `critical`, `high`, `medium`, `low`

**Recurrence values:**

| Pattern | Meaning |
|---------|---------|
| `daily` | Every day |
| `weekly` | Every week |
| `monthly` | Every month |
| `yearly` | Every year |
| `daily:N` | Every N days |
| `weekly:N` | Every N weeks |
| `weekly:MON` | Every week on Monday (MON/TUE/WED/THU/FRI/SAT/SUN) |
| `weekly:N:FRI` | Every N weeks on Friday |
| `monthly:2:TUE` | 2nd Tuesday of each month |

**Project encoding:** spaces → `%20`, colons → `%3A`, percent signs → `%25`

---

## Rules for Each Operation

### Adding a task

1. Read `<!-- next-id:N -->` from the header
2. Create a new task block at the end of the file using `N` as the `id`
3. Increment `next-id` in the header: `<!-- next-id:N+1 -->`
4. Set `created` to the current UTC timestamp in ISO 8601 format
5. Do not set `updated` on a newly created task

```
## [ ] New task title
<!-- id:N priority:medium created:2026-03-23T10:00:00+00:00 -->
```

### Editing a task

1. Find the task by its `id` in the metadata comment
2. Update only the fields that need to change in the metadata comment
3. Add or update `updated` to the current UTC timestamp
4. Do not change the `id` or `created` fields

### Completing a task

1. Find the task by its `id`
2. Change `## [ ]` to `## [x]`
3. Add or update `updated` to the current UTC timestamp

### Reopening a task

1. Find the task by its `id`
2. Change `## [x]` to `## [ ]`
3. Add or update `updated` to the current UTC timestamp

### Deleting a task

1. Find the task by its `id`
2. Remove the entire task block: heading + metadata comment + description (if any)
3. **Do not** renumber remaining task IDs or change `next-id`

---

## Rules to Never Break

- **Never change an existing task's `id`**
- **Never reuse an `id`** — always use and increment `next-id`
- **Never change `format:2`** in the header
- **Never remove `next-id`** from the header
- **Never reorder the header lines** (`format` must come before `next-id`)
- **Only modify fields inside the metadata comment** — do not invent new HTML comment blocks
- **Preserve description text exactly** — do not reflow, reformat, or truncate it unless explicitly asked
