## ADDED Requirements

### Requirement: Interactive version bump in deploy script
The deploy script SHALL include an interactive version-bump step that runs before the pre-flight check. The step SHALL read the current version from `Cargo.toml`, display it, and ask the user whether to bump. The step SHALL be skippable without affecting subsequent steps.

#### Scenario: User skips version bump
- **WHEN** the user answers "n" to the version bump prompt
- **THEN** the script SHALL proceed to the pre-flight step without modifying any files or creating any git objects

#### Scenario: User selects patch bump
- **WHEN** the user answers "y" to the version bump prompt and selects "patch"
- **THEN** the script SHALL increment the patch component (e.g., `0.1.0` → `0.1.1`), update `Cargo.toml`, commit with message `chore: bump to v0.1.1`, and create git tag `v0.1.1`

#### Scenario: User selects minor bump
- **WHEN** the user selects "minor"
- **THEN** the script SHALL increment the minor component and reset patch to 0 (e.g., `0.1.2` → `0.2.0`), update `Cargo.toml`, commit, and tag

#### Scenario: User selects major bump
- **WHEN** the user selects "major"
- **THEN** the script SHALL increment the major component and reset minor and patch to 0 (e.g., `0.3.1` → `1.0.0`), update `Cargo.toml`, commit, and tag

#### Scenario: User enters custom version
- **WHEN** the user selects "custom" and enters a valid semver string (e.g., `0.5.0`)
- **THEN** the script SHALL set the version to that exact string, update `Cargo.toml`, commit, and tag

#### Scenario: User enters invalid custom version
- **WHEN** the user selects "custom" and enters a string that does not match `X.Y.Z` (three non-negative integers separated by dots)
- **THEN** the script SHALL print an error and exit with a non-zero status without modifying any files

### Requirement: Version bump commit and tag
After updating `Cargo.toml`, the deploy script SHALL commit only `Cargo.toml` with the message `chore: bump to vX.Y.Z` and create an annotated or lightweight git tag `vX.Y.Z` pointing to that commit.

#### Scenario: Commit is created
- **WHEN** a version bump is confirmed
- **THEN** `git log` SHALL show a new commit with message `chore: bump to v<new-version>` as HEAD

#### Scenario: Tag is created
- **WHEN** a version bump is confirmed
- **THEN** `git tag` SHALL list `v<new-version>`

#### Scenario: Only Cargo.toml is committed
- **WHEN** other files have unstaged or staged changes at bump time
- **THEN** only `Cargo.toml` SHALL be staged and committed by the bump step; other changes SHALL remain untouched

### Requirement: Version bump is opt-out
The version bump step SHALL ask the user for consent before making any changes. Answering "n" SHALL leave the repo state identical to before the step ran.

#### Scenario: No changes on skip
- **WHEN** the user answers "n" to the bump prompt
- **THEN** `git status` and `Cargo.toml` SHALL be identical to their state before `deploy.sh` was run (up to the bump step)
