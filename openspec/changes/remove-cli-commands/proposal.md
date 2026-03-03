## Why

The TUI now has full AI-powered task management (NLP chat, inline editing, recurrence setting, Todoist import). The standalone CLI commands (add, list, show, edit, done, undo, rm, init, migrate, import) are redundant and add maintenance burden. The TUI is the primary interface.

## What Changes

- **BREAKING**: Remove CLI subcommands: `add`, `list`, `show`, `edit`, `done`, `undo`, `rm`, `init`, `migrate`, `import`
- Keep: `tui` (default command), `auth` (todoist/claude/status/revoke), `config` (set/get)
- Make `tui` the default when no subcommand is given (running bare `task` launches TUI)
- Remove `src/output.rs` (only used by CLI list/show/detail commands)
- Remove `--json` and `--strict` global flags (no longer needed without CLI commands)
- The TUI's internal `storage::load`/`save` and auto-init handle file management

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `cli-interface`: Remove task management subcommands, keep only tui/auth/config, make tui the default

## Impact

- `src/cli.rs`: Remove most Command variants, remove global flags
- `src/main.rs`: Remove command handlers for add/list/show/edit/done/undo/rm/init/migrate/import, remove helper functions
- `src/output.rs`: Delete entirely (print_task_table, print_task_detail only used by CLI)
- Integration tests in `tests/integration.rs` that test CLI commands will need removal
