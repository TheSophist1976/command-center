## MODIFIED Requirements

### Requirement: Build and install both binaries
The deploy script SHALL build both the `task` (CLI) binary and the `task-tui` (TUI) binary in release mode, and SHALL install both to the configured install directory. The CLI binary SHALL be built without the `tui` Cargo feature. The TUI binary SHALL be built with `--features tui`.

#### Scenario: Both binaries built
- **WHEN** the deploy script runs the build step
- **THEN** both `target/release/task` and `target/release/task-tui` SHALL be produced

#### Scenario: Both binaries installed
- **WHEN** the deploy script runs the install step
- **THEN** both `task` and `task-tui` SHALL be copied to `$INSTALL_DIR` and made executable

#### Scenario: Test step covers TUI code
- **WHEN** the deploy script runs the test step
- **THEN** tests SHALL be run with `--features tui` so TUI module tests are included
