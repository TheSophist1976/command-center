## Why

The project produces a single binary that bundles both CLI commands and the full interactive TUI, including ratatui, crossterm, Slack, NLP, and Claude session dependencies. This makes it impossible to deploy a lightweight CLI-only binary to headless servers or CI environments. Splitting into two binaries allows the CLI (`task`) to ship without any TUI dependencies, while the full TUI experience (`task-tui`) remains available for interactive use.

## What Changes

- Convert `src/main.rs` into a library crate (`src/lib.rs`) that exposes all existing modules as `pub mod`
- Add a `tui` Cargo feature that gates `ratatui`, `crossterm`, and TUI-related modules (`tui`, `claude_session`, `nlp`, `slack`, `todoist`)
- Create `src/bin/task.rs` — CLI-only binary: `auth`, `config`, `note`, task CRUD subcommands; default prints help
- Create `src/bin/task_tui.rs` — full TUI binary: all subcommands plus default → launch TUI
- Add two `[[bin]]` entries to `Cargo.toml`; `task-tui` requires the `tui` feature
- Remove `src/main.rs`
- Update `deploy.sh` to build and install both binaries

## Capabilities

### New Capabilities

_(none — this is a structural refactor of the build system)_

### Modified Capabilities

- `cli-interface`: The CLI binary changes its default behaviour (no TUI launch; prints help instead) and is now a separate binary. The TUI binary retains all existing behaviour.
- `deploy-script`: Script must build and install both `task` and `task-tui`.

## Impact

- `Cargo.toml`: optional deps, `[features]`, two `[[bin]]` sections
- `src/lib.rs`: new file — all existing `mod` declarations moved here, TUI-related modules wrapped in `#[cfg(feature = "tui")]`
- `src/bin/task.rs`: new file — CLI entry point
- `src/bin/task_tui.rs`: new file — TUI entry point (replaces `src/main.rs`)
- `src/main.rs`: deleted
- `deploy.sh`: build and install both binaries
- All existing source modules unchanged (no logic changes)
