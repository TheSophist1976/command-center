## Why

The `task` binary has no way for users to check what version is installed, and there is no disciplined workflow for cutting releases — version bumps are ad-hoc. Adding a `--version` flag and a structured release step in `deploy.sh` closes both gaps.

## What Changes

- Add `--version` flag to the `task` CLI binary, printing the version from `Cargo.toml` at compile time
- Add an interactive version-bump step to `deploy.sh` (before build) that reads the current version, prompts the user for patch/minor/major or a custom version, updates `Cargo.toml`, commits, and creates a `vX.Y.Z` git tag

## Capabilities

### New Capabilities
- `semver-version-flag`: CLI exposes `--version` flag that prints the current semver version (e.g. `task 0.2.0`)
- `semver-release-workflow`: `deploy.sh` includes an interactive pre-build step that bumps `Cargo.toml` version, commits, and tags the release

### Modified Capabilities
- `cli-interface`: Adding the `--version` flag requirement to the CLI spec
- `deploy-script`: Adding the version bump step requirement to the deploy spec

## Impact

- `src/cli.rs`: add `version` to the `#[command(...)]` attribute on `Cli` — no new dependencies
- `Cargo.toml`: version field is the single source of truth, updated by the release workflow
- `deploy.sh`: new Step 0 (version bump) added before existing Step 1 (pre-flight); all existing step numbers shift by one in display only
- No breaking changes to existing CLI behavior or deploy behavior — the version bump step in deploy is opt-out (user can skip)
