## Tasks

- [x] Update `src/cli.rs`: Remove `Command` variants `Add`, `List`, `Show`, `Edit`, `Done`, `Undo`, `Rm`, `Init`, `Migrate`, `Import` and their `ImportCommand` enum. Remove global `--json` and `--strict` flags from `Cli`. Make TUI the default by using `#[command(default_subcommand)]` or handling `None` command. Keep `Tui`, `Auth`, `Config`.
- [x] Update `src/main.rs`: Remove all command handlers for removed commands (Add, List, Show, Edit, Done, Undo, Rm, Init, Migrate, Import). Remove helper functions `parse_recurrence`, `parse_due_date`, `validate_and_parse_tags`. Replace `output::print_success`/`output::print_error` in Auth/Config handlers with `println!`/`eprintln!`. Remove `mod output` declaration. Remove unused imports. Handle default (no subcommand) by launching TUI.
- [x] Delete `src/output.rs` entirely. Remove any remaining references to `output::` in other files.
- [x] Update `tests/integration.rs`: Remove all tests for removed CLI commands (add, list, show, edit, done, undo, rm, init, migrate, import). Keep any auth/config tests if they exist, or remove the file entirely if all tests are for removed commands.
- [x] Build and run `cargo test` to verify everything compiles and passes. Fix any remaining compilation errors from removed references.
