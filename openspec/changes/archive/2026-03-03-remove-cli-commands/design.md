## Context

The CLI has ~15 subcommands. The TUI handles all task operations (add, edit, delete, toggle, filter, NLP chat, import, due dates, recurrence). Only `auth` and `config` are still needed as CLI commands since they configure credentials/settings before launching the TUI.

## Goals / Non-Goals

**Goals:**
- Remove dead CLI code to reduce maintenance surface
- Make `task` (bare command) launch the TUI directly
- Keep auth and config accessible from CLI

**Non-Goals:**
- No new TUI features — this is purely a removal
- Not removing the underlying modules (storage, parser, task) — the TUI still uses them

## Decisions

### 1. Default command is TUI
Running `task` with no subcommand launches the TUI. `task tui` remains as an explicit alias. This is the most natural UX — users just type `task`.

### 2. Keep --file global flag
The `--file` flag is still useful for pointing the TUI at a specific task file. Remove `--json` and `--strict` (only CLI commands used these).

### 3. Delete src/output.rs entirely
The `output` module contains `print_task_table`, `print_task_detail`, `print_success`, `print_error`. The first two are CLI-only. `print_success`/`print_error` are used in main.rs CLI handlers — with those gone, the entire module is unused. The auth commands can use simple `println!`/`eprintln!` instead.

### 4. Remove CLI integration tests
Tests in `tests/integration.rs` that exercise CLI commands (add, list, edit, done, etc.) should be removed. TUI behavior is tested via unit tests in `src/tui.rs`.

## Risks / Trade-offs

- [Automation breakage] External scripts calling `task add ...` or `task list --json` will break → This is intentional; the TUI+NLP is the new interface. Document in commit message.
- [Auth output] Auth/config commands currently use `output::print_success` → Replace with direct `println!`.
