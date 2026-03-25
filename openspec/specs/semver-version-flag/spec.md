## ADDED Requirements

### Requirement: Version flag
The CLI SHALL expose a `--version` flag that prints the current version string to stdout and exits. The version SHALL be the semver string from `Cargo.toml` at compile time, formatted as `task <version>` (e.g., `task 0.2.0`).

#### Scenario: Version flag prints version
- **WHEN** the user runs `task --version`
- **THEN** the system SHALL print `task <version>` (e.g., `task 0.2.0`) to stdout and exit with code 0

#### Scenario: Short version flag
- **WHEN** the user runs `task -V`
- **THEN** the system SHALL print the same version string as `--version` and exit with code 0

#### Scenario: Version is compile-time baked
- **WHEN** the binary is built from a `Cargo.toml` with `version = "1.3.0"`
- **THEN** `task --version` SHALL print `task 1.3.0` regardless of the current `Cargo.toml` on disk
