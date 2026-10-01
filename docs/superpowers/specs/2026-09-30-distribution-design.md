# Distribution: binary releases, installer, and in-app updates

Date: 2026-09-30
Status: draft, awaiting review

## Goal

Distribute command-center without its source code. GitHub Actions builds the Rust
binaries and the web UI; users install with one command and update either from the CLI
or from a button in the web UI.

## Decisions

| Topic | Decision |
| --- | --- |
| Source visibility | Source repo stays private. A separate **public releases repo** holds only binaries and `install.sh`. |
| Platforms | macOS (arm64, x64) and Linux (x64, arm64). No Windows. |
| Architecture | One self-contained `task` binary with the web UI embedded. Subcommands: `serve`, `setup`, `update`. |
| Update path | `task update` (CLI) and an Update button in the web UI, both using one shared updater. |
| First-run setup | `task setup` subcommand. The installer only places binaries. |
| Restart after UI update | Automatic: the server re-execs the new binary on the same port. |
| Signing | None. SHA-256 checksum verification only. |
| Todoist | Not part of `task setup`. Existing `task auth todoist` code is left untouched. |

## Non-goals

Windows support, code signing or notarization, auto-update without user action,
launchd/systemd service units, Homebrew tap, removing the Todoist code.

## 1. Build and release pipeline

**Trigger.** Pushing a tag `vX.Y.Z` to the private repo runs
`.github/workflows/release.yml`. The first step fails the run unless the tag equals the
`version` in `Cargo.toml`.

**Verify.** `cargo test` and the web build (`npm ci && npm run build`, which includes
`tsc -b`) must pass before anything is published.

**Build matrix** (native runners, no cross-compilation, because `rusqlite` is bundled
and needs a C toolchain):

| Target | Runner |
| --- | --- |
| aarch64-apple-darwin | macOS arm64 |
| x86_64-apple-darwin | macOS Intel |
| x86_64-unknown-linux-gnu | Ubuntu x64 |
| aarch64-unknown-linux-gnu | Ubuntu arm64 |

Each job builds the web UI first, then `cargo build --release --features tui`, so the
UI is embedded at compile time.

**TLS.** `reqwest` switches from its default (system OpenSSL) to `rustls`
(`default-features = false`, features `blocking`, `json`, `rustls-tls`). Linux binaries
then have no dynamic `libssl` dependency. The updater depends on this.

**Artifacts.** `task-vX.Y.Z-<target>.tar.gz` containing `task` and `task-tui`, plus a
single `SHA256SUMS` file covering every archive.

**Publishing.** The workflow creates a GitHub Release in the public releases repo and
uploads the archives and `SHA256SUMS`. It authenticates with a fine-grained token stored
as a repository secret, scoped to contents-write on the releases repo only. `install.sh`
lives in the releases repo and is fetched from its raw URL.

## 2. Binary layout and embedding

- The `task` binary gains subcommands `serve`, `setup`, and `update`.
- The built `web/dist` is embedded at compile time (e.g. `rust-embed`). In **debug**
  builds assets are read from `web/dist` on disk, so `./start-web.sh --dev` and
  `cargo run` are unchanged.
- `task_server` remains as a thin alias for `task serve` so `start-web.sh` keeps working.
- `AGENTS.md` and the Claude skills (`skills/`) are embedded too, so `task setup` and
  `task update` always install files matching the binary's version.
- The current version is `CARGO_PKG_VERSION`.

## 3. `install.sh`

Run as `curl -fsSL <releases-raw-url>/install.sh | sh`.

1. Detect OS and architecture, map to one of the four targets; exit with a clear message
   on anything else.
2. Find the latest release, download the matching archive and `SHA256SUMS`.
3. Verify the checksum; abort on mismatch.
4. Install `task` and `task-tui` into `~/.local/bin` (`INSTALL_DIR` overrides).
5. Offer to add the install directory to the shell profile, idempotently, as
   `deploy.sh` does today.
6. Print "run `task setup`".

The script contains no configuration logic.

## 4. `task setup`

Interactive, idempotent, never overwrites an existing value without asking. Sets:

- `default-dir` (where `tasks.db` lives)
- `notes-dir`
- Obsidian vault (`obsidian-vault`, optional)

