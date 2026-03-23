# task

A fast CLI task manager for developers and AI agents. Tasks are stored in a plain Markdown file that's human-readable, git-friendly, and easy for agents to parse.

## Build

Requires [Rust](https://rustup.rs/) (1.75+).

```sh
cargo build --release
```

The binary is at `target/release/task`. To install it system-wide:

```sh
cargo install --path .
```

## Quick Start

```sh
# Launch the interactive TUI (default command)
task

# Or explicitly
task tui

# Use a specific task file
task --file ~/projects/tasks.md
```

All task management (adding, editing, completing, deleting) is done through the interactive TUI.

## Commands

| Command | Description |
| --- | --- |
| `task` / `task tui` | Launch the interactive terminal UI (default) |
| `task config set <key> <value>` | Set a configuration value |
| `task config get <key>` | Get a configuration value |
| `task auth todoist [--token TOKEN]` | Store Todoist API token |
| `task auth claude [--key KEY]` | Store Claude API key |
| `task auth status` | Show authentication status |
| `task auth revoke` | Revoke stored tokens |
| `task import todoist [--test]` | Import open tasks from Todoist |

### Global Flags

| Flag | Description |
| --- | --- |
| `--file <path>` | Use a custom task file (default: `tasks.md`) |

## Interactive TUI

Launch with `task tui` or just `task`. The TUI provides a full-featured interface for managing tasks, notes, and AI-powered chat.

### Views

Cycle through views with `v` (next) and `V` (previous):

| View | Description |
| --- | --- |
| Today | Tasks due today, overdue, or with no due date |
| All Tasks | All tasks including completed |
| This Week | Tasks due within 7 days |
| This Month | Tasks due this month |
| This Year | Tasks due this year |
| No Due Date | Tasks without a due date |
| Recurring | Tasks with recurrence patterns |
| Notes | Markdown notes manager |

### Keybindings

**Navigation**

| Key | Action |
| --- | --- |
| `j` / `Down` | Move down |
| `k` / `Up` | Move up |
| `v` | Next view |
| `V` | Previous view |
| `Tab` | Toggle detail panel |
| `q` | Quit |

**Task Operations**

| Key | Action |
| --- | --- |
| `Space` / `Enter` | Toggle task status (open/done) |
| `a` | Add new task |
| `d` | Delete task (with confirmation) |
| `e` | Edit title |
| `p` | Edit priority (`c`=critical, `h`=high, `m`=medium, `l`=low) |
| `t` | Edit tags |
| `r` | Edit description |
| `R` | Edit recurrence pattern |
| `f` / `/` | Filter tasks |
| `Esc` | Clear active filter |

**Due Date Shortcuts**

| Key | Action |
| --- | --- |
| `T` | Set due today |
| `N` | Set due tomorrow |
| `W` | Set due next week |
| `M` | Set due next month |
| `Q` | Set due 3 months out |
| `Y` | Set due 1 year out |
| `X` | Clear due date |

**Notes**

| Key | Action |
| --- | --- |
| `n` | Open note picker (link/create note for selected task) |
| `g` | Open linked note in editor |

In Notes view: `a` to create a note, `Enter` to edit, `d` to delete.

In the note editor: `Ctrl+S` to save, `Esc` to exit (prompts if unsaved changes).

**Integrations**

| Key | Action |
| --- | --- |
| `:` | Open NLP chat with Claude |

### Filtering

Enter filter mode with `f` or `/`, then type a filter expression:

- `status:open` or `status:done`
- `priority:high` (also `critical`, `medium`, `low`)
- `tag:frontend`
- `project:myproject`
- `title:login` (case-insensitive substring match)

Multiple filters can be combined with spaces.

### Detail Panel

Press `Tab` to toggle a right-side panel showing all fields for the selected task. Navigate fields with `j`/`k`, press `Enter` to edit, `s` to save, `d` to discard.

## NLP Chat

Press `:` in the TUI to open a natural language chat powered by Claude. Requires a Claude API key (`task auth claude` or set `ANTHROPIC_API_KEY`).

Supported actions:

- **Filter** tasks by natural language ("show me high priority frontend tasks")
- **Update** tasks in bulk ("mark all infra tasks as done")
- **Set recurrence** ("make task 3 repeat weekly on Fridays")
- **Create notes** ("create a note about the deployment process")
- **Edit notes** ("update the deployment note with the new steps")
- **Fetch URLs** (reference a URL and the AI will summarize it)
- **Query tasks** ("what's overdue?", "what's due this week?")

## Todoist Import

Import open tasks from your Todoist account:

1. Authenticate: `task auth todoist --token YOUR_TOKEN`
2. Run `task import todoist`

Imported tasks preserve title, description, priority, labels (as tags), due dates, and project names. Imported tasks are labeled "exported" in Todoist to avoid duplicates. Use `--test` to import only the first 3 tasks without labeling them in Todoist.

## Task Features

### Priority Levels

Critical, High, Medium, Low. Shorthand in the TUI: `c`, `h`, `m`, `l`.

### Recurrence Patterns

Set with the `R` key in the TUI or via NLP chat.

| Pattern | Description |
| --- | --- |
| `daily` | Every day |
| `weekly` | Every week |
| `monthly` | Every month |
| `yearly` | Every year |
| `daily:N` | Every N days |
| `weekly:N` | Every N weeks |
| `monthly:N` | Every N months |
| `yearly:N` | Every N years |
| `weekly:MON` | Every week on Monday |
| `weekly:N:FRI` | Every N weeks on Friday |
| `monthly:2:TUE` | 2nd Tuesday of each month |

When a recurring task is completed, it automatically resets to open with the next due date.

### Notes

Markdown notes are stored as individual `.md` files alongside your task file. Notes can be standalone or linked to tasks.

- Create notes from the Notes view or via the note picker (`n`)
- Edit notes in the inline editor with line numbers
- Link notes to tasks — a note indicator appears in the task list

## Configuration

Config is stored at `~/.config/task-manager/config.md`.

| Key | Description |
| --- | --- |
| `default-dir` | Default directory for task/note files |
| `default-view` | Starting view (`today`, `all`, `weekly`, `monthly`, `yearly`, `no-due-date`, `recurring`, `notes`) |

```sh
task config set default-dir ~/projects
task config set default-view all
```

## File Path Resolution

The task file is resolved in this order:

1. `--file <path>` CLI flag
2. `TASK_FILE` environment variable
3. `default-dir` config value
4. `tasks.md` in the current directory

## File Format

Tasks are stored as Markdown with metadata in HTML comments:

```markdown
<!-- format:1 -->
<!-- next-id:4 -->

# Tasks

## [ ] Build the login page

<!-- id:1 priority:high tags:frontend,auth due:2026-03-15 created:2025-01-15T10:00:00+00:00 -->

Some notes about this task.

## [X] Set up CI pipeline

<!-- id:2 priority:medium tags:infra recur:weekly created:2025-01-10T08:00:00+00:00 -->

## [ ] Write deployment docs

<!-- id:3 priority:low note:deployment-guide created:2025-01-20T09:00:00+00:00 -->
```

Metadata keys: `id`, `priority`, `tags`, `due`, `recur`, `note`, `project`, `created`, `updated`.

The file is safe to edit by hand. The parser is tolerant of formatting issues — malformed entries are skipped rather than causing errors.

## Running Tests

```sh
cargo test
```

## License

MIT
