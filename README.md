# task

A fast CLI task manager for developers and AI agents. Tasks are stored in a plain Markdown file that's human-readable, git-friendly, and easy for agents to parse.

## Build

Requires [Rust](https://rustup.rs/) (1.75+).

```sh
# CLI only
cargo build --release

# CLI + TUI
cargo build --release --features tui
```

Two binaries are produced:
- `target/release/task` — CLI (auth, config, notes)
- `target/release/task-tui` — interactive terminal UI (requires `--features tui`)

## Quick Start

```sh
# Launch the interactive TUI
task-tui

# Use a specific task file
task-tui --file ~/projects/tasks.md
```

## CLI Commands

### Auth

| Command | Description |
| --- | --- |
| `task auth todoist [--token TOKEN]` | Store Todoist API token |
| `task auth status` | Show authentication status |
| `task auth revoke` | Revoke stored tokens |

### Config

| Command | Description |
| --- | --- |
| `task config set <key> <value>` | Set a configuration value |
| `task config get <key>` | Get a configuration value |

### Notes

Notes are stored in a `Notes/` subdirectory of the task file's directory.

| Command | Description |
| --- | --- |
| `task note list` | List all notes |
| `task note add "<title>"` | Create a note |
| `task note add "<title>" --task <id>` | Create a note and link to a task |
| `task note show <slug>` | Show a note's content |
| `task note edit <slug> --title "<title>"` | Edit a note's title |
| `task note edit <slug> --body "<body>"` | Edit a note's body |
| `task note rm <slug>` | Delete a note |
| `task note link <slug> <task-id>` | Link an existing note to a task |
| `task note unlink <task-id>` | Unlink the note from a task |

### Global Flags

| Flag | Description |
| --- | --- |
| `--file <path>` | Use a custom task file (default: `tasks.md` in current directory) |

## Interactive TUI

Launch with `task-tui`.

### Views

Cycle with `v` (forward) / `V` (backward):

| View | Description |
| --- | --- |
| **Due** | Tasks due in the current window (see below) — default view |
| **No Due Date** | Open tasks with no due date |
| **Recurring** | Tasks with a recurrence pattern |
| **Notes** | Markdown notes manager |

The **Due** view has a sub-window that controls which tasks are shown. Toggle with `]` (wider) and `[` (narrower):

| Window | Tasks shown |
| --- | --- |
| Day | Due today, overdue, incomplete (missing date or agent) |
| Week | Due this calendar week + overdue |
| Month | Due this month + overdue |
| Year | Due this year + overdue |
| All | All open tasks |

### Keybindings

**Navigation**

| Key | Action |
| --- | --- |
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `v` | Next view |
| `V` | Previous view |
| `]` | Expand Due window (Day→Week→Month→Year→All) |
| `[` | Shrink Due window |
| `Tab` | Toggle detail panel |
| `Esc` | Clear active filter / cancel |
| `q` | Quit |

**Task Operations**

| Key | Action |
| --- | --- |
| `Enter` / `Space` | Toggle task status (open ↔ done) |
| `a` | Add new task |
| `e` | Edit title |
| `p` | Edit priority (`c`=critical, `h`=high, `m`=medium, `l`=low) |
| `t` | Edit tags |
| `r` | Edit description |
| `R` | Edit recurrence pattern |
| `A` | Set agent |
| `f` / `/` | Filter tasks |
| `^r` | Reload tasks from disk |

**Due Date**

| Key | Action |
| --- | --- |
| `d` | Edit due date (type `YYYY-MM-DD`, empty to clear) |
| `T` | Set due today |
| `N` | Set due tomorrow |
| `W` | Set due next week (7 days) |
| `M` | Set due next month |
| `Q` | Set due 3 months out |
| `Y` | Set due 1 year out |
| `X` | Clear due date |

**Grouping**

| Key | Action |
| --- | --- |
| `G` | Cycle grouping: none → project → agent → priority → none |
| `:group <field>` | Set grouping explicitly (`agent`, `project`, `priority`, `none`) |

**Notes**

| Key | Action |
| --- | --- |
| `n` | Open note picker (link or create note for selected task) |
| `g` | Open linked note in editor |

In the **Notes view**: `a` to create, `Enter` to edit, `d` to delete (with confirmation).

In the **note editor**: `Ctrl+S` to save, `Esc` to exit (prompts if unsaved).

**Integrations**

| Key | Action |
| --- | --- |
| `C` | Open Claude session browser for selected task |
| `i` | Import tasks from Todoist (background) |
| `S` | Sync Slack inbox (background) |
| `D` | Set default task directory |

### Filtering

Press `f` or `/` to enter filter mode:

| Expression | Description |
| --- | --- |
| `status:open` / `status:done` | Filter by status |
| `priority:high` | Filter by priority (`critical`, `high`, `medium`, `low`) |
| `tag:<name>` | Filter by tag |
| `project:<name>` | Filter by project |
| `title:<text>` | Case-insensitive title match |

Multiple filters can be combined with spaces. Press `Esc` in normal mode to clear.

### Status Indicators

| Indicator | Meaning |
| --- | --- |
| `[ ]` | Open task |
| `[x]` | Done task (greyed out) |
| `[!]` | Overdue (red) |
| `[?]` | Incomplete — missing due date or agent (amber) |

### Detail Panel

Press `Tab` to toggle a right-side panel showing all task fields. Navigate fields with `j`/`k`, press `Enter` to edit inline, `s` to save, `d` to discard.

### Grouping

Press `G` to cycle through groupings, or use `:group <field>` in command mode. Group headings show the field name and value. Navigation respects group order. The active grouping is saved to config and restored on launch.

## Task Operations

### Deleting a Task

Task deletion is done via the CLI. There is no delete key in the TUI.

```sh
# Delete by editing the tasks.md file directly, or use task delete if available
```

### Priority Levels

`critical` > `high` > `medium` > `low`. Set in TUI with `p` then `c`/`h`/`m`/`l`.

### Recurrence Patterns

Set with the `R` key in the TUI.

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
| `weekly:MON` | Every Monday |
| `weekly:N:FRI` | Every N weeks on Friday |
| `monthly:2:TUE` | 2nd Tuesday of each month |

When a recurring task is completed it automatically reopens with the next due date.

### Notes

Notes are stored as `.md` files in a `Notes/` subdirectory alongside the task file. Notes can be standalone or linked to tasks.

- Create from the Notes view (`a`) or via the note picker (`n`) from any task
- Edit in the built-in editor with line numbers
- Link to tasks — linked tasks show a note indicator in the task list

## Configuration

Config is stored at `~/Library/Application Support/task-manager/config.md` (macOS) or `~/.config/task-manager/config.md` (Linux).

| Key | Description |
| --- | --- |
| `default-dir` | Default directory for task and note files |
| `default-view` | Starting view: `due`, `no-due-date`, `recurring`, `notes` |
| `group-by` | Default grouping: `agent`, `project`, `priority`, or `none` |
| `columns` | Comma-separated column list: `id,status,priority,title,due,agent,tags,project` |
| `agent-<name>` | Agent profile — maps a name to a project directory |

```sh
task config set default-dir ~/projects
task config set default-view due
task config set group-by agent
```

### Agent Profiles

Agent profiles associate a name with a project directory. When the TUI is launched from within that directory, the agent name is used to filter tasks assigned to that agent.

```
agent-myapp: ~/code/myapp
agent-work: ~/code/work-project
```

The most specific (longest-matching) directory wins when multiple profiles overlap.

## File Path Resolution

The task file is resolved in this order:

1. `--file <path>` CLI flag
2. `TASK_FILE` environment variable
3. `default-dir` config value → `<dir>/tasks.md`
4. `tasks.md` in the current directory

## File Format

Tasks are stored as Markdown with metadata in HTML comments:

```markdown
<!-- format:1 -->
<!-- next-id:4 -->

# Tasks

## [ ] Build the login page

<!-- id:1 priority:high tags:frontend,auth due:2026-03-15 agent:myapp created:2025-01-15T10:00:00Z updated:2025-01-16T09:00:00Z -->

## [x] Set up CI pipeline

<!-- id:2 priority:medium tags:infra recur:weekly created:2025-01-10T08:00:00Z -->

## [ ] Write deployment docs

<!-- id:3 priority:low note:deployment-guide created:2025-01-20T09:00:00Z -->
```

Metadata fields: `id`, `priority`, `tags`, `due`, `recur`, `note`, `project`, `agent`, `created`, `updated`.

The file is safe to edit by hand. The parser is tolerant of formatting issues — malformed entries are skipped rather than causing errors.

## Running Tests

```sh
cargo test --features tui
```

## License

MIT