Then writes `AGENTS.md` into the task directory and the Claude skills into
`~/.claude/skills`, both from the embedded copies. Config goes to the platform config
path used by the app (`dirs::config_dir()/task-manager/config.md`, i.e.
`~/Library/Application Support/...` on macOS). `deploy.sh` remains as the
developer-from-source path.

## 5. `task serve`

Runs the existing axum server, serving the embedded UI on `127.0.0.1:4287`
(`TASK_SERVER_PORT` overrides). Runs in the foreground; no daemonization.

## 6. Shared updater (`src/update.rs`)

One module, used by `task update` and `POST /api/update`:

1. `latest_release()` queries the releases repo's latest-release API (unauthenticated).
2. Compare against `CARGO_PKG_VERSION` using semver.
3. Download the archive for this platform and `SHA256SUMS`; verify the checksum.
4. Extract to a temp file in the install directory and `rename` over the existing
   `task` and `task-tui` (atomic on the same filesystem), so a failure never leaves a
   broken install.
5. Refresh `AGENTS.md` and skills from the new embedded copies, asking before
   overwriting files the user edited (CLI) or skipping edited files and reporting them
   (UI).

Errors are returned as typed variants (no network, no release for this platform, checksum
mismatch, install directory not writable) and leave the install untouched.

**CLI.** `task update` performs the update. `task update --check` only reports. If a
`task serve` process is running it prints "restart `task serve` to finish".

## 7. Web UI: version display and update button

**Endpoints**

- `GET /api/version` returns
  `{ "current", "latest", "update_available", "update_supported" }`.
  `latest` comes from the shared updater and is cached in the server for about one hour
  (the unauthenticated GitHub limit is 60 requests/hour). If the check fails, `latest`
  is `null` and `update_available` is false.
- `POST /api/update` starts the shared updater in the background and returns `202`
  immediately (`409` if an update is already running).
- `GET /api/update/status` returns the current phase: `idle`, `downloading`, `verifying`,
  `installing`, `restarting`, or `failed` with a typed error. The UI polls it. When the
  phase reaches `restarting` the server re-execs (below).

**`update_supported`** is false unless the running executable is an installed release
build (not `cargo run`, not a debug build). The UI hides the button in that case.

**Security.** `POST /api/update` replaces executables, so beyond the existing loopback
bind and host-header check it requires a custom request header
(`X-Command-Center: update`) that a cross-site page cannot send without a CORS preflight,
which the server does not allow. Requests without it get 403.

**Restart.** After a successful install the server re-execs the newly installed binary
as `task serve` with the same port and database arguments. Listening sockets are
close-on-exec, so the port is released and rebound by the new process. In-flight requests
and the live-update (SSE) stream drop for about a second or two.

**UI.**

- A small header/footer element shows `v<current>`.
- When `update_available`, it shows `v<latest> available` and an **Update** button.
- Clicking runs `POST /api/update`, polls `GET /api/update/status` to show the phase text, then polls `GET /api/version`
  until `current` equals the target version and reloads the page. If it does not come
  back within a timeout, it shows an error telling the user to restart `task serve`.
- Typed errors are shown inline.

## 8. Testing

- Updater: unit tests with a mock releases server (`mockito`, already a dev-dependency)
  covering up-to-date, newer available, checksum mismatch, missing platform asset, no
  network, and non-writable install directory; atomic replace verified on a temp dir.
- Version logic: semver comparison and the 1-hour cache.
- `/api/version` and `/api/update`: router tests, including the missing custom header
  returning 403 and `update_supported = false` in test builds.
- `task setup`: tests against a temp config path for idempotency and the
  no-overwrite-without-asking rule.
- `install.sh`: a shellcheck run in CI, plus a CI job that installs the just-built
  artifact on a clean runner and runs `task --version`.
- Release workflow: exercised end to end with a pre-release tag before the first real
  release.

## 9. Documentation

- `README.md`: replace the build-from-source-first instructions with the install and
  update commands; keep a developer section.
- `AGENTS.md`: no `tasks.md` format change, so no update required by `CLAUDE.md`'s sync
  rule beyond mentioning it is installed by `task setup`.

## Open items (need owner confirmation before implementation)

1. Name and owner of the public releases repo (assumed
   `TheSophist1976/command-center-releases`), and that it will be created.
2. Creating the scoped token and adding it as a secret in the private repo. Neither is
   created without explicit approval.
3. Whether `task update` should print a once-a-day "update available" notice (default:
   off).
