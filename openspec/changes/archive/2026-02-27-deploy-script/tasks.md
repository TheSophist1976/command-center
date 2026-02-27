## 1. Script Scaffold and Pre-flight

- [x] 1.1 Create `deploy.sh` at project root with bash shebang, `set -euo pipefail`, and color/formatting helper functions
- [x] 1.2 Add pre-flight check: verify `cargo` is in PATH, print error with install instructions if missing

## 2. Build

- [x] 2.1 Run `cargo build --release` and exit on failure with the build error output

## 3. Install Binary

- [x] 3.1 Determine install directory (`INSTALL_DIR` env var or default `~/.local/bin`), create with `mkdir -p` if needed
- [x] 3.2 Copy `target/release/task` to the install directory and set executable permissions

## 4. PATH Detection and Shell Profile

- [x] 4.1 Check if install directory is in `$PATH`; skip modification if already present
- [x] 4.2 Detect shell profile file (`~/.zshrc`, `~/.bash_profile`, or `~/.bashrc`) based on `$SHELL`
- [x] 4.3 Prompt user for consent, append `export PATH` line if accepted, or print manual instructions if declined
- [x] 4.4 Check for existing PATH export line in profile before appending (idempotency)

## 5. Config Setup

- [x] 5.1 Check for `default-dir` in `~/.config/task-manager/config.md`; skip if already set
- [x] 5.2 Prompt user for directory path, expand `~`, optionally create the directory, and write to config file

## 6. Auth Setup

- [x] 6.1 Check for Todoist token at `~/.config/task-manager/todoist_token`; skip if present, otherwise ask user and write with 0600 permissions
- [x] 6.2 Check for Claude API key via `ANTHROPIC_API_KEY` env var then `~/.config/task-manager/claude_api_key`; skip if either is set, otherwise ask user and write with 0600 permissions

## 7. Summary and Finish

- [x] 7.1 Print a summary showing status of each component (binary, PATH, default-dir, Todoist, Claude API)
- [x] 7.2 Make `deploy.sh` executable and verify it runs end-to-end on a clean setup
