## MODIFIED Requirements

### Requirement: Interactive version bump before build
Before the pre-flight check, the deploy script SHALL offer an optional interactive version-bump step. The step SHALL read the current version from `Cargo.toml`, prompt the user to bump (patch/minor/major/custom) or skip, and — if confirmed — update `Cargo.toml`, commit, and tag. See `semver-release-workflow` spec for full bump requirements.

#### Scenario: Version bump offered at deploy start
- **WHEN** the user runs `./deploy.sh`
- **THEN** the first interactive prompt SHALL ask whether to bump the version

#### Scenario: Skip proceeds to pre-flight
- **WHEN** the user answers "n" to the version bump prompt
- **THEN** the script SHALL proceed immediately to the cargo pre-flight check with no file changes
