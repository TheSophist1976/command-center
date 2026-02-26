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
# Create your first task (auto-creates tasks.md)
task add "Build the login page" --priority high --tags frontend,auth

# Add more tasks
task add "Set up CI pipeline" --tags infra
task add "Write tests" --priority low

# List all tasks
task list

# Filter tasks
task list --status open --priority high
task list --tag frontend

# Complete a task
task done 1

# View task details
task show 1

# Edit a task
task edit 2 --title "Configure CI/CD pipeline" --priority high

# Reopen a completed task
task undo 1

# Remove a task
task rm 3

# Launch the interactive TUI
task tui
```

## Commands

| Command                         | Description                            |
| ------------------------------- | -------------------------------------- |
| `task init`                     | Create an empty task file              |
| `task add <title>`              | Add a new task                         |
| `task list`                     | List tasks (with optional filters)     |
| `task show <id>`                | Show full task details                 |
| `task edit <id>`                | Edit a task's title, priority, or tags |
| `task done <id>`                | Mark a task as complete                |
| `task undo <id>`                | Reopen a completed task                |
| `task rm <id>`                  | Remove a task                          |
| `task tui`                      | Launch the interactive terminal UI     |
| `task config set <key> <value>` | Set a configuration value              |
| `task config get <key>`         | Get a configuration value              |

### Global Flags

| Flag            | Description                                         |
| --------------- | --------------------------------------------------- |
| `--file <path>` | Use a custom task file (default: `tasks.md`)        |
| `--json`        | Output in JSON format (for AI agents and scripting) |
| `--strict`      | Report errors for malformed task entries            |

### Filter Flags (for `list`)

| Flag                             | Description        |
| -------------------------------- | ------------------ |
| `--status <open\|done>`          | Filter by status   |
| `--priority <high\|medium\|low>` | Filter by priority |
| `--tag <name>`                   | Filter by tag      |

## JSON Mode

Every command supports `--json` for machine-readable output:

```sh
$ task list --json
{"ok":true,"tasks":[{"id":1,"title":"Build the login page","status":"open","priority":"high","tags":["frontend","auth"],"created":"2025-01-15T10:00:00+00:00"}]}

$ task show 999 --json
{"ok":false,"error":"Task 999 not found"}
```

JSON responses always include an `ok` boolean. Exit codes: `0` success, `1` error, `2` not found.

## File Path Resolution

The task file is resolved in this order:

1. `--file <path>` flag
2. `TASK_FILE` environment variable
3. `default-dir` config value (set via `task config set default-dir <path>`)
4. `tasks.md` in the current directory

```sh
# Use a custom file for a single command
task --file ~/projects/my-tasks.md list

# Or set the env var for the current session
export TASK_FILE=~/projects/my-tasks.md
task list

# Or set a persistent default directory
task config set default-dir ~/projects
task config get default-dir
```

## File Format

Tasks are stored as Markdown with metadata in HTML comments:

```markdown
<!-- format:1 -->
<!-- next-id:3 -->

# Tasks

## [ ] Build the login page

<!-- id:1 priority:high tags:frontend,auth created:2025-01-15T10:00:00+00:00 -->

Some notes about this task.

## [X] Set up CI pipeline

<!-- id:2 priority:medium tags:infra created:2025-01-10T08:00:00+00:00 -->
```

The file is safe to edit by hand. The parser is tolerant of formatting issues — malformed entries are skipped rather than causing errors.

## Running Tests

```sh
cargo test
```

## License

MIT
