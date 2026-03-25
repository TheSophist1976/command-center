## Context

Currently `src/main.rs` both declares all modules (via `mod` statements) and contains `fn main()`. All source files live in `src/` and are referenced as sibling modules. The package has a single implicit binary target.

Rust supports multiple binary targets in one package. Each binary target gets its own entry-point file in `src/bin/`. Modules shared between binaries must be declared in a library crate (`src/lib.rs`). Each binary then accesses shared code via `use <package_name>::<module>`.

## Goals / Non-Goals

**Goals:**
- `task` binary: compiles without ratatui/crossterm; includes auth, config, note, task CRUD; default → print help
- `task-tui` binary: full existing behaviour; default → launch TUI
- No changes to any module's logic or public API
- Both binaries install via `deploy.sh`

**Non-Goals:**
- Adding new CLI commands to the CLI binary
- Changing module structure (no moving files)
- Workspace split (separate crates) — single package only

## Decisions

### 1. Library crate (`src/lib.rs`)

All `mod` declarations move from `main.rs` to `lib.rs` as `pub mod`. TUI-related modules are wrapped in `#[cfg(feature = "tui")]`:

```rust
// Always compiled
pub mod auth;
pub mod cli;
pub mod config;
pub mod note;
pub mod parser;
pub mod storage;
pub mod task;

// Only with `tui` feature
#[cfg(feature = "tui")]
pub mod claude_session;
#[cfg(feature = "tui")]
pub mod nlp;
#[cfg(feature = "tui")]
pub mod slack;
#[cfg(feature = "tui")]
pub mod todoist;
#[cfg(feature = "tui")]
pub mod tui;
```

**Rationale**: `crate::` references inside modules resolve to the lib crate root regardless of which binary is active — no module code changes needed.

### 2. Feature flag

```toml
[features]
tui = ["dep:ratatui", "dep:crossterm"]

[dependencies]
ratatui  = { version = "0.29", optional = true }
crossterm = { version = "0.28", optional = true }
```

`task-tui` requires the `tui` feature; `task` does not. Building `task` alone compiles without ratatui/crossterm.

### 3. Binary entry points

`src/bin/task.rs` — uses only the always-compiled modules:
```rust
use task::{auth, cli, config, note, storage, task as task_mod};
```
Default subcommand: print help (via `clap`'s built-in `--help` output).

`src/bin/task_tui.rs` — mirrors current `main.rs` logic, uses all modules including TUI.

### 4. Cargo.toml `[[bin]]` entries

```toml
[[bin]]
name = "task"
path = "src/bin/task.rs"

[[bin]]
name = "task-tui"
path = "src/bin/task_tui.rs"
required-features = ["tui"]
```

Removing the implicit default binary requires deleting `src/main.rs`. Cargo automatically stops treating `src/main.rs` as a binary when `[[bin]]` entries are present that cover all targets, but to avoid ambiguity it should be deleted.

### 5. `deploy.sh` update

Replace the single `cargo build --release` with:
```bash
cargo build --release                          # builds task (no tui feature)
cargo build --release --features tui           # builds task-tui
```
Install both `target/release/task` and `target/release/task-tui` to `~/.local/bin/`.

## Risks / Trade-offs

- **`crate::` resolution**: all existing `use crate::X` statements inside modules resolve to the library root — this is correct. No changes needed to module internals.
- **Test compilation**: `cargo test` compiles against the lib crate. Tests in TUI modules that need `ratatui` must be run with `--features tui`. The `deploy.sh` test step will need `--features tui` to cover TUI tests.
- **`src/main.rs` removal**: deleting the file is required. If a developer runs `cargo run` without `--bin`, Cargo will error (no default bin). Developers must use `cargo run --bin task` or `cargo run --bin task-tui --features tui`.

## Open Questions

_(none)_
