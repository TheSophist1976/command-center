## 1. Project Setup

- [x] 1.1 Initialize Rust project with `cargo init --name task`
- [x] 1.2 Add dependencies: `clap` (derive), `serde`, `serde_json`, `chrono`
- [x] 1.3 Set up module structure: `main.rs`, `cli.rs`, `parser.rs`, `storage.rs`, `task.rs`, `output.rs`

## 2. Data Model

- [x] 2.1 Define `Task` struct with fields: id (u32), title (String), status (enum: Open/Done), priority (enum: High/Medium/Low), tags (Vec<String>), created (DateTime), updated (Option<DateTime>), description (Option<String>)
- [x] 2.2 Define `TaskFile` struct holding the task list, next-id counter, and format version
- [x] 2.3 Implement `Serialize`/`Deserialize` for JSON output on Task

## 3. Markdown Parser

- [x] 3.1 Implement line-by-line parser that extracts file header comments (`format`, `next-id`)
- [x] 3.2 Parse H2 headings with checkbox syntax (`## [ ] title` / `## [x] title`) into task status and title
- [x] 3.3 Parse HTML metadata comments (`<!-- id:N priority:P tags:T created:C -->`) into task fields
- [x] 3.4 Capture description body text between metadata comment and next H2 heading
- [x] 3.5 Implement tolerant mode: skip malformed sections, continue parsing
- [x] 3.6 Implement strict mode: collect and report parse errors via `--strict`
- [x] 3.7 Handle edge cases: empty file, missing file (return empty list), missing header, missing optional fields

## 4. Markdown Serializer

- [x] 4.1 Implement `TaskFile` to Markdown string serialization (header comments, then task sections)
- [x] 4.2 Ensure round-trip fidelity: parse → serialize produces equivalent output for well-formed files

## 5. File I/O and Safety

- [x] 5.1 Implement file path resolution: `--file` flag > `TASK_FILE` env var > `./tasks.md`
- [x] 5.2 Implement atomic writes: write to temp file, then rename over original
- [x] 5.3 Implement advisory file locking (`flock`) for write operations with brief retry
- [x] 5.4 Implement auto-init: create file with headers when adding to a nonexistent file

## 6. CLI Definition

- [x] 6.1 Define top-level CLI with global `--file` and `--json` flags using clap derive
- [x] 6.2 Define `init` subcommand (no args)
- [x] 6.3 Define `add` subcommand: positional title, optional `--priority`, optional `--tags`
- [x] 6.4 Define `list` subcommand: optional `--status`, `--priority`, `--tag` filters
- [x] 6.5 Define `show` subcommand: positional task ID
- [x] 6.6 Define `edit` subcommand: positional task ID, optional `--title`, `--priority`, `--tags`
- [x] 6.7 Define `done` subcommand: positional task ID
- [x] 6.8 Define `undo` subcommand: positional task ID
- [x] 6.9 Define `rm` subcommand: positional task ID

## 7. Command Implementations

- [x] 7.1 Implement `init`: create file with `<!-- format:1 -->` and `<!-- next-id:1 -->` headers, error if exists
- [x] 7.2 Implement `add`: parse file (or auto-init), assign next-id, append task, write file
- [x] 7.3 Implement `list`: parse file, apply filters (status/priority/tag with AND logic), display results
- [x] 7.4 Implement `show`: parse file, find task by ID, display full detail including description
- [x] 7.5 Implement `edit`: parse file, find task by ID, update specified fields, set updated timestamp, write file
- [x] 7.6 Implement `done`: parse file, find task by ID, set status to Done, set updated timestamp, write file
- [x] 7.7 Implement `undo`: parse file, find task by ID, set status to Open, set updated timestamp, write file
- [x] 7.8 Implement `rm`: parse file, find task by ID, remove from list, write file

## 8. Output Formatting

- [x] 8.1 Implement human-readable table output for `list` (aligned columns: ID, Status, Pri, Title, Tags)
- [x] 8.2 Implement human-readable detail output for `show` (all fields, description)
- [x] 8.3 Implement human-readable confirmation messages for mutating commands (add, edit, done, undo, rm)
- [x] 8.4 Implement JSON output mode: wrap all responses in `{"ok": true/false, ...}` structure
- [x] 8.5 Implement consistent exit codes: 0 success, 1 error, 2 not found

## 9. Input Validation

- [x] 9.1 Validate priority values against {high, medium, low}, exit 1 with error on invalid
- [x] 9.2 Validate tag format (lowercase alphanumeric + hyphens, comma-separated), exit 1 with error on invalid
- [x] 9.3 Validate task ID is a positive integer

## 10. Testing

- [x] 10.1 Unit tests for parser: well-formed files, empty files, malformed sections, missing headers
- [x] 10.2 Unit tests for serializer: round-trip fidelity
- [x] 10.3 Integration tests for each subcommand: add, list, show, edit, done, undo, rm, init
- [x] 10.4 Integration tests for filters: status, priority, tag, combined
- [x] 10.5 Integration tests for JSON output mode on each subcommand
- [x] 10.6 Integration tests for exit codes: success, error, not found
- [x] 10.7 Integration tests for file path resolution: --file flag, TASK_FILE env, default
- [x] 10.8 Test auto-init behavior when task file doesn't exist
