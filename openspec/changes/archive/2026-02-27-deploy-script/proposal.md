## Why

There is no automated way to build the `task` binary and install it for everyday use. Users must manually run `cargo build --release`, locate the binary, copy it to a PATH directory, and separately verify that config and auth credentials are in place. A deploy script eliminates this friction and makes first-time setup seamless.

## What Changes

- Add a `deploy.sh` shell script at the project root that:
  1. Builds the project in release mode via `cargo build --release`
  2. Checks for required config (`default-dir` in `~/.config/task-manager/config.md`) — if missing, interactively walks the user through setting it
  3. Checks for optional auth credentials (Todoist token at `~/.config/task-manager/todoist_token`, Claude API key at `~/.config/task-manager/claude_api_key` or `ANTHROPIC_API_KEY` env var) — prompts the user to configure each, with the option to skip
  4. Installs the built binary to a user-writable PATH location (e.g., `~/.local/bin` or `/usr/local/bin`), creating the directory and updating shell profile if needed
- The script is idempotent — safe to re-run for upgrades or reconfiguration

## Capabilities

### New Capabilities
- `deploy-script`: Shell-based build, config validation, auth setup, and PATH installation workflow

### Modified Capabilities

## Impact

- New file: `deploy.sh` at project root
- No changes to existing Rust source code
- Reads from existing config/auth paths defined in `src/config.rs` and `src/auth.rs`
- Touches user shell profile (`~/.zshrc`, `~/.bashrc`, or `~/.bash_profile`) only if PATH modification is needed and user consents
