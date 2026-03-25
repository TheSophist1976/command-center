## 1. CLI Version Flag

- [x] 1.1 Add `version` to `#[command(...)]` attribute on `Cli` in `src/cli.rs`
- [x] 1.2 Build and verify `task --version` prints `task 0.1.0` and `task -V` produces the same output

## 2. Deploy Script — Version Bump Step

- [x] 2.1 Add Step 0 "Version bump" to `deploy.sh` before the existing pre-flight check, shifting display labels of existing steps by 1
- [x] 2.2 Implement version reading: extract current version from `Cargo.toml` via grep/sed and display it
- [x] 2.3 Implement bump menu: offer choices patch / minor / major / custom; if "n" skip entirely
- [x] 2.4 Implement semver arithmetic for patch/minor/major options using shell arithmetic
- [x] 2.5 Validate custom version input matches `X.Y.Z` pattern; exit with error on invalid input
- [x] 2.6 Update `Cargo.toml` version field in-place using `sed -i ''` (macOS-compatible)
- [x] 2.7 Stage only `Cargo.toml` and commit with message `chore: bump to vX.Y.Z`
- [x] 2.8 Create lightweight git tag `vX.Y.Z` pointing to the bump commit
- [x] 2.9 Add version bump status to the final summary table in deploy.sh

## 3. Verification

- [ ] 3.1 Manually run `./deploy.sh`, choose a patch bump, and confirm `Cargo.toml`, commit, and tag are correct
- [ ] 3.2 Manually run `./deploy.sh`, skip the bump, and confirm no git changes occur
- [x] 3.3 Run the full test suite (`cargo test --features tui`) to confirm no regressions
