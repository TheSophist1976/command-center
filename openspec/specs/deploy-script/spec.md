## ADDED Requirements

### Requirement: Interactive version bump before build
Before the pre-flight check, the deploy script SHALL offer an optional interactive version-bump step. The step SHALL read the current version from `Cargo.toml`, prompt the user to bump (patch/minor/major/custom) or skip, and — if confirmed — update `Cargo.toml`, commit, and tag. See `semver-release-workflow` spec for full bump requirements.

#### Scenario: Version bump offered at deploy start
- **WHEN** the user runs `./deploy.sh`
- **THEN** the first interactive prompt SHALL ask whether to bump the version

#### Scenario: Skip proceeds to pre-flight
- **WHEN** the user answers "n" to the version bump prompt
- **THEN** the script SHALL proceed immediately to the cargo pre-flight check with no file changes

### Requirement: Build and install both binaries
The deploy script SHALL build both the `task` (CLI) binary and the `task-tui` (TUI) binary in release mode, and SHALL install both to the configured install directory. The CLI binary SHALL be built without the `tui` Cargo feature. The TUI binary SHALL be built with `--features tui`. The script SHALL verify that `cargo` is available before attempting the build.

#### Scenario: Both binaries built
- **WHEN** the deploy script runs the build step
- **THEN** both `target/release/task` and `target/release/task-tui` SHALL be produced

#### Scenario: Both binaries installed
- **WHEN** the deploy script runs the install step
- **THEN** both `task` and `task-tui` SHALL be copied to `$INSTALL_DIR` and made executable

#### Scenario: Test step covers TUI code
- **WHEN** the deploy script runs the test step
- **THEN** tests SHALL be run with `--features tui` so TUI module tests are included

#### Scenario: Successful build
- **WHEN** the user runs `./deploy.sh` and cargo is installed
- **THEN** the script builds both binaries and proceeds to the next step

#### Scenario: Cargo not installed
- **WHEN** the user runs `./deploy.sh` and cargo is not found in PATH
- **THEN** the script prints an error message with install instructions and exits with a non-zero status

#### Scenario: Build failure
- **WHEN** `cargo build --release` exits with a non-zero status
- **THEN** the script prints the build error and exits with a non-zero status

### Requirement: Install binary to PATH directory
The script SHALL copy the built binary from `target/release/task` to an install directory. The default install directory SHALL be `~/.local/bin`. The user MAY override the install directory via the `INSTALL_DIR` environment variable. The script SHALL create the install directory if it does not exist.

#### Scenario: Default install location
- **WHEN** the build succeeds and `INSTALL_DIR` is not set
- **THEN** the script copies the binary to `~/.local/bin/task` and sets executable permissions

#### Scenario: Custom install location
- **WHEN** the build succeeds and `INSTALL_DIR` is set to `/usr/local/bin`
- **THEN** the script copies the binary to `/usr/local/bin/task`

#### Scenario: Install directory does not exist
- **WHEN** the install directory does not exist
- **THEN** the script creates it with `mkdir -p` before copying

### Requirement: Detect and update PATH
The script SHALL check if the install directory is present in the user's `$PATH`. If not, the script SHALL ask the user for consent before modifying their shell profile. The script SHALL detect the appropriate shell profile file based on the current shell.

#### Scenario: Install dir already in PATH
- **WHEN** the install directory is already in `$PATH`
- **THEN** the script skips PATH modification and prints a confirmation

#### Scenario: Install dir not in PATH and user consents
- **WHEN** the install directory is not in `$PATH` and the user answers "y" to the prompt
- **THEN** the script appends `export PATH="$HOME/.local/bin:$PATH"` to the detected shell profile and advises the user to source it or open a new terminal

#### Scenario: Install dir not in PATH and user declines
- **WHEN** the install directory is not in `$PATH` and the user answers "n"
- **THEN** the script prints the export line for manual addition and continues without modifying any file

