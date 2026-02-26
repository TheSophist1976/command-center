## Context

This is a greenfield Rust CLI project. There is no existing codebase. The primary users are developers who want a fast, terminal-native task manager for their projects. AI agents (Claude Code, GPT-based tooling, custom agents) are a key secondary audience — developers may ask an agent to help manage tasks, or an agent may operate the tool independently during autonomous work sessions. The Markdown storage format ensures both humans and agents can read and edit the same file.

Constraints:
- Must be a single static binary with no runtime dependencies
- Every operation must be a single non-interactive command
- Human-friendly by default, agent-friendly with a flag
- The Markdown file format must be hand-editable without breaking the parser

## Goals / Non-Goals

**Goals:**
- Sub-millisecond startup for simple operations (list, show)
- Clean, readable default output that developers enjoy using day-to-day
- `--json` mode on every subcommand for agent consumption
- Markdown storage that survives hand-editing, git merges, and partial corruption
- Exit codes for scripting (0 = success, 1 = error, 2 = not found)
- Minimal, obvious command surface — intuitive for developers, usable by agents without reading docs

**Non-Goals:**
- TUI or interactive mode
- Multi-user collaboration or locking
- Syncing, networking, or cloud storage
- Sub-task hierarchies or dependency graphs (v1)
- Plugin system or extensibility hooks

## Decisions

### 1. Markdown format: heading-per-task with inline metadata

Each task is an H2 heading with a checkbox, followed by metadata in an HTML comment:

```markdown
# Tasks

## [ ] Build the login page
<!-- id:3 priority:high tags:frontend,auth created:2025-01-15T10:00:00Z -->

Optional notes or description here.

## [x] Set up CI pipeline
<!-- id:1 priority:medium tags:infra created:2025-01-10T08:00:00Z -->
```

**Why this over alternatives:**
- *vs. JSON storage*: Markdown is human-readable, git-diffable, and editable in any text editor. A developer can open `tasks.md` and immediately understand their task list. JSON would be faster to parse but hostile to manual editing.
- *vs. YAML frontmatter at file top*: Per-task metadata inline keeps each task self-contained. A developer can move, copy, or delete a task section without breaking other tasks.
- *vs. visible metadata*: HTML comments (`<!-- -->`) are invisible when rendered, so the Markdown reads cleanly in GitHub, VS Code preview, or any renderer. The developer sees just checkboxes and titles.

### 2. Task ID: monotonic integer, stored in file header

A counter in the file header (`<!-- next-id:5 -->`) tracks the next available ID. IDs are never reused.

**Why this over alternatives:**
- *vs. UUID*: Integer IDs are short and easy to type — `task done 3` is natural at a terminal. Agents also benefit from compact IDs.
- *vs. hash-based*: Deterministic IDs from content would change when the task is edited, breaking references for both humans and agents.

### 3. CLI framework: `clap` with derive macros

Use `clap`'s derive API for argument parsing. Subcommands: `add`, `list`, `show`, `edit`, `done`, `undo`, `rm`.

**Why:**
- `clap` is the Rust standard, generates help text and shell completions automatically.
- Derive macros keep the CLI definition close to the code, reducing boilerplate.
- Built-in `--help` on every subcommand means developers can discover usage without external docs.

### 4. Output modes: human-first, agent-friendly

Default output is human-readable plain text — clean tables and concise summaries that feel natural in a terminal. `--json` flag switches to structured JSON for agent consumption. JSON output includes a top-level `ok` boolean for easy success checking.

```
$ task list
 ID  Status  Pri   Title                  Tags
  3  [ ]     high  Build the login page   frontend, auth
  1  [x]     med   Set up CI pipeline     infra

$ task list --json
{"ok": true, "tasks": [...]}
```

**Why:**
- Developers are the default audience — the tool should feel good to use without flags.
- `--json` is an opt-in for agents. One flag bridges both audiences.
- `ok` field lets agents check success without parsing error messages.

### 5. File location: `tasks.md` in current directory, overridable

Default looks for `tasks.md` in the working directory. `--file <path>` overrides. Also respects `TASK_FILE` environment variable.

**Why:**
- Convention over configuration — drop a `tasks.md` in any project and it just works. Developers see their tasks by running `task list` with zero setup.
- Env var lets agents set the file once per session rather than passing `--file` every time.
- `--file` gives explicit control when needed (multiple task files, non-standard locations).

### 6. Parser strategy: tolerant of malformed input

The parser reads the file line by line, builds task records, and silently skips malformed sections rather than failing. A `--strict` flag can opt into validation errors.

**Why:**
- Developers will hand-edit the Markdown file — typos, extra whitespace, or merge conflicts shouldn't break the tool.
- Agents need reliability — a parse error on one task shouldn't block listing the others.
- `--strict` exists for CI or validation use cases where correctness matters.

## Risks / Trade-offs

- **Hand-editing vs. format integrity**: Developers editing the Markdown directly may break metadata comments or checkbox syntax. → Mitigation: Tolerant parser that recovers gracefully. `task list --strict` can report format issues.
- **Concurrent access**: A developer and an agent (or two agents) writing to the same file simultaneously could corrupt it. → Mitigation: Use file-level advisory locking (`flock`). Sufficient for local single-machine use.
- **Large files**: Performance degrades with thousands of tasks since the entire file is read/written per operation. → Mitigation: Acceptable for v1. Most projects have <100 active tasks. Add archival subcommand later.
- **Markdown format lock-in**: Changing the format later requires migration. → Mitigation: Keep format simple, version it in the file header (`<!-- format:1 -->`), and write a migration path if v2 is needed.
