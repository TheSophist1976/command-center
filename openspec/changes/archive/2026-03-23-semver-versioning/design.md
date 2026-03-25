## Context

The `task` binary is built with Clap and currently has no `--version` flag. The `Cargo.toml` version field (`0.1.0`) exists but is never surfaced to the user. Additionally, there is no workflow to increment that version before a deployment — developers bump it manually (or forget to).

The project has a single `deploy.sh` that handles testing, building, installing, and optional auth/config setup. Releases are personal (single-developer, local machine), not CI-driven.

## Goals / Non-Goals

**Goals:**
- Let users run `task --version` to see the installed version
- Add an interactive version-bump step to `deploy.sh` that updates `Cargo.toml`, commits, and tags before the build

**Non-Goals:**
- Changelog generation
- GitHub/GitLab release creation
- Automated CI/CD versioning
- Enforcing semver semantics on callers

## Decisions

### 1. Use Clap's built-in `version` attribute

Clap reads `CARGO_PKG_VERSION` at compile time when `version` is added to `#[command(...)]`. This is zero-cost (no runtime parsing), requires no new dependencies, and stays in sync automatically whenever `Cargo.toml` changes.

**Alternative considered:** Read `Cargo.toml` at runtime. Rejected — adds file I/O, can fail, and creates a sync gap if the binary is moved.

### 2. Version bump lives in `deploy.sh` as a new Step 0

The bump step is added before the existing pre-flight check. It reads the current version via `grep` on `Cargo.toml`, offers four choices (patch/minor/major/custom), edits `Cargo.toml` in-place with `sed`, then runs `git commit` and `git tag`. The step is skippable — the user can answer "n" to skip versioning and proceed directly to pre-flight.

**Alternative considered:** Standalone `release.sh`. Rejected per user preference — integrating into deploy avoids a separate entry point and keeps the workflow linear.

### 3. `Cargo.toml` as the single source of truth

The version is read from and written to `Cargo.toml` only. `Cargo.lock` is updated automatically by Cargo during the subsequent build step.

### 4. Tag format: `vX.Y.Z`

Standard convention (`v0.2.0`), consistent with the project's existing git history conventions.

## Risks / Trade-offs

- **Dirty working tree at tag time** → The bump step commits `Cargo.toml` before tagging, so the tag always points to a clean commit. If the working tree has other staged changes, they will be included in the bump commit — mitigation: warn the user if `git status` shows staged changes before the bump.
- **sed portability (macOS vs Linux)** → macOS `sed -i` requires an extension argument. Use `sed -i ''` on macOS and `sed -i` on Linux, or use `perl -pi` for portability. Since the project currently targets macOS (darwin), `sed -i ''` is acceptable and matches existing deploy.sh patterns.
- **Version string already exists in Cargo.lock** → Cargo regenerates `Cargo.lock` on the next build; no manual action needed.

## Migration Plan

1. Add `version` to `#[command(...)]` in `src/cli.rs` — one-line change, no migration needed
2. Add Step 0 to `deploy.sh` — existing step numbers shift in display labels only; no behavior of existing steps changes
3. No data migration required; no external API changes