#### Scenario: Shell profile detection
- **WHEN** the user's shell is zsh
- **THEN** the script targets `~/.zshrc`
- **WHEN** the user's shell is bash and `~/.bash_profile` exists
- **THEN** the script targets `~/.bash_profile`
- **WHEN** the user's shell is bash and `~/.bash_profile` does not exist
- **THEN** the script targets `~/.bashrc`

### Requirement: Validate default-dir config
The script SHALL check if the `default-dir` key exists in `~/.config/task-manager/config.md`. If missing, the script SHALL prompt the user to enter a directory path and write it to the config file using the existing `key: value` format.

#### Scenario: Config already set
- **WHEN** `default-dir` is present in the config file
- **THEN** the script prints the current value and skips config setup

#### Scenario: Config missing and user provides path
- **WHEN** `default-dir` is not set and the user enters `/home/user/tasks`
- **THEN** the script creates `~/.config/task-manager/config.md` (if needed) and writes `default-dir: /home/user/tasks`

#### Scenario: Tilde expansion in provided path
- **WHEN** the user enters `~/tasks` as the default directory
- **THEN** the script expands `~` to the user's home directory before writing

#### Scenario: Directory does not exist and user consents to create
- **WHEN** the entered directory does not exist and the user answers "y" to create it
- **THEN** the script creates the directory with `mkdir -p`

### Requirement: Optional Todoist token setup
The script SHALL check if a Todoist token exists at `~/.config/task-manager/todoist_token`. If missing, the script SHALL ask the user if they want to configure it. Setup MUST be skippable.

#### Scenario: Token already configured
- **WHEN** the Todoist token file exists and is non-empty
- **THEN** the script prints "Todoist: configured" and skips setup

#### Scenario: Token missing and user wants to set up
- **WHEN** the token file does not exist and the user answers "y"
- **THEN** the script prompts for the token, writes it to `~/.config/task-manager/todoist_token` with 0600 permissions

#### Scenario: Token missing and user skips
- **WHEN** the token file does not exist and the user answers "n"
- **THEN** the script prints a skip message and continues

### Requirement: Optional Claude API key setup
The script SHALL check for a Claude API key from the `ANTHROPIC_API_KEY` environment variable first, then from `~/.config/task-manager/claude_api_key`. If neither is set, the script SHALL ask the user if they want to configure it. Setup MUST be skippable.

#### Scenario: Key set via env var
- **WHEN** `ANTHROPIC_API_KEY` environment variable is set and non-empty
- **THEN** the script prints "Claude API: configured (env var)" and skips setup

#### Scenario: Key set via file
- **WHEN** `ANTHROPIC_API_KEY` is not set but `~/.config/task-manager/claude_api_key` exists and is non-empty
- **THEN** the script prints "Claude API: configured (file)" and skips setup

#### Scenario: Key missing and user wants to set up
- **WHEN** neither source has a key and the user answers "y"
- **THEN** the script prompts for the key, writes it to `~/.config/task-manager/claude_api_key` with 0600 permissions

#### Scenario: Key missing and user skips
- **WHEN** neither source has a key and the user answers "n"
- **THEN** the script prints a skip message and continues

### Requirement: Print setup summary
The script SHALL print a summary at the end showing the status of each component: binary location, PATH status, default-dir, Todoist, and Claude API.

#### Scenario: Full setup completed
- **WHEN** all steps complete (some skipped, some configured)
- **THEN** the script prints a summary table showing each component's status (e.g., "installed", "configured", "skipped")

### Requirement: Script is idempotent
The script SHALL be safe to re-run. Re-running SHALL rebuild and overwrite the binary, re-check config/auth (skipping already-configured items), and not duplicate PATH entries in shell profiles.

#### Scenario: Re-run with everything configured
- **WHEN** the script is run a second time with all config/auth already set
- **THEN** the script rebuilds, overwrites the binary, confirms all config is present, and does not modify shell profile

#### Scenario: Re-run does not duplicate PATH entry
- **WHEN** the shell profile already contains the PATH export line
- **THEN** the script does not append it again
