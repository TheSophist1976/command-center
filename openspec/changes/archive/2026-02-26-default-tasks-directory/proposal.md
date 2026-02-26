## Why

Users need a way to set a persistent default path for their `tasks.md` file so the TUI opens that file automatically without requiring a path argument every time. Currently there is no such configuration — the user must always specify a file path or rely on a hardcoded default.

## What Changes

- Add a new `task config` CLI subcommand with `set` and `get` operations to manage configuration values
- Add a `default-dir` config key that stores the directory from which `tasks.md` is loaded
- The `task tui` command reads the configured default directory and loads `tasks.md` from there automatically
- The TUI exposes an in-app action to update the default directory setting without leaving the terminal
- Configuration is persisted to `{config_dir}/task-manager/config.md` (plain markdown file, human-readable)

## Capabilities

### New Capabilities
- `app-config`: Read/write persistent application configuration (default tasks directory) stored in a markdown config file; update via CLI and TUI

### Modified Capabilities
- `tui`: TUI startup now reads app config to determine which `tasks.md` file to load by default

## Impact

- `src/main.rs`: new `config` subcommand handler
- `src/cli.rs`: new `ConfigCommand` enum with `Set` and `Get` variants
- `src/config.rs`: new module for config file read/write
- `src/tui.rs`: reads default directory from config on startup; adds in-app keybinding to update it
- No new dependencies required
