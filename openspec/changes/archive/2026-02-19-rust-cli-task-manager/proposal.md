## Why

Developers need a lightweight, fast task manager they can use directly from the terminal — and that also works seamlessly when an AI agent (Claude, GPT, etc.) is assisting them. Today's task tools are either GUI-heavy, require cloud accounts, or produce formats that are opaque to both humans and agents. A Rust CLI that stores tasks in a simple Markdown file gives developers a tool they can use themselves, hand off to an AI agent for help, or let an agent operate independently — all on the same readable, git-friendly file.

## What Changes

- New Rust CLI binary (`task`) with subcommands for full task lifecycle
- Markdown-file storage — one `.md` file per project, human-readable and git-friendly
- Human-friendly default output (table/plain text) for everyday use
- Machine-parseable output (JSON mode) so AI agents can consume results programmatically
- Task metadata: id, title, status, priority, tags, created/updated timestamps
- Filtering and querying by status, priority, and tags
- Designed for single-command operations — no interactive prompts, every action is one invocation
- Works equally well when a human types the command or an agent invokes it

## Capabilities

### New Capabilities

- `task-storage`: Markdown file format for persisting tasks — parsing, serialization, and file I/O
- `task-lifecycle`: Core CRUD operations — add, list, update, complete, delete tasks
- `cli-interface`: Command-line argument parsing, JSON/plain output modes, exit codes for scripting

### Modified Capabilities

(none — greenfield project)

## Impact

- **New binary**: `task` (or configurable name) installed via `cargo install`
- **Dependencies**: Rust toolchain, `clap` for CLI, `serde`/`serde_json` for JSON output
- **File system**: Creates/reads `.md` task files in the current directory or a specified path
- **No external services**: Fully offline, no database, no network calls
