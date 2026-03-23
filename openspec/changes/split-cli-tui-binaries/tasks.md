## 1. Cargo.toml — Feature Flag and Optional Dependencies

- [x] 1.1 Make `ratatui` optional: `ratatui = { version = "0.29", optional = true }`
- [x] 1.2 Make `crossterm` optional: `crossterm = { version = "0.28", optional = true }`
- [x] 1.3 Add `[features]` section: `tui = ["dep:ratatui", "dep:crossterm"]`
- [x] 1.4 Add `[[bin]]` entry for `task`: `name = "task"`, `path = "src/bin/task.rs"`
- [x] 1.5 Add `[[bin]]` entry for `task-tui`: `name = "task-tui"`, `path = "src/bin/task_tui.rs"`, `required-features = ["tui"]`

## 2. Library Crate — `src/lib.rs`

- [x] 2.1 Create `src/lib.rs` with unconditional `pub mod` declarations for: `auth`, `cli`, `config`, `note`, `parser`, `storage`, `task`
- [x] 2.2 Add `#[cfg(feature = "tui")] pub mod claude_session;` to `src/lib.rs`
- [x] 2.3 Add `#[cfg(feature = "tui")] pub mod nlp;` to `src/lib.rs`
- [x] 2.4 Add `#[cfg(feature = "tui")] pub mod slack;` to `src/lib.rs`
- [x] 2.5 Add `#[cfg(feature = "tui")] pub mod todoist;` to `src/lib.rs`
- [x] 2.6 Add `#[cfg(feature = "tui")] pub mod tui;` to `src/lib.rs`

## 3. CLI Binary — `src/bin/task.rs`

- [x] 3.1 Create `src/bin/task.rs` with a `main()` that parses the CLI using `task::cli::Cli` and dispatches to `auth`, `config`, `note` subcommands (same logic as current `main.rs` minus the TUI branch)
- [x] 3.2 Set the default (no subcommand) to print clap help and exit 0 — remove the `None | Some(Command::Tui)` TUI launch branch

## 4. TUI Binary — `src/bin/task_tui.rs`

- [x] 4.1 Create `src/bin/task_tui.rs` with a `main()` identical to the current `src/main.rs` content, using `task::*` imports from the library crate

## 5. Remove `src/main.rs`

- [x] 5.1 Delete `src/main.rs` (replaced by `src/lib.rs` + `src/bin/task.rs` + `src/bin/task_tui.rs`)

## 6. Verify Build

- [x] 6.1 Run `cargo build --bin task` — confirm it succeeds and produces a binary without ratatui
- [x] 6.2 Run `cargo build --bin task-tui --features tui` — confirm it succeeds
- [x] 6.3 Run `cargo test --features tui -- --skip auth::tests --skip todoist::tests` — confirm all tests pass

## 7. Update `deploy.sh`

- [x] 7.1 Replace the single `cargo build --release` with two builds: `cargo build --release` (task) and `cargo build --release --features tui` (task-tui)
- [x] 7.2 Update the test step to use `--features tui` so TUI tests are included
- [x] 7.3 Add install step for `task-tui`: copy `target/release/task-tui` to `$INSTALL_DIR/task-tui` and make executable
- [x] 7.4 Add `task-tui` to the workspace copy step (if `$WORKSPACE_DIR` exists)
- [x] 7.5 Update the summary output to show both binaries installed
