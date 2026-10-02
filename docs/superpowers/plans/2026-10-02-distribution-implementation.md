# Distribution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship command-center as a single self-contained `task` binary (web UI embedded), built and released by GitHub Actions to a public releases repo, installable with one command and updatable from the CLI or a web UI button.

**Architecture:** `task` gains `serve`, `setup`, `update` (plus a hidden `refresh-files`) subcommands. The built `web/dist`, `AGENTS.md` and `skills/` are embedded with `rust-embed` (read from disk in debug builds). One updater module (`src/update.rs`) is used by `task update` and by the server's `POST /api/update`; the server wraps it with a cached version check, phase/status tracking and an automatic re-exec. A tag push builds four native targets, smoke-tests `install.sh` on clean runners, and publishes to the public releases repo.

**Tech Stack:** Rust 2024 (axum 0.7, tokio, reqwest blocking + rustls, rust-embed 8, semver, sha2, flate2, tar, mockito for tests), React/Vite web UI, POSIX `sh` installer, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-09-30-distribution-design.md`

## Global Constraints

- Source repo stays private; a separate **public releases repo** holds only binaries and `install.sh`.
- Platforms: macOS (arm64, x64) and Linux (x64, arm64). No Windows.
- One self-contained `task` binary with the web UI embedded. Subcommands: `serve`, `setup`, `update`.
- Signing: none. SHA-256 checksum verification only.
- Todoist is not part of `task setup`; existing `task auth todoist` code is left untouched.
- Tag `vX.Y.Z` triggers `.github/workflows/release.yml`; the first step fails unless the tag equals `version` in `Cargo.toml`.
- `cargo test` and the web build (`npm ci && npm run build`, which includes `tsc -b`) must pass before anything is published.
- Native runners, no cross-compilation (`rusqlite` is bundled and needs a C toolchain). Targets: `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`.
- Each job builds the web UI first, then `cargo build --release --features tui`.
- `reqwest` uses `default-features = false`, features `blocking`, `json`, `rustls-tls`.
- Artifacts: `task-vX.Y.Z-<target>.tar.gz` containing `task` and `task-tui`, plus a single `SHA256SUMS` covering every archive.
- CI authenticates to the releases repo with a fine-grained token stored as a repository secret, contents-write on the releases repo only. `install.sh` lives in the releases repo.
- Debug builds read assets from `web/dist` on disk so `./start-web.sh --dev` and `cargo run` are unchanged; `task_server` stays as a thin alias for `task serve`.
- Current version is `CARGO_PKG_VERSION`.
- `install.sh`: install into `~/.local/bin` (`INSTALL_DIR` overrides); offer to add the dir to the shell profile idempotently; contains no configuration logic; prints "run `task setup`".
- `task setup`: interactive, idempotent, never overwrites an existing value without asking; sets `default-dir`, `notes-dir`, optional `obsidian-vault`; config path is `dirs::config_dir()/task-manager/config.md`; `deploy.sh` remains the from-source path.
- `task serve`: foreground, `127.0.0.1:4287`, `TASK_SERVER_PORT` overrides, no daemonization.
- Updater: unauthenticated releases API; semver compare; verify checksum; extract to a temp file in the install dir then `rename` (atomic); typed errors (no network, no release for this platform, checksum mismatch, install directory not writable) leave the install untouched.
- `GET /api/version` returns `{ current, latest, update_available, update_supported }`; `latest` cached ~1 hour; failure gives `latest: null`, `update_available: false`.
- `POST /api/update` returns `202` immediately (`409` if running); `GET /api/update/status` phases: `idle`, `downloading`, `verifying`, `installing`, `restarting`, `failed` (with typed error). `POST /api/update` requires header `X-Command-Center: update` (else `403`) on top of loopback bind and host-header check.
- `update_supported` is false unless the running executable is an installed release build (not `cargo run`, not debug).
- After install the server re-execs the new binary as `task serve` with the same port and database arguments.
- Non-goals: Windows, signing/notarization, auto-update without user action, launchd/systemd units, Homebrew tap, removing Todoist code.

## Design clarifications (the spec is silent; decided here)

1. **Refreshing AGENTS.md/skills after an update.** The running (old) process only embeds the *old* copies, so after replacing binaries the updater runs the **newly installed** `task refresh-files` (hidden subcommand). CLI passes through stdin so it can ask; the server passes `--skip-edited` and reports skipped files in the status.
2. **"User edited" detection.** `installed-files.json` next to `config.md` records the SHA-256 of what we last wrote per path. A file is "edited" if its current hash differs from both the new content and the recorded hash. A pre-existing file with no record is treated as edited (conservative).
3. **Linux `current_exe()`** returns `… (deleted)` after the binary is replaced under a running process, so the server captures the exe path once at startup.

## Review Focus

Failure modes the spec implies that no single spec section tests; each is pinned by a test in the owning task.

1. **Corrupt/truncated archive or checksum mismatch** must leave existing binaries byte-identical and no `.update-*` temp files behind (Task 7).
2. **Releases API rate-limited (HTTP 403), returns garbage JSON, or has no assets** must never panic or break the UI: `/api/version` still returns 200 with `latest: null` (Tasks 7, 8).
3. **Tag strings** with a `v` prefix, prerelease suffixes (`4.1.0-rc.1`), or non-semver junk must parse or fail cleanly, and a prerelease must not count as newer than its final release (Task 6).
4. **User-edited `AGENTS.md`/skills are never silently overwritten**, and paths containing spaces work (Task 4).
5. **Double `POST /api/update` → 409; missing header → 403; after a failed update a new attempt is allowed** (Task 8).

---

## File Structure

| File | Responsibility |
| --- | --- |
| `Cargo.toml` | New deps; reqwest → rustls (Task 1) |
| `build.rs` (new) | Ensure `web/dist` exists so `rust-embed` compiles |
| `src/assets.rs` (new) | Embedded web UI, `AGENTS.md`, skills |
| `src/prompt.rs` (new) | `Prompter` trait, stdin impl, scripted test impl |
| `src/managed.rs` (new) | Install managed files with edited-file protection; `sha256_hex` |
| `src/setup.rs` (new) | `task setup` |
| `src/update.rs` (new) | Pure updater: versions, checksums, release lookup, install, `perform_update`, CLI entry |
| `src/update_state.rs` (new) | Server-side cache, status, background job, restart |
| `src/server.rs` | `serve()`, embedded static handler, version/update endpoints |
| `src/cli.rs`, `src/bin/task.rs` | New subcommands |
| `src/bin/task_server.rs` | Thin alias for `serve` |
| `web/src/{types.ts,api.ts,components/VersionBadge.tsx,App.tsx}` | Version/update UI |
| `install.sh`, `tests/install_sh_test.sh` (new) | Installer and its test |
| `.github/workflows/release.yml` (new) | Release pipeline |
| `README.md`, `AGENTS.md` | Docs |

---

### Task 1: Dependencies and rustls

**Files:**
- Modify: `Cargo.toml`

**Interfaces:**
- Produces: crates `rust-embed`, `semver`, `sha2`, `flate2`, `tar` available to all later tasks; Linux builds with no OpenSSL.

- [ ] **Step 1: Edit `Cargo.toml` dependencies**

Replace the `reqwest` line and add the new crates:

```toml
reqwest = { version = "0.12", default-features = false, features = ["blocking", "json", "rustls-tls"] }
rust-embed = { version = "8", features = ["mime-guess"] }
semver = "1"
sha2 = "0.10"
flate2 = "1"
tar = "0.4"
```

- [ ] **Step 2: Verify it builds and OpenSSL is gone**

Run: `cargo build 2>&1 | tail -3 && cargo tree -e normal -i openssl-sys 2>&1 | head -3`
Expected: build `Finished`; `cargo tree` prints an error like `package ID specification 'openssl-sys' did not match any packages`.

- [ ] **Step 3: Verify existing tests still pass (Todoist uses reqwest)**

Run: `cargo test --lib todoist 2>&1 | tail -4`
Expected: `test result: ok`.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "build: switch reqwest to rustls and add release/update dependencies"
```

---

### Task 2: Embedded assets

**Files:**
- Create: `build.rs`, `src/assets.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Produces:
  - `assets::AGENTS_MD: &str`
  - `assets::ManagedFile { pub rel_path: String, pub contents: Vec<u8> }`
  - `assets::skill_files() -> Vec<ManagedFile>` (`rel_path` like `task-manager/SKILL.md`)
  - `assets::web_asset(path: &str) -> Option<(Vec<u8>, String)>` (bytes, mime type)

- [ ] **Step 1: Write `build.rs`**

```rust
fn main() {
    // rust-embed's derive fails to compile if the folder is missing.
    std::fs::create_dir_all("web/dist").expect("create web/dist");
    println!("cargo:rerun-if-changed=web/dist");
    println!("cargo:rerun-if-changed=AGENTS.md");
    println!("cargo:rerun-if-changed=skills");
}
```

- [ ] **Step 2: Write the failing tests in `src/assets.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_files_include_task_manager_skill() {
        let files = skill_files();
        assert!(files.iter().any(|f| f.rel_path == "task-manager/SKILL.md"));
        assert!(files.iter().all(|f| !f.rel_path.ends_with(".DS_Store")));
    }

    #[test]
    fn agents_md_is_embedded() {
        assert!(AGENTS_MD.contains("needs-input"));
    }

    #[test]
    fn unknown_web_asset_is_none() {
        assert!(web_asset("definitely/not/here.js").is_none());
    }
}
```

Add `pub mod assets;` to `src/lib.rs`.

- [ ] **Step 3: Run to verify failure**

Run: `cargo test --lib assets 2>&1 | tail -6`
Expected: FAIL (compile error: `skill_files` not found).

- [ ] **Step 4: Implement above the tests in `src/assets.rs`**

```rust
use rust_embed::RustEmbed;

/// The built web UI. Debug builds read `web/dist` from disk; release builds embed it.
#[derive(RustEmbed)]
#[folder = "web/dist"]
struct WebAssets;

#[derive(RustEmbed)]
#[folder = "skills"]
#[exclude = ".DS_Store"]
struct SkillAssets;

pub const AGENTS_MD: &str = include_str!("../AGENTS.md");

pub struct ManagedFile {
    pub rel_path: String,
    pub contents: Vec<u8>,
}

pub fn skill_files() -> Vec<ManagedFile> {
    SkillAssets::iter()
        .filter_map(|name| {
            let file = SkillAssets::get(&name)?;
            Some(ManagedFile { rel_path: name.to_string(), contents: file.data.into_owned() })
        })
        .collect()
}

pub fn web_asset(path: &str) -> Option<(Vec<u8>, String)> {
    let file = WebAssets::get(path)?;
    let mime = file.metadata.mimetype().to_string();
    Some((file.data.into_owned(), mime))
}
```

- [ ] **Step 5: Run to verify pass**

Run: `cargo test --lib assets 2>&1 | tail -4`
Expected: `3 passed`.

- [ ] **Step 6: Commit**

```bash
git add build.rs src/assets.rs src/lib.rs
git commit -m "feat: embed web UI, AGENTS.md and skills in the binary"
```

---

### Task 3: `task serve` and the `task_server` alias

**Files:**
- Modify: `src/server.rs` (router split, `serve`, embedded handler), `src/cli.rs`, `src/bin/task.rs`, `src/bin/task_server.rs`

**Interfaces:**
- Consumes: `assets::web_asset` (Task 2).
- Produces:
  - `server::router_embedded(state: AppState) -> Router`
  - `server::serve(db_path: PathBuf) -> Result<(), String>` (async)
  - `cli::Command::Serve`

- [ ] **Step 1: Write failing tests** at the end of the `tests` module in `src/server.rs`

```rust
#[tokio::test]
async fn test_embedded_router_unknown_asset_is_404_and_api_still_works() {
    let (_dir, state) = make_state();
    let app = router_embedded(state);
    let missing = app.clone()
        .oneshot(Request::builder().uri("/no-such-file.js").header("host", "127.0.0.1").body(Body::empty()).unwrap())
        .await.unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    let api = app
        .oneshot(Request::builder().uri("/api/tasks").header("host", "127.0.0.1").body(Body::empty()).unwrap())
        .await.unwrap();
    assert_eq!(api.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_embedded_router_still_rejects_spoofed_host() {
    let (_dir, state) = make_state();
    let response = router_embedded(state)
        .oneshot(Request::builder().uri("/").header("host", "evil.com").body(Body::empty()).unwrap())
        .await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
```

(Check the spoofed-host test at `test_host_header_rejects_spoofed_host` for the exact status it expects and match it.)

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib test_embedded_router 2>&1 | tail -5`
Expected: FAIL (`router_embedded` not found).

- [ ] **Step 3: Refactor the router in `src/server.rs`**

Replace `router_with_static` (the function body starting at `let state = Arc::new(state);` through `.with_state(state)`) with:

```rust
fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/tasks", get(list_tasks).post(add_task))
        .route("/api/tasks/:id", get(get_task).patch(edit_task).delete(delete_task))
        .route("/api/tasks/:id/done", axum::routing::post(done_task))
        .route("/api/tasks/:id/reopen", axum::routing::post(reopen_task))
        .route("/api/tasks/:id/notes", get(list_task_notes).post(create_task_note))
        .route("/api/tasks/:id/notes/:slug", axum::routing::delete(unlink_task_note))
        .route("/api/tasks/:id/review", get(get_task_review).post(add_task_review_feedback))
        .route("/api/tasks/:id/question", get(get_task_question).post(answer_task_question))
        .route("/api/notes/:slug/open", axum::routing::post(open_note))
        .route("/api/agents", get(list_agents))
        .route("/api/agents/:name/instructions", get(get_agent_instructions).put(edit_agent_instructions))
        .route("/api/agents/:name/memory", get(get_agent_memory).put(edit_agent_memory))
        .route("/api/events", get(task_events))
}

pub fn router_with_static(state: AppState, static_dir: Option<std::path::PathBuf>) -> Router {
    let mut app = api_routes();
    if let Some(dir) = static_dir {
        app = app.fallback_service(tower_http::services::ServeDir::new(dir));
    }
    app.layer(middleware::from_fn(validate_host)).with_state(Arc::new(state))
}

/// Serves the web UI embedded in the binary (read from `web/dist` in debug builds).
pub fn router_embedded(state: AppState) -> Router {
    api_routes()
        .fallback(embedded_handler)
        .layer(middleware::from_fn(validate_host))
        .with_state(Arc::new(state))
}

async fn embedded_handler(uri: axum::http::Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    match crate::assets::web_asset(path) {
        Some((bytes, mime)) => ([(axum::http::header::CONTENT_TYPE, mime)], bytes).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
```

(If Task 8 has already added routes, keep them in `api_routes` too.)

- [ ] **Step 4: Add `serve` to `src/server.rs`** (above `#[cfg(test)]`)

```rust
/// Runs the web server in the foreground until the process exits.
pub async fn serve(db_path: PathBuf) -> Result<(), String> {
    db::backup_daily(&db_path);
    let notes_dir = db::resolve_notes_dir(&db_path);
    let port: u16 = std::env::var("TASK_SERVER_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4287);

    println!("task serve: serving {} on http://127.0.0.1:{}", db_path.display(), port);

    let (change_tx, _) = tokio::sync::broadcast::channel(16);
    // Held for the lifetime of the process — dropping it would stop the watch.
    let _watcher = match crate::watch::spawn(db_path.clone(), change_tx.clone()) {
        Ok(w) => Some(w),
        Err(e) => {
            eprintln!("task serve: failed to watch {} for changes — live updates disabled: {}", db_path.display(), e);
            None
        }
    };

    let app = router_embedded(AppState {
        notes_dir,
        db_path,
        write_lock: Arc::new(tokio::sync::Mutex::new(())),
        change_tx,
    });
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|e| format!("Failed to bind 127.0.0.1:{}: {}", port, e))?;
    axum::serve(listener, app).await.map_err(|e| e.to_string())
}
```

- [ ] **Step 5: Replace `src/bin/task_server.rs`**

```rust
#[tokio::main]
async fn main() {
    let db_path = task::db::resolve_file_path(std::env::args().nth(1).as_deref());
    if let Err(e) = task::server::serve(db_path).await {
        eprintln!("task_server: {}", e);
        std::process::exit(1);
    }
}
```

- [ ] **Step 6: Add the CLI subcommand**

In `src/cli.rs`, add to `enum Command` (next to `Tui`):

```rust
    /// Run the web UI server (foreground, http://127.0.0.1:4287)
    Serve,
```

In `src/bin/task.rs` `run`, add *before* `let path = ...` is not possible (path needed); instead add right after `let path = task::db::resolve_file_path(...)` and **before** `task::db::backup_daily(&path);`:

```rust
    if matches!(cli.command, Some(Command::Serve)) {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| (1, e.to_string()))?;
        return runtime.block_on(task::server::serve(path)).map_err(|e| (1, e));
    }
```

and add `Some(Command::Serve) => unreachable!("handled above"),` to the `match cli.command` (the compiler will point at the spot).

- [ ] **Step 7: Run tests and a manual smoke**

Run: `cargo test --lib test_embedded_router 2>&1 | tail -4` → `2 passed`.
Run: `cargo build --bin task --bin task_server 2>&1 | tail -2` → `Finished`.
Manual: `TASK_SERVER_PORT=4999 cargo run --bin task -- --file "$TMPDIR/t.db" serve &` then `curl -s -o /dev/null -w '%{http_code}\n' localhost:4999/api/tasks` → `200`; `kill %1`.

- [ ] **Step 8: Commit**

```bash
git add src/server.rs src/cli.rs src/bin/task.rs src/bin/task_server.rs
git commit -m "feat: task serve with embedded UI; task_server becomes an alias"
```

---

### Task 4: Prompter and managed-file installer

**Files:**
- Create: `src/prompt.rs`, `src/managed.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Produces:
  - `prompt::Prompter { fn confirm(&mut self, question: &str, default_yes: bool) -> bool; fn ask(&mut self, question: &str, default: Option<&str>) -> String }`
  - `prompt::StdioPrompter`, `prompt::testing::ScriptedPrompter::new(&[&str])` (`#[cfg(test)]`, field `questions: Vec<String>`)
  - `managed::sha256_hex(&[u8]) -> String`
  - `managed::EditedPolicy<'a> { Ask(&'a mut dyn Prompter), Skip }`
  - `managed::InstallReport { installed, unchanged, skipped_edited: Vec<PathBuf> }`
  - `managed::install_managed_files(files: &[(PathBuf, Vec<u8>)], manifest: &Path, policy: &mut EditedPolicy) -> Result<InstallReport, String>`
  - `managed::managed_files(config_file: &Path, home: &Path) -> Vec<(PathBuf, Vec<u8>)>`
  - `managed::manifest_path(config_file: &Path) -> PathBuf`
  - `managed::refresh(config_file: &Path, home: &Path, policy: &mut EditedPolicy) -> Result<InstallReport, String>`

- [ ] **Step 1: Write `src/prompt.rs`**

```rust
use std::io::{BufRead, Write};

pub trait Prompter {
    fn confirm(&mut self, question: &str, default_yes: bool) -> bool;
    /// Returns the typed line, or `default` (or empty) if the user just pressed Enter.
    fn ask(&mut self, question: &str, default: Option<&str>) -> String;
}

pub struct StdioPrompter;

impl Prompter for StdioPrompter {
    fn confirm(&mut self, question: &str, default_yes: bool) -> bool {
        let hint = if default_yes { "[Y/n]" } else { "[y/N]" };
        print!("▸ {} {} ", question, hint);
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        let _ = std::io::stdin().lock().read_line(&mut line);
        match line.trim().to_lowercase().as_str() {
            "" => default_yes,
            s => s.starts_with('y'),
        }
    }

    fn ask(&mut self, question: &str, default: Option<&str>) -> String {
        match default {
            Some(d) => print!("▸ {} [{}]: ", question, d),
            None => print!("▸ {}: ", question),
        }
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        let _ = std::io::stdin().lock().read_line(&mut line);
        let line = line.trim();
        if line.is_empty() { default.unwrap_or("").to_string() } else { line.to_string() }
    }
}

#[cfg(test)]
pub mod testing {
    use super::Prompter;
    use std::collections::VecDeque;

    pub struct ScriptedPrompter {
        answers: VecDeque<String>,
        pub questions: Vec<String>,
    }

    impl ScriptedPrompter {
        pub fn new(answers: &[&str]) -> Self {
            Self { answers: answers.iter().map(|s| s.to_string()).collect(), questions: Vec::new() }
        }
    }

    impl Prompter for ScriptedPrompter {
        fn confirm(&mut self, question: &str, default_yes: bool) -> bool {
            self.questions.push(question.to_string());
            match self.answers.pop_front().unwrap_or_default().as_str() {
                "" => default_yes,
                s => s.starts_with('y'),
            }
        }

        fn ask(&mut self, question: &str, default: Option<&str>) -> String {
            self.questions.push(question.to_string());
            let a = self.answers.pop_front().unwrap_or_default();
            if a.is_empty() { default.unwrap_or("").to_string() } else { a }
        }
    }
}
```

Add `pub mod prompt; pub mod managed;` to `src/lib.rs`.

- [ ] **Step 2: Write the failing tests in `src/managed.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompt::testing::ScriptedPrompter;
    use tempfile::tempdir;

    fn files(dir: &Path, body: &str) -> Vec<(PathBuf, Vec<u8>)> {
        vec![(dir.join("my dir").join("AGENTS.md"), body.as_bytes().to_vec())]
    }

    #[test]
    fn fresh_install_writes_file_and_records_hash_even_with_spaces_in_path() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("installed-files.json");
        let f = files(dir.path(), "v1");
        let report = install_managed_files(&f, &manifest, &mut EditedPolicy::Skip).unwrap();
        assert_eq!(report.installed.len(), 1);
        assert_eq!(std::fs::read_to_string(&f[0].0).unwrap(), "v1");
        assert!(manifest.exists());
    }

    #[test]
    fn unmodified_previous_install_is_overwritten_silently() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("installed-files.json");
        install_managed_files(&files(dir.path(), "v1"), &manifest, &mut EditedPolicy::Skip).unwrap();
        let f2 = files(dir.path(), "v2");
        let report = install_managed_files(&f2, &manifest, &mut EditedPolicy::Skip).unwrap();
        assert_eq!(report.installed.len(), 1);
        assert!(report.skipped_edited.is_empty());
        assert_eq!(std::fs::read_to_string(&f2[0].0).unwrap(), "v2");
    }

    #[test]
    fn identical_content_is_reported_unchanged() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("installed-files.json");
        let f = files(dir.path(), "v1");
        install_managed_files(&f, &manifest, &mut EditedPolicy::Skip).unwrap();
        let report = install_managed_files(&f, &manifest, &mut EditedPolicy::Skip).unwrap();
        assert_eq!(report.unchanged.len(), 1);
    }

    #[test]
    fn edited_file_is_skipped_under_skip_policy() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("installed-files.json");
        let f1 = files(dir.path(), "v1");
        install_managed_files(&f1, &manifest, &mut EditedPolicy::Skip).unwrap();
        std::fs::write(&f1[0].0, "my edits").unwrap();
        let report = install_managed_files(&files(dir.path(), "v2"), &manifest, &mut EditedPolicy::Skip).unwrap();
        assert_eq!(report.skipped_edited.len(), 1);
        assert_eq!(std::fs::read_to_string(&f1[0].0).unwrap(), "my edits");
    }

    #[test]
    fn edited_file_asks_and_honors_both_answers() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("installed-files.json");
        let f1 = files(dir.path(), "v1");
        install_managed_files(&f1, &manifest, &mut EditedPolicy::Skip).unwrap();
        std::fs::write(&f1[0].0, "my edits").unwrap();

        let mut no = ScriptedPrompter::new(&["n"]);
        let report = install_managed_files(&files(dir.path(), "v2"), &manifest, &mut EditedPolicy::Ask(&mut no)).unwrap();
        assert_eq!(report.skipped_edited.len(), 1);
        assert_eq!(std::fs::read_to_string(&f1[0].0).unwrap(), "my edits");
        assert_eq!(no.questions.len(), 1);

        let mut yes = ScriptedPrompter::new(&["y"]);
        install_managed_files(&files(dir.path(), "v2"), &manifest, &mut EditedPolicy::Ask(&mut yes)).unwrap();
        assert_eq!(std::fs::read_to_string(&f1[0].0).unwrap(), "v2");
    }

    #[test]
    fn existing_file_without_manifest_record_is_treated_as_edited() {
        let dir = tempdir().unwrap();
        let manifest = dir.path().join("installed-files.json");
        let f = files(dir.path(), "new");
        std::fs::create_dir_all(f[0].0.parent().unwrap()).unwrap();
        std::fs::write(&f[0].0, "hand-written").unwrap();
        let report = install_managed_files(&f, &manifest, &mut EditedPolicy::Skip).unwrap();
        assert_eq!(report.skipped_edited.len(), 1);
        assert_eq!(std::fs::read_to_string(&f[0].0).unwrap(), "hand-written");
    }

    #[test]
    fn managed_files_targets_default_dir_and_claude_skills() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.md");
        crate::config::write_config_value_to(&config, "default-dir", dir.path().join("tasks").to_str().unwrap()).unwrap();
        let home = dir.path().join("home");
        let targets = managed_files(&config, &home);
        assert!(targets.iter().any(|(p, _)| p == &dir.path().join("tasks").join("AGENTS.md")));
        assert!(targets.iter().any(|(p, _)| p == &home.join(".claude/skills/task-manager/SKILL.md")));
    }

    #[test]
    fn managed_files_skips_agents_md_when_default_dir_unset() {
        let dir = tempdir().unwrap();
        let targets = managed_files(&dir.path().join("config.md"), &dir.path().join("home"));
        assert!(targets.iter().all(|(p, _)| p.file_name().unwrap() != "AGENTS.md"));
    }
}
```

- [ ] **Step 3: Run to verify failure**

Run: `cargo test --lib managed 2>&1 | tail -5`
Expected: FAIL (compile error: items not found).

- [ ] **Step 4: Implement above the tests in `src/managed.rs`**

```rust
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::prompt::Prompter;

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{:02x}", b)).collect()
}

pub enum EditedPolicy<'a> {
    /// Ask before overwriting a file the user changed (CLI).
    Ask(&'a mut dyn Prompter),
    /// Leave edited files alone and report them (web UI).
    Skip,
}

#[derive(Debug, Default, PartialEq)]
pub struct InstallReport {
    pub installed: Vec<PathBuf>,
    pub unchanged: Vec<PathBuf>,
    pub skipped_edited: Vec<PathBuf>,
}

type Manifest = BTreeMap<String, String>;

fn load_manifest(path: &Path) -> Manifest {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_manifest(path: &Path, manifest: &Manifest) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create {}: {}", parent.display(), e))?;
    }
    let json = serde_json::to_string_pretty(manifest).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| format!("Failed to write {}: {}", path.display(), e))
}

fn write_file(dest: &Path, contents: &[u8]) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create {}: {}", parent.display(), e))?;
    }
    std::fs::write(dest, contents).map_err(|e| format!("Failed to write {}: {}", dest.display(), e))
}

pub fn install_managed_files(
    files: &[(PathBuf, Vec<u8>)],
    manifest_path: &Path,
    policy: &mut EditedPolicy,
) -> Result<InstallReport, String> {
    let mut manifest = load_manifest(manifest_path);
    let mut report = InstallReport::default();

    for (dest, contents) in files {
        let key = dest.to_string_lossy().to_string();
        let new_hash = sha256_hex(contents);

        let overwrite = match std::fs::read(dest) {
            Err(_) => true,
            Ok(current) => {
                let current_hash = sha256_hex(&current);
                if current_hash == new_hash {
                    manifest.insert(key, new_hash);
                    report.unchanged.push(dest.clone());
                    continue;
                }
                let untouched = manifest.get(&key) == Some(&current_hash);
                if untouched {
                    true
                } else {
                    match policy {
                        EditedPolicy::Skip => false,
                        EditedPolicy::Ask(p) => p.confirm(
                            &format!("{} has local changes. Overwrite with the new version?", dest.display()),
                            false,
                        ),
                    }
                }
            }
        };

        if overwrite {
            write_file(dest, contents)?;
            manifest.insert(key, new_hash);
            report.installed.push(dest.clone());
        } else {
            report.skipped_edited.push(dest.clone());
        }
    }

    save_manifest(manifest_path, &manifest)?;
    Ok(report)
}

/// Everything this binary installs on the user's machine, with its destination.
pub fn managed_files(config_file: &Path, home: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    if let Some(dir) = crate::config::read_config_value_from(config_file, "default-dir").filter(|d| !d.is_empty()) {
        out.push((crate::config::expand_tilde(&dir).join("AGENTS.md"), crate::assets::AGENTS_MD.as_bytes().to_vec()));
    }
    for f in crate::assets::skill_files() {
        out.push((home.join(".claude/skills").join(&f.rel_path), f.contents));
    }
    out
}

pub fn manifest_path(config_file: &Path) -> PathBuf {
    config_file.with_file_name("installed-files.json")
}

pub fn refresh(config_file: &Path, home: &Path, policy: &mut EditedPolicy) -> Result<InstallReport, String> {
    install_managed_files(&managed_files(config_file, home), &manifest_path(config_file), policy)
}
```

- [ ] **Step 5: Run to verify pass**

Run: `cargo test --lib managed 2>&1 | tail -4`
Expected: `7 passed`.

- [ ] **Step 6: Commit**

```bash
git add src/prompt.rs src/managed.rs src/lib.rs
git commit -m "feat: managed-file installer that never silently overwrites user edits"
```

---

### Task 5: `task setup` and `task refresh-files`

**Files:**
- Create: `src/setup.rs`
- Modify: `src/lib.rs`, `src/cli.rs`, `src/bin/task.rs`

**Interfaces:**
- Consumes: `managed::{refresh, EditedPolicy, InstallReport}`, `prompt::{Prompter, StdioPrompter}`, `config::{read_config_value_from, write_config_value_to, expand_tilde}`.
- Produces:
  - `setup::run_setup(config_file: &Path, home: &Path, prompter: &mut dyn Prompter) -> Result<managed::InstallReport, String>`
  - `cli::Command::Setup`, `cli::Command::RefreshFiles { skip_edited: bool }` (hidden)

- [ ] **Step 1: Write failing tests in `src/setup.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::read_config_value_from;
    use crate::prompt::testing::ScriptedPrompter;
    use tempfile::tempdir;

    #[test]
    fn fresh_setup_writes_config_creates_dirs_and_installs_files() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("cfg").join("config.md");
        let home = dir.path().join("home");
        let tasks = dir.path().join("my tasks");
        let mut p = ScriptedPrompter::new(&[tasks.to_str().unwrap(), "", "MyVault"]);
        run_setup(&config, &home, &mut p).unwrap();

        assert_eq!(read_config_value_from(&config, "default-dir").unwrap(), tasks.to_str().unwrap());
        assert_eq!(read_config_value_from(&config, "notes-dir").unwrap(), format!("{}/Notes", tasks.display()));
        assert_eq!(read_config_value_from(&config, "obsidian-vault").unwrap(), "MyVault");
        assert!(tasks.join("Notes").is_dir());
        assert!(tasks.join("AGENTS.md").is_file());
        assert!(home.join(".claude/skills/task-manager/SKILL.md").is_file());
    }

    #[test]
    fn rerun_keeps_existing_values_unless_user_says_change() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.md");
        let home = dir.path().join("home");
        let tasks = dir.path().join("tasks");
        let mut first = ScriptedPrompter::new(&[tasks.to_str().unwrap(), "", ""]);
        run_setup(&config, &home, &mut first).unwrap();

        // "Change?" for default-dir and notes-dir (both no); the vault was left blank so it is asked fresh.
        let mut second = ScriptedPrompter::new(&["n", "n", ""]);
        run_setup(&config, &home, &mut second).unwrap();
        assert_eq!(read_config_value_from(&config, "default-dir").unwrap(), tasks.to_str().unwrap());
    }

    #[test]
    fn changing_an_existing_value_requires_confirmation() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.md");
        let home = dir.path().join("home");
        let old = dir.path().join("old");
        let new = dir.path().join("new");
        let mut first = ScriptedPrompter::new(&[old.to_str().unwrap(), "", ""]);
        run_setup(&config, &home, &mut first).unwrap();

        let mut second = ScriptedPrompter::new(&["y", new.to_str().unwrap(), "n", ""]);
        run_setup(&config, &home, &mut second).unwrap();
        assert_eq!(read_config_value_from(&config, "default-dir").unwrap(), new.to_str().unwrap());
    }
}
```

Add `pub mod setup;` to `src/lib.rs`.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib setup 2>&1 | tail -5`
Expected: FAIL (`run_setup` not found).

- [ ] **Step 3: Implement above the tests**

```rust
use std::path::Path;

use crate::config::{expand_tilde, read_config_value_from, write_config_value_to};
use crate::managed::{self, EditedPolicy, InstallReport};
use crate::prompt::Prompter;

/// Asks for one config value. An existing value is only replaced if the user says so.
/// Returns the value now in effect, if any.
fn configure_key(
    config_file: &Path,
    key: &str,
    question: &str,
    suggested: Option<&str>,
    optional: bool,
    prompter: &mut dyn Prompter,
) -> Result<Option<String>, String> {
    let existing = read_config_value_from(config_file, key).filter(|v| !v.is_empty());
    if let Some(current) = &existing {
        if !prompter.confirm(&format!("{} is currently {}. Change it?", key, current), false) {
            return Ok(existing);
        }
    }
    let answer = prompter.ask(question, suggested.or(existing.as_deref()));
    if answer.is_empty() {
        if optional {
            return Ok(existing);
        }
        return Err(format!("{} is required", key));
    }
    write_config_value_to(config_file, key, &answer)?;
    Ok(Some(answer))
}

pub fn run_setup(config_file: &Path, home: &Path, prompter: &mut dyn Prompter) -> Result<InstallReport, String> {
    let default_dir = configure_key(config_file, "default-dir", "Directory for tasks.db", Some("~/tasks"), false, prompter)?;
    let notes_suggestion = default_dir.as_ref().map(|d| format!("{}/Notes", d.trim_end_matches('/')));
    configure_key(config_file, "notes-dir", "Directory for notes", notes_suggestion.as_deref(), false, prompter)?;
    configure_key(config_file, "obsidian-vault", "Obsidian vault name (blank to skip)", None, true, prompter)?;

    for key in ["default-dir", "notes-dir"] {
        if let Some(v) = read_config_value_from(config_file, key).filter(|v| !v.is_empty()) {
            let dir = expand_tilde(&v);
            std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create {}: {}", dir.display(), e))?;
        }
    }

    managed::refresh(config_file, home, &mut EditedPolicy::Ask(prompter))
}
```

(The third test script `["y", new, "n", ""]` = change default-dir yes → new value; notes-dir keep? "n"; vault blank → asked fresh → "". Check the question order matches `configure_key` order if the test fails and fix the script, not the code.)

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --lib setup 2>&1 | tail -4`
Expected: `3 passed`.

- [ ] **Step 5: Wire the CLI**

In `src/cli.rs` add to `enum Command`:

```rust
    /// Configure task directories and install AGENTS.md and Claude skills
    Setup,
    /// Re-install AGENTS.md and Claude skills from this binary (used by `task update`)
    #[command(hide = true)]
    RefreshFiles {
        /// Leave user-edited files alone instead of asking
        #[arg(long)]
        skip_edited: bool,
    },
```

In `src/bin/task.rs` `run`, next to the `Serve` early-return (before `backup_daily`):

```rust
    match &cli.command {
        Some(Command::Setup) => {
            let config = task::config::config_path().ok_or((1, "Config directory unavailable on this platform".to_string()))?;
            let home = dirs::home_dir().ok_or((1, "Home directory unavailable".to_string()))?;
            let report = task::setup::run_setup(&config, &home, &mut task::prompt::StdioPrompter).map_err(|e| (1, e))?;
            println!("Setup complete. Installed {} file(s); {} unchanged; {} left as edited.", report.installed.len(), report.unchanged.len(), report.skipped_edited.len());
            return Ok(());
        }
        Some(Command::RefreshFiles { skip_edited }) => {
            let config = task::config::config_path().ok_or((1, "Config directory unavailable on this platform".to_string()))?;
            let home = dirs::home_dir().ok_or((1, "Home directory unavailable".to_string()))?;
            let mut stdio = task::prompt::StdioPrompter;
            let mut policy = if *skip_edited { task::managed::EditedPolicy::Skip } else { task::managed::EditedPolicy::Ask(&mut stdio) };
            let report = task::managed::refresh(&config, &home, &mut policy).map_err(|e| (1, e))?;
            for p in &report.skipped_edited {
                println!("skipped-edited: {}", p.display());
            }
            return Ok(());
        }
        _ => {}
    }
```

Add `dirs = "5"` is already a dependency. Add `Some(Command::Setup | Command::RefreshFiles { .. }) => unreachable!("handled above"),` to the main match.

- [ ] **Step 6: Manual check**

Run: `TASK_CONFIG_FILE="$TMPDIR/cfg/config.md" HOME="$TMPDIR/h" cargo run --bin task -- setup < /dev/null 2>&1 | tail -3`
Expected: succeeds (empty stdin accepts every suggested default), prints `Setup complete. Installed …`, and creates `$TMPDIR/h/tasks/Notes`, `$TMPDIR/h/tasks/AGENTS.md` and `$TMPDIR/h/.claude/skills/task-manager/SKILL.md`.

- [ ] **Step 7: Commit**

```bash
git add src/setup.rs src/lib.rs src/cli.rs src/bin/task.rs
git commit -m "feat: task setup and hidden refresh-files subcommand"
```

---

### Task 6: Updater — pure logic

**Files:**
- Create: `src/update.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Consumes: `managed::sha256_hex`.
- Produces (all in `update`):
  - `enum UpdateError { Network(String), BadRelease(String), NoPlatformAsset(String), ChecksumMissing(String), ChecksumMismatch(String), InstallDirNotWritable(PathBuf), Install(String) }` with `kind() -> &'static str` and `Display`
  - `enum Phase { Idle, Downloading, Verifying, Installing, Restarting, Failed }` (`Serialize`, lowercase)
  - `const DEFAULT_RELEASES_API: &str`, `fn releases_api() -> String`
  - `fn current_version() -> semver::Version`, `fn parse_version(&str) -> Result<Version, UpdateError>`, `fn is_newer(&Version, &Version) -> bool`
  - `fn target_for(os: &str, arch: &str) -> Result<&'static str, UpdateError>`, `fn target_triple() -> Result<&'static str, UpdateError>`
  - `fn archive_name(&Version, target: &str) -> String`
  - `fn parse_checksums(&str) -> HashMap<String, String>`, `fn verify_checksum(name: &str, bytes: &[u8], sums: &HashMap<String, String>) -> Result<(), UpdateError>`

- [ ] **Step 1: Write failing tests in `src/update.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version_accepts_v_prefix_and_prerelease() {
        assert_eq!(parse_version("v4.1.0").unwrap(), Version::new(4, 1, 0));
        assert_eq!(parse_version("4.1.0-rc.1").unwrap().to_string(), "4.1.0-rc.1");
    }

    #[test]
    fn parse_version_rejects_junk_with_typed_error() {
        assert!(matches!(parse_version("nightly"), Err(UpdateError::BadRelease(_))));
        assert!(matches!(parse_version(""), Err(UpdateError::BadRelease(_))));
    }

    #[test]
    fn prerelease_is_not_newer_than_its_final_release() {
        let final_release = Version::new(4, 1, 0);
        let rc = parse_version("4.1.0-rc.1").unwrap();
        assert!(!is_newer(&final_release, &rc));
        assert!(is_newer(&rc, &final_release));
        assert!(is_newer(&Version::new(4, 0, 0), &final_release));
        assert!(!is_newer(&final_release, &final_release));
    }

    #[test]
    fn target_for_maps_the_four_supported_platforms() {
        assert_eq!(target_for("macos", "aarch64").unwrap(), "aarch64-apple-darwin");
        assert_eq!(target_for("macos", "x86_64").unwrap(), "x86_64-apple-darwin");
        assert_eq!(target_for("linux", "x86_64").unwrap(), "x86_64-unknown-linux-gnu");
        assert_eq!(target_for("linux", "aarch64").unwrap(), "aarch64-unknown-linux-gnu");
        assert!(matches!(target_for("windows", "x86_64"), Err(UpdateError::NoPlatformAsset(_))));
    }

    #[test]
    fn archive_name_matches_release_workflow() {
        assert_eq!(archive_name(&Version::new(4, 1, 0), "aarch64-apple-darwin"), "task-v4.1.0-aarch64-apple-darwin.tar.gz");
    }

    #[test]
    fn checksums_parse_sha256sum_format_including_binary_marker() {
        let sums = parse_checksums("AAA  one.tar.gz\nbbb *two.tar.gz\n\nmalformed\n");
        assert_eq!(sums.get("one.tar.gz").unwrap(), "aaa");
        assert_eq!(sums.get("two.tar.gz").unwrap(), "bbb");
        assert_eq!(sums.len(), 2);
    }

    #[test]
    fn verify_checksum_accepts_match_and_rejects_mismatch_or_missing() {
        let bytes = b"hello";
        let mut sums = HashMap::new();
        sums.insert("a.tar.gz".to_string(), crate::managed::sha256_hex(bytes));
        assert!(verify_checksum("a.tar.gz", bytes, &sums).is_ok());
        assert!(matches!(verify_checksum("a.tar.gz", b"tampered", &sums), Err(UpdateError::ChecksumMismatch(_))));
        assert!(matches!(verify_checksum("b.tar.gz", bytes, &sums), Err(UpdateError::ChecksumMissing(_))));
    }

    #[test]
    fn update_error_kinds_are_stable_strings() {
        assert_eq!(UpdateError::Network("x".into()).kind(), "network");
        assert_eq!(UpdateError::ChecksumMismatch("x".into()).kind(), "checksum_mismatch");
        assert_eq!(UpdateError::InstallDirNotWritable("/x".into()).kind(), "install_dir_not_writable");
    }
}
```

Add `pub mod update;` to `src/lib.rs`.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib update:: 2>&1 | tail -5`
Expected: FAIL (items not found).

- [ ] **Step 3: Implement above the tests**

```rust
use std::collections::HashMap;
use std::path::PathBuf;

use semver::Version;
use serde::Serialize;

pub const DEFAULT_RELEASES_API: &str = "https://api.github.com/repos/TheSophist1976/command-center-releases";

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateError {
    Network(String),
    BadRelease(String),
    NoPlatformAsset(String),
    ChecksumMissing(String),
    ChecksumMismatch(String),
    InstallDirNotWritable(PathBuf),
    Install(String),
}

impl UpdateError {
    pub fn kind(&self) -> &'static str {
        match self {
            UpdateError::Network(_) => "network",
            UpdateError::BadRelease(_) => "bad_release",
            UpdateError::NoPlatformAsset(_) => "no_platform_asset",
            UpdateError::ChecksumMissing(_) => "checksum_missing",
            UpdateError::ChecksumMismatch(_) => "checksum_mismatch",
            UpdateError::InstallDirNotWritable(_) => "install_dir_not_writable",
            UpdateError::Install(_) => "install",
        }
    }
}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateError::Network(m) => write!(f, "Could not reach the release server: {}", m),
            UpdateError::BadRelease(m) => write!(f, "The release data was not understood: {}", m),
            UpdateError::NoPlatformAsset(a) => write!(f, "No release is available for this platform ({})", a),
            UpdateError::ChecksumMissing(a) => write!(f, "No checksum was published for {}", a),
            UpdateError::ChecksumMismatch(a) => write!(f, "Checksum mismatch for {} — nothing was installed", a),
            UpdateError::InstallDirNotWritable(d) => write!(f, "Cannot write to {} — reinstall into a writable directory or fix its permissions", d.display()),
            UpdateError::Install(m) => write!(f, "Install failed: {}", m),
        }
    }
}

impl std::error::Error for UpdateError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Idle,
    Downloading,
    Verifying,
    Installing,
    Restarting,
    Failed,
}

pub fn releases_api() -> String {
    std::env::var("TASK_RELEASES_API")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_RELEASES_API.to_string())
}

pub fn current_version() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).expect("CARGO_PKG_VERSION is valid semver")
}

pub fn parse_version(tag: &str) -> Result<Version, UpdateError> {
    Version::parse(tag.trim().trim_start_matches('v'))
        .map_err(|e| UpdateError::BadRelease(format!("tag {:?}: {}", tag, e)))
}

pub fn is_newer(current: &Version, latest: &Version) -> bool {
    latest > current
}

pub fn target_for(os: &str, arch: &str) -> Result<&'static str, UpdateError> {
    match (os, arch) {
        ("macos", "aarch64") => Ok("aarch64-apple-darwin"),
        ("macos", "x86_64") => Ok("x86_64-apple-darwin"),
        ("linux", "x86_64") => Ok("x86_64-unknown-linux-gnu"),
        ("linux", "aarch64") => Ok("aarch64-unknown-linux-gnu"),
        (os, arch) => Err(UpdateError::NoPlatformAsset(format!("{}-{}", arch, os))),
    }
}

pub fn target_triple() -> Result<&'static str, UpdateError> {
    target_for(std::env::consts::OS, std::env::consts::ARCH)
}

pub fn archive_name(version: &Version, target: &str) -> String {
    format!("task-v{}-{}.tar.gz", version, target)
}

pub fn parse_checksums(text: &str) -> HashMap<String, String> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let hash = parts.next()?;
            let name = parts.next()?.trim_start_matches('*');
            Some((name.to_string(), hash.to_lowercase()))
        })
        .collect()
}

pub fn verify_checksum(name: &str, bytes: &[u8], sums: &HashMap<String, String>) -> Result<(), UpdateError> {
    let expected = sums.get(name).ok_or_else(|| UpdateError::ChecksumMissing(name.to_string()))?;
    if &crate::managed::sha256_hex(bytes) == expected {
        Ok(())
    } else {
        Err(UpdateError::ChecksumMismatch(name.to_string()))
    }
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --lib update:: 2>&1 | tail -4`
Expected: `8 passed`.

- [ ] **Step 5: Commit**

```bash
git add src/update.rs src/lib.rs
git commit -m "feat: updater version, platform and checksum logic"
```

---

### Task 7: Updater — release lookup, install, `perform_update`, `task update`

**Files:**
- Modify: `src/update.rs`, `src/cli.rs`, `src/bin/task.rs`

**Interfaces:**
- Consumes: everything from Task 6; `managed::sha256_hex`.
- Produces (in `update`):
  - `struct Release { pub version: Version, pub assets: Vec<ReleaseAsset> }` + `fn asset_url(&self, name: &str) -> Option<&str>`; `struct ReleaseAsset { pub name: String, pub url: String }`
  - `fn latest_release(api_base: &str) -> Result<Release, UpdateError>`
  - `fn download(url: &str) -> Result<Vec<u8>, UpdateError>`
  - `fn install_binaries(archive: &[u8], install_dir: &Path) -> Result<Vec<String>, UpdateError>`
  - `struct UpdateOutcome { pub from: Version, pub to: Version, pub installed: Vec<String> }`
  - `fn perform_update(api_base: &str, install_dir: &Path, on_phase: &mut dyn FnMut(Phase)) -> Result<Option<UpdateOutcome>, UpdateError>` (`None` = already up to date)
  - `fn run_cli(check_only: bool) -> Result<(), String>`
  - `cli::Command::Update { check: bool }`

- [ ] **Step 1: Write failing tests** (append to `update.rs` `tests` module)

```rust
    use std::io::Write;

    fn make_archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut builder = tar::Builder::new(gz);
        for (name, data) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            builder.append_data(&mut header, name, *data).unwrap();
        }
        let gz = builder.into_inner().unwrap();
        gz.finish().unwrap()
    }

    fn leftover_temps(dir: &std::path::Path) -> Vec<String> {
        std::fs::read_dir(dir).unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .filter(|n| n.contains(".update-"))
            .collect()
    }

    #[test]
    fn install_binaries_replaces_both_binaries_atomically() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("task"), "old").unwrap();
        let archive = make_archive(&[("task", b"new-task"), ("task-tui", b"new-tui")]);
        let installed = install_binaries(&archive, dir.path()).unwrap();
        assert_eq!(installed, vec!["task".to_string(), "task-tui".to_string()]);
        assert_eq!(std::fs::read(dir.path().join("task")).unwrap(), b"new-task");
        assert_eq!(std::fs::read(dir.path().join("task-tui")).unwrap(), b"new-tui");
        assert!(leftover_temps(dir.path()).is_empty());
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(dir.path().join("task")).unwrap().permissions().mode() & 0o777, 0o755);
    }

    #[test]
    fn corrupt_archive_leaves_existing_binary_untouched_and_no_temp_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("task"), "old").unwrap();
        let mut archive = make_archive(&[("task", b"new-task")]);
        archive.truncate(archive.len() / 2);
        assert!(install_binaries(&archive, dir.path()).is_err());
        assert_eq!(std::fs::read(dir.path().join("task")).unwrap(), b"old");
        assert!(leftover_temps(dir.path()).is_empty());
    }

    #[test]
    fn archive_without_task_binary_is_rejected_and_cleans_up() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("task"), "old").unwrap();
        let archive = make_archive(&[("task-tui", b"only-tui")]);
        assert!(matches!(install_binaries(&archive, dir.path()), Err(UpdateError::Install(_))));
        assert_eq!(std::fs::read(dir.path().join("task")).unwrap(), b"old");
        assert!(!dir.path().join("task-tui").exists());
        assert!(leftover_temps(dir.path()).is_empty());
    }

    #[test]
    fn non_writable_install_dir_is_a_typed_error() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();
        let archive = make_archive(&[("task", b"new")]);
        let result = install_binaries(&archive, dir.path());
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(matches!(result, Err(UpdateError::InstallDirNotWritable(_))));
    }

    fn release_json(server_url: &str, tag: &str, target: &str) -> String {
        let v = parse_version(tag).unwrap();
        let archive = archive_name(&v, target);
        format!(
            r#"{{"tag_name":"{tag}","assets":[
                {{"name":"{archive}","browser_download_url":"{server_url}/dl/{archive}"}},
                {{"name":"SHA256SUMS","browser_download_url":"{server_url}/dl/SHA256SUMS"}}]}}"#
        )
    }

    #[test]
    fn latest_release_parses_tag_and_assets() {
        let mut server = mockito::Server::new();
        let target = target_triple().unwrap();
        server.mock("GET", "/releases/latest").with_status(200)
            .with_body(release_json(&server.url(), "v99.0.0", target)).create();
        let release = latest_release(&server.url()).unwrap();
        assert_eq!(release.version, Version::new(99, 0, 0));
        assert!(release.asset_url("SHA256SUMS").unwrap().ends_with("/dl/SHA256SUMS"));
    }

    #[test]
    fn latest_release_rate_limit_garbage_and_no_network_are_typed_errors() {
        let mut server = mockito::Server::new();
        let limited = server.mock("GET", "/releases/latest").with_status(403).create();
        assert!(matches!(latest_release(&server.url()), Err(UpdateError::Network(_))));
        limited.remove();

        server.mock("GET", "/releases/latest").with_status(200).with_body("<html>oops</html>").create();
        assert!(matches!(latest_release(&server.url()), Err(UpdateError::BadRelease(_))));

        assert!(matches!(latest_release("http://127.0.0.1:1"), Err(UpdateError::Network(_))));
    }

    #[test]
    fn latest_release_with_non_semver_tag_is_bad_release() {
        let mut server = mockito::Server::new();
        server.mock("GET", "/releases/latest").with_status(200).with_body(r#"{"tag_name":"nightly","assets":[]}"#).create();
        assert!(matches!(latest_release(&server.url()), Err(UpdateError::BadRelease(_))));
    }

    fn serve_release(server: &mut mockito::ServerGuard, archive: &[u8], sums_hash: &str) {
        let target = target_triple().unwrap();
        let name = archive_name(&Version::new(99, 0, 0), target);
        server.mock("GET", "/releases/latest").with_status(200)
            .with_body(release_json(&server.url(), "v99.0.0", target)).create();
        server.mock("GET", format!("/dl/{}", name).as_str()).with_status(200).with_body(archive).create();
        server.mock("GET", "/dl/SHA256SUMS").with_status(200).with_body(format!("{}  {}\n", sums_hash, name)).create();
    }

    #[test]
    fn perform_update_installs_when_newer_and_reports_phases() {
        let mut server = mockito::Server::new();
        let archive = make_archive(&[("task", b"v99")]);
        serve_release(&mut server, &archive, &crate::managed::sha256_hex(&archive));
        let dir = tempfile::tempdir().unwrap();
        let mut phases = Vec::new();
        let outcome = perform_update(&server.url(), dir.path(), &mut |p| phases.push(p)).unwrap().unwrap();
        assert_eq!(outcome.to, Version::new(99, 0, 0));
        assert_eq!(std::fs::read(dir.path().join("task")).unwrap(), b"v99");
        assert_eq!(phases, vec![Phase::Downloading, Phase::Verifying, Phase::Installing]);
    }

    #[test]
    fn perform_update_checksum_mismatch_installs_nothing() {
        let mut server = mockito::Server::new();
        let archive = make_archive(&[("task", b"v99")]);
        serve_release(&mut server, &archive, &"0".repeat(64));
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("task"), "old").unwrap();
        let err = perform_update(&server.url(), dir.path(), &mut |_| {}).unwrap_err();
        assert!(matches!(err, UpdateError::ChecksumMismatch(_)));
        assert_eq!(std::fs::read(dir.path().join("task")).unwrap(), b"old");
        assert!(leftover_temps(dir.path()).is_empty());
    }

    #[test]
    fn perform_update_is_a_noop_when_already_current_or_ahead() {
        let mut server = mockito::Server::new();
        server.mock("GET", "/releases/latest").with_status(200)
            .with_body(format!(r#"{{"tag_name":"v{}","assets":[]}}"#, current_version())).create();
        let dir = tempfile::tempdir().unwrap();
        assert!(perform_update(&server.url(), dir.path(), &mut |_| {}).unwrap().is_none());
    }

    #[test]
    fn perform_update_missing_platform_asset_is_typed() {
        let mut server = mockito::Server::new();
        server.mock("GET", "/releases/latest").with_status(200)
            .with_body(r#"{"tag_name":"v99.0.0","assets":[]}"#).create();
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(perform_update(&server.url(), dir.path(), &mut |_| {}), Err(UpdateError::NoPlatformAsset(_))));
    }
```

(Remove the unused `use std::io::Write;` if the compiler warns.)

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib update:: 2>&1 | tail -5`
Expected: FAIL (`latest_release` etc. not found).

- [ ] **Step 3: Implement** (append to `src/update.rs` above `#[cfg(test)]`; add `use std::path::Path;` and `use std::time::Duration;` and `use serde::Deserialize;` at the top)

```rust
#[derive(Debug, Clone)]
pub struct ReleaseAsset {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct Release {
    pub version: Version,
    pub assets: Vec<ReleaseAsset>,
}

impl Release {
    pub fn asset_url(&self, name: &str) -> Option<&str> {
        self.assets.iter().find(|a| a.name == name).map(|a| a.url.as_str())
    }
}

#[derive(Deserialize)]
struct ReleaseJson {
    tag_name: String,
    #[serde(default)]
    assets: Vec<AssetJson>,
}

#[derive(Deserialize)]
struct AssetJson {
    name: String,
    browser_download_url: String,
}

fn client(timeout_secs: u64) -> Result<reqwest::blocking::Client, UpdateError> {
    reqwest::blocking::Client::builder()
        .user_agent(concat!("task/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .map_err(|e| UpdateError::Network(e.to_string()))
}

pub fn latest_release(api_base: &str) -> Result<Release, UpdateError> {
    let url = format!("{}/releases/latest", api_base.trim_end_matches('/'));
    let response = client(15)?
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .map_err(|e| UpdateError::Network(e.to_string()))?;
    if !response.status().is_success() {
        return Err(UpdateError::Network(format!("release API returned HTTP {}", response.status())));
    }
    let parsed: ReleaseJson = response.json().map_err(|e| UpdateError::BadRelease(e.to_string()))?;
    Ok(Release {
        version: parse_version(&parsed.tag_name)?,
        assets: parsed.assets.into_iter().map(|a| ReleaseAsset { name: a.name, url: a.browser_download_url }).collect(),
    })
}

pub fn download(url: &str) -> Result<Vec<u8>, UpdateError> {
    let response = client(300)?.get(url).send().map_err(|e| UpdateError::Network(e.to_string()))?;
    if !response.status().is_success() {
        return Err(UpdateError::Network(format!("download returned HTTP {}", response.status())));
    }
    response.bytes().map(|b| b.to_vec()).map_err(|e| UpdateError::Network(e.to_string()))
}

const BINARIES: [&str; 2] = ["task", "task-tui"];

/// Extracts `task` / `task-tui` into temp files in `install_dir`, then renames them over
/// the existing binaries. Any failure before the renames removes every temp file and
/// leaves the install untouched.
pub fn install_binaries(archive: &[u8], install_dir: &Path) -> Result<Vec<String>, UpdateError> {
    use std::io::Read;
    use std::os::unix::fs::PermissionsExt;

    let io_err = |e: std::io::Error| UpdateError::Install(e.to_string());
    let mut staged: Vec<(String, PathBuf)> = Vec::new();

    let extracted = (|| -> Result<(), UpdateError> {
        let mut tar = tar::Archive::new(flate2::read::GzDecoder::new(archive));
        for entry in tar.entries().map_err(io_err)? {
            let mut entry = entry.map_err(io_err)?;
            if !entry.header().entry_type().is_file() {
                continue;
            }
            let name = match entry.path().ok().and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string())) {
                Some(n) if BINARIES.contains(&n.as_str()) => n,
                _ => continue,
            };
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(io_err)?;
            let tmp = install_dir.join(format!(".{}.update-{}", name, std::process::id()));
            std::fs::write(&tmp, &buf).map_err(|e| {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    UpdateError::InstallDirNotWritable(install_dir.to_path_buf())
                } else {
                    UpdateError::Install(e.to_string())
                }
            })?;
            staged.push((name, tmp.clone()));
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755)).map_err(io_err)?;
        }
        if !staged.iter().any(|(n, _)| n == "task") {
            return Err(UpdateError::Install("archive does not contain `task`".to_string()));
        }
        Ok(())
    })();

    let cleanup = |staged: &[(String, PathBuf)]| {
        for (_, tmp) in staged {
            let _ = std::fs::remove_file(tmp);
        }
    };

    if let Err(e) = extracted {
        cleanup(&staged);
        return Err(e);
    }

    let mut installed = Vec::new();
    for (i, (name, tmp)) in staged.iter().enumerate() {
        if let Err(e) = std::fs::rename(tmp, install_dir.join(name)) {
            cleanup(&staged[i..]);
            return Err(UpdateError::Install(e.to_string()));
        }
        installed.push(name.clone());
    }
    Ok(installed)
}

#[derive(Debug, Clone, PartialEq)]
pub struct UpdateOutcome {
    pub from: Version,
    pub to: Version,
    pub installed: Vec<String>,
}

/// Looks up the latest release and installs it if it is newer. `Ok(None)` means already
/// up to date.
pub fn perform_update(
    api_base: &str,
    install_dir: &Path,
    on_phase: &mut dyn FnMut(Phase),
) -> Result<Option<UpdateOutcome>, UpdateError> {
    let release = latest_release(api_base)?;
    let current = current_version();
    if !is_newer(&current, &release.version) {
        return Ok(None);
    }
    let archive = archive_name(&release.version, target_triple()?);
    let archive_url = release.asset_url(&archive).ok_or_else(|| UpdateError::NoPlatformAsset(archive.clone()))?;
    let sums_url = release.asset_url("SHA256SUMS").ok_or_else(|| UpdateError::ChecksumMissing("SHA256SUMS".to_string()))?;

    on_phase(Phase::Downloading);
    let bytes = download(archive_url)?;
    let sums_text = String::from_utf8_lossy(&download(sums_url)?).into_owned();

    on_phase(Phase::Verifying);
    verify_checksum(&archive, &bytes, &parse_checksums(&sums_text))?;

    on_phase(Phase::Installing);
    let installed = install_binaries(&bytes, install_dir)?;
    Ok(Some(UpdateOutcome { from: current, to: release.version, installed }))
}

fn server_is_running() -> bool {
    let port: u16 = std::env::var("TASK_SERVER_PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(4287);
    std::net::TcpStream::connect_timeout(&std::net::SocketAddr::from(([127, 0, 0, 1], port)), Duration::from_millis(200)).is_ok()
}

/// `task update` / `task update --check`.
pub fn run_cli(check_only: bool) -> Result<(), String> {
    let api = releases_api();
    let current = current_version();
    if check_only {
        let release = latest_release(&api).map_err(|e| e.to_string())?;
        if is_newer(&current, &release.version) {
            println!("Update available: v{} -> v{}. Run `task update`.", current, release.version);
        } else {
            println!("task v{} is up to date.", current);
        }
        return Ok(());
    }

    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let install_dir = exe.parent().ok_or("Cannot locate the install directory")?.to_path_buf();
    let outcome = perform_update(&api, &install_dir, &mut |p| {
        println!("▸ {}…", format!("{:?}", p).to_lowercase());
    })
    .map_err(|e| e.to_string())?;

    match outcome {
        None => println!("task v{} is already up to date.", current),
        Some(o) => {
            // The new binary carries the new AGENTS.md/skills, so it does the refresh.
            let status = std::process::Command::new(install_dir.join("task")).arg("refresh-files").status();
            if !matches!(status, Ok(s) if s.success()) {
                eprintln!("warning: could not refresh AGENTS.md/skills; run `task setup` to reinstall them");
            }
            println!("Updated task v{} -> v{}.", o.from, o.to);
            if server_is_running() {
                println!("`task serve` is running — restart it to finish the update.");
            }
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Wire the CLI**

In `src/cli.rs` add to `enum Command`:

```rust
    /// Update task to the latest release
    Update {
        /// Only report whether an update is available
        #[arg(long)]
        check: bool,
    },
```

In `src/bin/task.rs`, add to the early `match &cli.command` block (before `backup_daily`):

```rust
        Some(Command::Update { check }) => {
            return task::update::run_cli(*check).map_err(|e| (1, e));
        }
```

and `Command::Update { .. }` to the `unreachable!` arm.

- [ ] **Step 5: Run to verify pass**

Run: `cargo test --lib update:: 2>&1 | tail -4`
Expected: all `update::` tests pass (8 from Task 6 + 11 new).
Run: `cargo run --bin task -- update --check 2>&1 | tail -2`
Expected: a clean error (`Could not reach the release server…` / HTTP 404 until the releases repo exists) — not a panic.

- [ ] **Step 6: Commit**

```bash
git add src/update.rs src/cli.rs src/bin/task.rs
git commit -m "feat: task update with checksum-verified atomic install"
```

---

### Task 8: Server version/update endpoints and restart

**Files:**
- Create: `src/update_state.rs`
- Modify: `src/lib.rs`, `src/server.rs`

**Interfaces:**
- Consumes: `update::{latest_release, perform_update, releases_api, current_version, is_newer, Phase, UpdateError}`.
- Produces:
  - `update_state::UpdateState` with `new(api_base: String, exe: PathBuf, supported: bool)`, `from_env()`, `for_tests(api_base: &str, supported: bool)` (`#[cfg(test)]`), `with_cache_ttl(Duration)`, `latest_version(&self) -> Option<String>` (blocking), `status(&self) -> StatusSnapshot`, `try_begin(&self) -> bool`, `set_phase`, `fail(&UpdateError)`, `set_skipped(Vec<String>)`, public fields `supported: bool`
  - `update_state::{StatusSnapshot { phase, error: Option<ErrorInfo>, skipped_files: Vec<String> }, ErrorInfo { kind, message }, path_looks_installed(&Path) -> bool, is_installed_release(&Path) -> bool, restart_args(&Path) -> Vec<OsString>, run_update_job(&UpdateState, &Path)}`
  - `AppState.update: Arc<UpdateState>`
  - Routes `GET /api/version`, `POST /api/update`, `GET /api/update/status`

- [ ] **Step 1: Write failing unit tests in `src/update_state.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn installed_release_path_detection() {
        assert!(path_looks_installed(Path::new("/home/me/.local/bin/task")));
        assert!(!path_looks_installed(Path::new("/code/command-center/target/release/task")));
        assert!(!path_looks_installed(Path::new("/code/command-center/target/debug/task")));
    }

    #[test]
    fn restart_args_pass_the_database_and_serve() {
        let args = restart_args(Path::new("/data/my tasks/tasks.db"));
        assert_eq!(args, vec![
            std::ffi::OsString::from("--file"),
            std::ffi::OsString::from("/data/my tasks/tasks.db"),
            std::ffi::OsString::from("serve"),
        ]);
    }

    #[test]
    fn try_begin_is_exclusive_until_failure_then_allows_retry() {
        let s = UpdateState::for_tests("http://127.0.0.1:1", true);
        assert!(s.try_begin());
        assert!(!s.try_begin());
        s.fail(&crate::update::UpdateError::ChecksumMismatch("x".into()));
        let snap = s.status();
        assert_eq!(snap.phase, crate::update::Phase::Failed);
        assert_eq!(snap.error.unwrap().kind, "checksum_mismatch");
        assert!(s.try_begin(), "a failed update must not block the next attempt");
        assert!(s.status().error.is_none());
    }

    #[test]
    fn latest_version_is_cached_including_failures() {
        let mut server = mockito::Server::new();
        let ok = server.mock("GET", "/releases/latest").with_status(200)
            .with_body(r#"{"tag_name":"v99.0.0","assets":[]}"#).expect(1).create();
        let s = UpdateState::for_tests(&server.url(), false);
        assert_eq!(s.latest_version().as_deref(), Some("99.0.0"));
        assert_eq!(s.latest_version().as_deref(), Some("99.0.0"));
        ok.assert();

        let mut failing = mockito::Server::new();
        let limited = failing.mock("GET", "/releases/latest").with_status(403).expect(1).create();
        let s2 = UpdateState::for_tests(&failing.url(), false);
        assert_eq!(s2.latest_version(), None);
        assert_eq!(s2.latest_version(), None);
        limited.assert();
    }
}
```

Add `pub mod update_state;` to `src/lib.rs`.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib update_state 2>&1 | tail -5`
Expected: FAIL (items not found).

- [ ] **Step 3: Implement `src/update_state.rs`** (above the tests)

```rust
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::update::{self, Phase, UpdateError};

const LATEST_CACHE_TTL: Duration = Duration::from_secs(3600);
/// A failed lookup (rate limit, offline) is cached briefly so the UI cannot hammer the API.
const FAILURE_CACHE_TTL: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ErrorInfo {
    pub kind: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StatusSnapshot {
    pub phase: Phase,
    pub error: Option<ErrorInfo>,
    pub skipped_files: Vec<String>,
}

pub struct UpdateState {
    pub api_base: String,
    /// Captured at startup: on Linux `current_exe()` reads `... (deleted)` once the binary
    /// has been replaced underneath the running process.
    pub exe: PathBuf,
    pub supported: bool,
    cache_ttl: Duration,
    latest: Mutex<Option<(Instant, Option<String>)>>,
    status: Mutex<StatusSnapshot>,
}

pub fn path_looks_installed(exe: &Path) -> bool {
    !exe.components().any(|c| c.as_os_str() == "target")
}

pub fn is_installed_release(exe: &Path) -> bool {
    !cfg!(debug_assertions) && path_looks_installed(exe)
}

pub fn restart_args(db_path: &Path) -> Vec<OsString> {
    vec!["--file".into(), db_path.as_os_str().to_owned(), "serve".into()]
}

impl UpdateState {
    pub fn new(api_base: String, exe: PathBuf, supported: bool) -> Self {
        Self {
            api_base,
            exe,
            supported,
            cache_ttl: LATEST_CACHE_TTL,
            latest: Mutex::new(None),
            status: Mutex::new(StatusSnapshot { phase: Phase::Idle, error: None, skipped_files: Vec::new() }),
        }
    }

    pub fn from_env() -> Self {
        let exe = std::env::current_exe().unwrap_or_default();
        let supported = is_installed_release(&exe);
        Self::new(update::releases_api(), exe, supported)
    }

    #[cfg(test)]
    pub fn for_tests(api_base: &str, supported: bool) -> Self {
        Self::new(api_base.to_string(), PathBuf::from("/nonexistent/task"), supported)
    }

    /// Blocking: call from `spawn_blocking`.
    pub fn latest_version(&self) -> Option<String> {
        let mut cache = self.latest.lock().unwrap();
        if let Some((fetched_at, value)) = cache.as_ref() {
            let ttl = if value.is_some() { self.cache_ttl } else { FAILURE_CACHE_TTL.min(self.cache_ttl) };
            if fetched_at.elapsed() < ttl {
                return value.clone();
            }
        }
        let fetched = update::latest_release(&self.api_base).ok().map(|r| r.version.to_string());
        *cache = Some((Instant::now(), fetched.clone()));
        fetched
    }

    pub fn status(&self) -> StatusSnapshot {
        self.status.lock().unwrap().clone()
    }

    /// Claims the single update slot. False if an update is already in flight.
    pub fn try_begin(&self) -> bool {
        let mut s = self.status.lock().unwrap();
        if matches!(s.phase, Phase::Downloading | Phase::Verifying | Phase::Installing | Phase::Restarting) {
            return false;
        }
        *s = StatusSnapshot { phase: Phase::Downloading, error: None, skipped_files: Vec::new() };
        true
    }

    pub fn set_phase(&self, phase: Phase) {
        self.status.lock().unwrap().phase = phase;
    }

    pub fn set_skipped(&self, files: Vec<String>) {
        self.status.lock().unwrap().skipped_files = files;
    }

    pub fn fail(&self, error: &UpdateError) {
        let mut s = self.status.lock().unwrap();
        s.phase = Phase::Failed;
        s.error = Some(ErrorInfo { kind: error.kind().to_string(), message: error.to_string() });
    }
}

/// Runs the whole update on a background thread: install, refresh managed files with the
/// new binary, then re-exec it as `task serve`.
pub fn run_update_job(update: &UpdateState, db_path: &Path) {
    let Some(install_dir) = update.exe.parent().map(|d| d.to_path_buf()) else {
        update.fail(&UpdateError::Install("cannot locate the install directory".to_string()));
        return;
    };

    let result = update::perform_update(&update.api_base, &install_dir, &mut |p| update.set_phase(p));
    match result {
        Err(e) => update.fail(&e),
        Ok(None) => update.set_phase(Phase::Idle),
        Ok(Some(_)) => {
            update.set_skipped(refresh_files_with_new_binary(&install_dir));
            update.set_phase(Phase::Restarting);
            // Give the UI a poll or two to observe `restarting` before the port drops.
            std::thread::sleep(Duration::from_millis(1500));
            let err = restart(&update.exe, db_path);
            update.fail(&UpdateError::Install(format!("installed, but restart failed ({}); run `task serve` again", err)));
        }
    }
}

fn refresh_files_with_new_binary(install_dir: &Path) -> Vec<String> {
    std::process::Command::new(install_dir.join("task"))
        .args(["refresh-files", "--skip-edited"])
        .output()
        .map(|out| {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .filter_map(|l| l.strip_prefix("skipped-edited: ").map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// Replaces this process with the new binary. Listening sockets are close-on-exec, so the
/// port is released and rebound by the new process. Only returns on failure.
#[cfg(unix)]
fn restart(exe: &Path, db_path: &Path) -> std::io::Error {
    use std::os::unix::process::CommandExt;
    std::process::Command::new(exe).args(restart_args(db_path)).exec()
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --lib update_state 2>&1 | tail -4`
Expected: `4 passed`.

- [ ] **Step 5: Add `update` to `AppState` and fix constructors**

In `src/server.rs` add to `pub struct AppState`:

```rust
    /// Version cache, update status and the background update job.
    pub update: Arc<crate::update_state::UpdateState>,
```

Update every constructor:

```bash
sed -i '' 's|change_tx: tokio::sync::broadcast::channel(16).0 }|change_tx: tokio::sync::broadcast::channel(16).0, update: Arc::new(crate::update_state::UpdateState::for_tests("http://127.0.0.1:1", false)) }|' src/server.rs
```

then fix the multi-line constructor near `AppState {` in the `tests` module (~line 1370) by hand, and in `serve()` add `update: Arc::new(crate::update_state::UpdateState::from_env()),` after `change_tx,`. Run `cargo test --lib 2>&1 | tail -3` — only the two known pre-existing failures (`auth::…token_public_api`, `watch::…broadcasts_on_db_file_write`) may fail.

- [ ] **Step 6: Write failing router tests** (append to the `tests` module in `src/server.rs`)

```rust
fn get_req(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).header("host", "127.0.0.1").body(Body::empty()).unwrap()
}

fn post_update_req(with_header: bool) -> Request<Body> {
    let mut b = Request::builder().method("POST").uri("/api/update").header("host", "127.0.0.1");
    if with_header {
        b = b.header("x-command-center", "update");
    }
    b.body(Body::empty()).unwrap()
}

#[tokio::test]
async fn test_version_reports_newer_release_but_unsupported_in_test_builds() {
    let (_dir, mut state) = make_state();
    let mut server = mockito::Server::new_async().await;
    server.mock("GET", "/releases/latest").with_status(200)
        .with_body(r#"{"tag_name":"v99.0.0","assets":[]}"#).create_async().await;
    state.update = Arc::new(crate::update_state::UpdateState::for_tests(&server.url(), false));
    let response = router(state).oneshot(get_req("/api/version")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    assert_eq!(json["current"], env!("CARGO_PKG_VERSION"));
    assert_eq!(json["latest"], "99.0.0");
    assert_eq!(json["update_available"], true);
    assert_eq!(json["update_supported"], false);
}

#[tokio::test]
async fn test_version_survives_rate_limited_release_api() {
    let (_dir, mut state) = make_state();
    let mut server = mockito::Server::new_async().await;
    server.mock("GET", "/releases/latest").with_status(403).create_async().await;
    state.update = Arc::new(crate::update_state::UpdateState::for_tests(&server.url(), true));
    let response = router(state).oneshot(get_req("/api/version")).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    assert!(json["latest"].is_null());
    assert_eq!(json["update_available"], false);
}

#[tokio::test]
async fn test_post_update_without_custom_header_is_403() {
    let (_dir, mut state) = make_state();
    state.update = Arc::new(crate::update_state::UpdateState::for_tests("http://127.0.0.1:1", true));
    let update = state.update.clone();
    let response = router(state).oneshot(post_update_req(false)).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(update.status().phase, crate::update::Phase::Idle);
}

#[tokio::test]
async fn test_post_update_unsupported_build_is_400() {
    let (_dir, state) = make_state(); // for_tests(.., supported = false)
    let response = router(state).oneshot(post_update_req(true)).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_post_update_while_running_is_409() {
    let (_dir, mut state) = make_state();
    state.update = Arc::new(crate::update_state::UpdateState::for_tests("http://127.0.0.1:1", true));
    assert!(state.update.try_begin()); // an update is "already running"
    let response = router(state).oneshot(post_update_req(true)).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_update_status_reports_phase_and_typed_error() {
    let (_dir, state) = make_state();
    state.update.fail(&crate::update::UpdateError::NoPlatformAsset("x".into()));
    let response = router(state).oneshot(get_req("/api/update/status")).await.unwrap();
    let json = body_json(response).await;
    assert_eq!(json["phase"], "failed");
    assert_eq!(json["error"]["kind"], "no_platform_asset");
}
```

- [ ] **Step 7: Run to verify failure**

Run: `cargo test --lib test_version test_post_update test_update_status 2>&1 | tail -6`
Expected: FAIL (404/405: routes missing). (If `cargo test` rejects multiple filters, run them one at a time.)

- [ ] **Step 8: Implement the handlers** (in `src/server.rs`, above `api_routes`)

```rust
#[derive(Serialize)]
struct VersionResponse {
    current: String,
    latest: Option<String>,
    update_available: bool,
    update_supported: bool,
}

async fn get_version(State(state): State<Arc<AppState>>) -> Json<VersionResponse> {
    let update = state.update.clone();
    let latest = tokio::task::spawn_blocking(move || update.latest_version()).await.ok().flatten();
    let current = crate::update::current_version();
    let update_available = latest
        .as_deref()
        .and_then(|l| semver::Version::parse(l).ok())
        .is_some_and(|l| crate::update::is_newer(&current, &l));
    Json(VersionResponse {
        current: current.to_string(),
        latest,
        update_available,
        update_supported: state.update.supported,
    })
}

async fn post_update(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    // A cross-site page cannot set this header without a CORS preflight, which we never allow.
    if headers.get("x-command-center").and_then(|v| v.to_str().ok()) != Some("update") {
        return Err(app_error(StatusCode::FORBIDDEN, "missing X-Command-Center header"));
    }
    if !state.update.supported {
        return Err(app_error(StatusCode::BAD_REQUEST, "updates are not supported for this build"));
    }
    if !state.update.try_begin() {
        return Err(app_error(StatusCode::CONFLICT, "an update is already running"));
    }
    let update = state.update.clone();
    let db_path = state.db_path.clone();
    std::thread::spawn(move || crate::update_state::run_update_job(&update, &db_path));
    Ok(StatusCode::ACCEPTED)
}

async fn get_update_status(State(state): State<Arc<AppState>>) -> Json<crate::update_state::StatusSnapshot> {
    Json(state.update.status())
}
```

and add to `api_routes()`:

```rust
        .route("/api/version", get(get_version))
        .route("/api/update", axum::routing::post(post_update))
        .route("/api/update/status", get(get_update_status))
```

- [ ] **Step 9: Run to verify pass**

Run each: `cargo test --lib test_version`, `cargo test --lib test_post_update`, `cargo test --lib test_update_status`
Expected: all pass. Then `cargo test --lib 2>&1 | tail -3` → only the two pre-existing failures.

- [ ] **Step 10: Commit**

```bash
git add src/update_state.rs src/lib.rs src/server.rs
git commit -m "feat: version and in-app update endpoints with automatic restart"
```

---

### Task 9: Web UI version badge and Update button

**Files:**
- Modify: `web/src/types.ts`, `web/src/api.ts`, `web/src/App.tsx`
- Create: `web/src/components/VersionBadge.tsx`

**Interfaces:**
- Consumes: `GET /api/version`, `POST /api/update` (header `X-Command-Center: update`), `GET /api/update/status` (Task 8).
- Produces: `<VersionBadge />` (no props).

The web app has no test runner; the verification is `npm run build` (runs `tsc -b`) plus the manual check in Step 5.

- [ ] **Step 1: Types** — append to `web/src/types.ts`

```ts
export interface VersionInfo {
  current: string;
  latest: string | null;
  update_available: boolean;
  update_supported: boolean;
}

export interface UpdateStatus {
  phase: 'idle' | 'downloading' | 'verifying' | 'installing' | 'restarting' | 'failed';
  error: { kind: string; message: string } | null;
  skipped_files: string[];
}
```

- [ ] **Step 2: API helpers** — append to `web/src/api.ts` (add the two types to the existing `import type … from './types'`)

```ts
export async function fetchVersion(): Promise<VersionInfo> {
  return jsonOrThrow(await fetch('/api/version'));
}

export async function startUpdate(): Promise<void> {
  const response = await fetch('/api/update', {
    method: 'POST',
    headers: { 'X-Command-Center': 'update' },
  });
  if (!response.ok) {
    const body = await response.json().catch(() => ({}));
    throw new Error(body.error ?? `Request failed: ${response.status}`);
  }
}

export async function fetchUpdateStatus(): Promise<UpdateStatus> {
  return jsonOrThrow(await fetch('/api/update/status'));
}
```

- [ ] **Step 3: Component** — create `web/src/components/VersionBadge.tsx`

```tsx
import { useEffect, useState } from 'react';
import type { UpdateStatus, VersionInfo } from '../types';
import { fetchUpdateStatus, fetchVersion, startUpdate } from '../api';
import { Button } from './Button';

const PHASE_TEXT: Record<UpdateStatus['phase'], string> = {
  idle: 'Starting…',
  downloading: 'Downloading…',
  verifying: 'Verifying…',
  installing: 'Installing…',
  restarting: 'Restarting…',
  failed: 'Failed',
};

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

export function VersionBadge() {
  const [info, setInfo] = useState<VersionInfo | null>(null);
  const [busy, setBusy] = useState(false);
  const [text, setText] = useState('');
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchVersion().then(setInfo).catch(() => {});
  }, []);

  async function runUpdate() {
    if (!info?.latest) return;
    const target = info.latest;
    setError(null);
    setBusy(true);
    setText(PHASE_TEXT.idle);

    try {
      await startUpdate();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setBusy(false);
      return;
    }

    // Follow the phases until the server says it is restarting (or the connection drops
    // because it already re-exec'd).
    for (;;) {
      await sleep(700);
      let status: UpdateStatus;
      try {
        status = await fetchUpdateStatus();
      } catch {
        break;
      }
      if (status.phase === 'failed') {
        setError(status.error?.message ?? 'Update failed');
        setBusy(false);
        return;
      }
      if (status.phase === 'idle') {
        setBusy(false); // already up to date
        return;
      }
      setText(PHASE_TEXT[status.phase]);
      if (status.phase === 'restarting') break;
    }

    setText(PHASE_TEXT.restarting);
    const deadline = Date.now() + 60_000;
    while (Date.now() < deadline) {
      await sleep(1000);
      try {
        const v = await fetchVersion();
        if (v.current === target) {
          window.location.reload();
          return;
        }
      } catch {
        // server is mid-restart
      }
    }
    setError('The server did not come back. Restart `task serve` and reload this page.');
    setBusy(false);
  }

  if (!info) return null;

  return (
    <div style={{ marginLeft: 'auto', display: 'flex', alignItems: 'center', gap: 10, fontSize: 12, color: 'var(--fg-4)' }}>
      <span>v{info.current}</span>
      {info.update_available && info.latest && (
        <>
          <span style={{ color: 'var(--fg-2)' }}>v{info.latest} available</span>
          {info.update_supported && (
            <Button size="sm" onClick={runUpdate} disabled={busy}>
              {busy ? text : 'Update'}
            </Button>
          )}
        </>
      )}
      {error && <span style={{ color: 'var(--danger)' }}>{error}</span>}
    </div>
  );
}
```

- [ ] **Step 3b: Mount it** — in `web/src/App.tsx` add `import { VersionBadge } from './components/VersionBadge';` and render `<VersionBadge />` as the **last child of the `<header …>` element** at ~line 716 (the header already has `display: 'flex'`, `alignItems: 'center'`).

- [ ] **Step 4: Build**

Run: `cd web && npm run build 2>&1 | tail -6`
Expected: `✓ built in …` with no TypeScript errors.

- [ ] **Step 5: Manual check**

Run `cargo run --bin task_server` (debug: update unsupported) and open the UI: the header shows `v4.0.0` and no Update button. With `TASK_RELEASES_API` pointing at a local stub returning `v99.0.0`, it shows "v99.0.0 available" but still no button (unsupported). The button path is verified end to end in the pre-release dry run (Task 11).

- [ ] **Step 6: Commit**

```bash
git add web/src
git commit -m "feat(web): show version and an Update button"
```

---

### Task 10: `install.sh` and the release workflow

**Files:**
- Create: `install.sh`, `tests/install_sh_test.sh`, `.github/workflows/release.yml`

**Interfaces:**
- Consumes: archive naming `task-vX.Y.Z-<target>.tar.gz` and `SHA256SUMS` from the spec.
- Produces: `install.sh` env hooks `INSTALL_DIR`, `RELEASE_TAG`, `RELEASE_BASE_URL`, `RELEASES_REPO`, and `INSTALL_SH_LIB=1` (source without running).

- [ ] **Step 1: Write the failing shell test** — `tests/install_sh_test.sh`

```sh
#!/bin/sh
# Tests for install.sh. Run: sh tests/install_sh_test.sh
set -eu

HERE=$(cd "$(dirname "$0")/.." && pwd)
INSTALL_SH_LIB=1
export INSTALL_SH_LIB
. "$HERE/install.sh"

failures=0
check() { # name expected actual
    if [ "$2" = "$3" ]; then printf 'ok   %s\n' "$1"; else printf 'FAIL %s: expected [%s] got [%s]\n' "$1" "$2" "$3"; failures=$((failures + 1)); fi
}

# --- detect_target, with uname faked ---
uname() { if [ "$1" = "-s" ]; then echo "$FAKE_OS"; else echo "$FAKE_ARCH"; fi; }
check "darwin arm64" aarch64-apple-darwin "$(FAKE_OS=Darwin FAKE_ARCH=arm64 detect_target)"
check "darwin x64" x86_64-apple-darwin "$(FAKE_OS=Darwin FAKE_ARCH=x86_64 detect_target)"
check "linux x64" x86_64-unknown-linux-gnu "$(FAKE_OS=Linux FAKE_ARCH=x86_64 detect_target)"
check "linux arm64" aarch64-unknown-linux-gnu "$(FAKE_OS=Linux FAKE_ARCH=aarch64 detect_target)"
unsupported=$( (FAKE_OS=Windows_NT FAKE_ARCH=x86_64 detect_target) 2>&1 || true)
case "$unsupported" in *unsupported*) check "unsupported platform message" ok ok ;; *) check "unsupported platform message" ok "$unsupported" ;; esac
unset -f uname

# --- verify ---
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
printf 'hello' > "$tmp/a.tar.gz"
good=$(sha256_of "$tmp/a.tar.gz")
printf '%s  a.tar.gz\n' "$good" > "$tmp/SHA256SUMS"
( cd "$tmp" && verify a.tar.gz SHA256SUMS ) && check "verify accepts match" ok ok
printf '%s  a.tar.gz\n' "0000000000000000000000000000000000000000000000000000000000000000" > "$tmp/SHA256SUMS"
out=$( (cd "$tmp" && verify a.tar.gz SHA256SUMS) 2>&1 || true)
case "$out" in *mismatch*) check "verify rejects mismatch" ok ok ;; *) check "verify rejects mismatch" ok "$out" ;; esac
printf '%s  other.tar.gz\n' "$good" > "$tmp/SHA256SUMS"
out=$( (cd "$tmp" && verify a.tar.gz SHA256SUMS) 2>&1 || true)
case "$out" in *"no checksum"*) check "verify rejects missing entry" ok ok ;; *) check "verify rejects missing entry" ok "$out" ;; esac

[ "$failures" -eq 0 ] || { echo "$failures failure(s)"; exit 1; }
echo "all passed"
```

- [ ] **Step 2: Run to verify failure**

Run: `sh tests/install_sh_test.sh 2>&1 | tail -3`
Expected: FAIL (`install.sh: No such file`).

- [ ] **Step 3: Write `install.sh`**

```sh
#!/bin/sh
# Install command-center (`task`, `task-tui`) from the public releases repo.
#   curl -fsSL https://raw.githubusercontent.com/TheSophist1976/command-center-releases/main/install.sh | sh
# Env: INSTALL_DIR (default ~/.local/bin). Test hooks: RELEASE_TAG, RELEASE_BASE_URL.
set -eu

REPO="${RELEASES_REPO:-TheSophist1976/command-center-releases}"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

err() { printf 'error: %s\n' "$1" >&2; exit 1; }
info() { printf '▸ %s\n' "$1"; }

detect_target() {
    os=$(uname -s)
    arch=$(uname -m)
    case "$os/$arch" in
        Darwin/arm64) echo aarch64-apple-darwin ;;
        Darwin/x86_64) echo x86_64-apple-darwin ;;
        Linux/x86_64) echo x86_64-unknown-linux-gnu ;;
        Linux/aarch64 | Linux/arm64) echo aarch64-unknown-linux-gnu ;;
        *) err "unsupported platform: $os $arch (supported: macOS arm64/x64, Linux x64/arm64)" ;;
    esac
}

latest_tag() {
    curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
        | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n 1
}

sha256_of() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "$1" | awk '{print $1}'
    else
        shasum -a 256 "$1" | awk '{print $1}'
    fi
}

# verify <archive-file-in-cwd> <sums-file>
verify() {
    expected=$(awk -v f="$1" '$2 == f || $2 == "*" f { print $1 }' "$2")
    [ -n "$expected" ] || err "no checksum for $1 in SHA256SUMS"
    actual=$(sha256_of "$1")
    [ "$expected" = "$actual" ] || err "checksum mismatch for $1 — aborting"
}

profile_file() {
    case "${SHELL:-}" in
        */zsh) echo "$HOME/.zshrc" ;;
        */bash) if [ "$(uname -s)" = Darwin ]; then echo "$HOME/.bash_profile"; else echo "$HOME/.bashrc"; fi ;;
        *) echo "$HOME/.profile" ;;
    esac
}

offer_path() {
    case ":$PATH:" in *":$INSTALL_DIR:"*) return 0 ;; esac
    profile=$(profile_file)
    line="export PATH=\"$INSTALL_DIR:\$PATH\""
    if [ -f "$profile" ] && grep -qF "$INSTALL_DIR" "$profile"; then
        info "$INSTALL_DIR is already referenced in $profile (restart your shell)"
        return 0
    fi
    if [ -r /dev/tty ]; then
        printf '▸ Add %s to your PATH in %s? [Y/n] ' "$INSTALL_DIR" "$profile"
        read -r answer < /dev/tty || answer=""
        case "$answer" in [Nn]*) info "Skipped. Add it yourself: $line"; return 0 ;; esac
        printf '\n# added by command-center installer\n%s\n' "$line" >> "$profile"
        info "Added to $profile (restart your shell)"
    else
        info "Add $INSTALL_DIR to your PATH: $line"
    fi
}

main() {
    target=$(detect_target)
    tag="${RELEASE_TAG:-$(latest_tag)}"
    [ -n "$tag" ] || err "could not determine the latest release of $REPO"
    base="${RELEASE_BASE_URL:-https://github.com/$REPO/releases/download/$tag}"
    archive="task-$tag-$target.tar.gz"

    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT
    info "Downloading $archive"
    curl -fsSL "$base/$archive" -o "$tmp/$archive" || err "download failed: $base/$archive"
    curl -fsSL "$base/SHA256SUMS" -o "$tmp/SHA256SUMS" || err "download failed: $base/SHA256SUMS"
    (cd "$tmp" && verify "$archive" SHA256SUMS)

    tar -xzf "$tmp/$archive" -C "$tmp"
    mkdir -p "$INSTALL_DIR"
    for bin in task task-tui; do
        [ -f "$tmp/$bin" ] && install -m 755 "$tmp/$bin" "$INSTALL_DIR/$bin"
    done
    info "Installed to $INSTALL_DIR"
    offer_path
    echo
    echo "Next: run \`task setup\`"
}

[ "${INSTALL_SH_LIB:-}" = 1 ] || main "$@"
```

- [ ] **Step 4: Run to verify pass**

Run: `sh tests/install_sh_test.sh 2>&1 | tail -10`
Expected: every line `ok …` and `all passed`.
Run: `shellcheck install.sh tests/install_sh_test.sh` (install shellcheck with `brew install shellcheck` if absent).
Expected: no findings. Fix any that appear (the test's unused `t()` helper should be deleted if flagged).

- [ ] **Step 5: Write `.github/workflows/release.yml`**

```yaml
name: release

on:
  push:
    tags: ['v*']

permissions:
  contents: read

env:
  RELEASES_REPO: TheSophist1976/command-center-releases

jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Tag must equal Cargo.toml version
        run: |
          tag="${GITHUB_REF_NAME#v}"
          cargo_version=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
          [ "$tag" = "$cargo_version" ] || { echo "tag $tag != Cargo.toml $cargo_version"; exit 1; }
      - uses: actions/setup-node@v4
        with: { node-version: 20, cache: npm, cache-dependency-path: web/package-lock.json }
      - run: npm ci && npm run build
        working-directory: web
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test
      - run: shellcheck install.sh tests/install_sh_test.sh
      - run: sh tests/install_sh_test.sh

  build:
    needs: verify
    strategy:
      fail-fast: true
      matrix:
        include:
          # Verify these runner labels still exist when you cut the first release.
          - { target: aarch64-apple-darwin, os: macos-14 }
          - { target: x86_64-apple-darwin, os: macos-13 }
          - { target: x86_64-unknown-linux-gnu, os: ubuntu-22.04 }
          - { target: aarch64-unknown-linux-gnu, os: ubuntu-22.04-arm }
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 20, cache: npm, cache-dependency-path: web/package-lock.json }
      - run: npm ci && npm run build
        working-directory: web
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --release --features tui
      - name: Package
        run: |
          name="task-${GITHUB_REF_NAME}-${{ matrix.target }}"
          mkdir stage
          cp target/release/task target/release/task-tui stage/
          tar -czf "$name.tar.gz" -C stage task task-tui
      - uses: actions/upload-artifact@v4
        with:
          name: archive-${{ matrix.target }}
          path: task-*.tar.gz

  checksums:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
        with: { pattern: 'archive-*', path: dist, merge-multiple: true }
      - name: SHA256SUMS
        run: cd dist && sha256sum task-*.tar.gz > SHA256SUMS && cat SHA256SUMS
      - uses: actions/upload-artifact@v4
        with: { name: release-files, path: dist/* }

  smoke:
    needs: checksums
    strategy:
      matrix:
        os: [macos-14, ubuntu-22.04]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: actions/download-artifact@v4
        with: { name: release-files, path: dist }
      - name: Install the just-built artifact on a clean runner
        run: |
          INSTALL_DIR="$RUNNER_TEMP/bin" \
          RELEASE_TAG="$GITHUB_REF_NAME" \
          RELEASE_BASE_URL="file://$PWD/dist" \
          sh install.sh
          "$RUNNER_TEMP/bin/task" --version | tee /dev/stderr | grep -F "${GITHUB_REF_NAME#v}"

  publish:
    needs: [checksums, smoke]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/download-artifact@v4
        with: { name: release-files, path: dist }
      - name: Publish install.sh to the releases repo
        env:
          TOKEN: ${{ secrets.RELEASES_REPO_TOKEN }}
        run: |
          git clone "https://x-access-token:${TOKEN}@github.com/${RELEASES_REPO}.git" releases
          cp install.sh releases/install.sh
          cd releases
          git config user.name "release-bot"
          git config user.email "release-bot@users.noreply.github.com"
          if ! git diff --quiet || [ -n "$(git status --porcelain)" ]; then
            git add install.sh && git commit -m "install.sh for ${GITHUB_REF_NAME}" && git push
          fi
      - name: Create the release
        env:
          GH_TOKEN: ${{ secrets.RELEASES_REPO_TOKEN }}
        run: |
          flag=""
          case "$GITHUB_REF_NAME" in *-*) flag="--prerelease" ;; esac
          gh release create "$GITHUB_REF_NAME" dist/* --repo "$RELEASES_REPO" --title "$GITHUB_REF_NAME" --generate-notes $flag
```

- [ ] **Step 6: Validate the workflow YAML**

Run: `python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/release.yml')); print('yaml ok')"`
Expected: `yaml ok`. (If `actionlint` is installed, also run `actionlint .github/workflows/release.yml`.)

- [ ] **Step 7: Commit**

```bash
git add install.sh tests/install_sh_test.sh .github/workflows/release.yml
git commit -m "feat: install.sh and tag-triggered release workflow"
```

---

### Task 11: Documentation and the first-release checklist

**Files:**
- Modify: `README.md`, `AGENTS.md`

- [ ] **Step 1: Update `README.md`**

Replace the build-from-source-first instructions with, at the top of the install section:

````markdown
## Install

```sh
curl -fsSL https://raw.githubusercontent.com/TheSophist1976/command-center-releases/main/install.sh | sh
task setup     # choose your task/notes directories; installs AGENTS.md and Claude skills
task serve     # web UI at http://127.0.0.1:4287
```

## Update

```sh
task update            # or: task update --check
```

The web UI also shows the running version and an **Update** button when a newer release exists. It restarts the server automatically (about a second of downtime).

## Develop from source

(keep the existing from-source / `deploy.sh` / `start-web.sh` instructions here)
````

- [ ] **Step 2: Update `AGENTS.md`**

Add one line near the top-level description of how agents' instructions get installed: `This file is installed into your task directory by \`task setup\` and refreshed by \`task update\`; edit the copy in the source repo, not the installed one.` (No `tasks.md` format change, so no other section changes.)

- [ ] **Step 3: Full verification**

Run: `cargo test 2>&1 | grep -E "^test result|FAILED"` — expect only the two pre-existing sandbox-related failures (`auth::…token_public_api`, `watch::…broadcasts_on_db_file_write`) if run in the sandbox; none outside it.
Run: `cd web && npm run build 2>&1 | tail -2` — `✓ built`.
Run: `cargo build --release --features tui 2>&1 | tail -2 && target/release/task --version` — prints the Cargo version.
Run: `sh tests/install_sh_test.sh | tail -1` — `all passed`.

- [ ] **Step 4: Commit**

```bash
git add README.md AGENTS.md
git commit -m "docs: document install, update and the release flow"
```

- [ ] **Step 5: First-release checklist — owner actions, do NOT perform without explicit approval**

These create external resources or publish; an agent must stop and ask before each.

1. Create the **public** repo `TheSophist1976/command-center-releases` (confirm the name; it is set in `src/update.rs` `DEFAULT_RELEASES_API`, `install.sh` `REPO`, and `release.yml` `RELEASES_REPO` — change all three together if it differs). Add an initial commit so `git clone` works.
2. Create a fine-grained token scoped to **contents: write on that repo only**; add it to the private repo as secret `RELEASES_REPO_TOKEN`.
3. Dry run with a pre-release: set `version = "4.1.0-rc.1"` in `Cargo.toml` (run `cargo build` to update `Cargo.lock`), commit, then `git tag v4.1.0-rc.1 && git push origin v4.1.0-rc.1`. Watch the run: all four builds, both smoke jobs, and the publish job must pass; the release must appear as a pre-release with 4 archives + `SHA256SUMS`, and `install.sh` must be on the releases repo's `main`.
4. On a clean machine/container: `curl … | sh`, `task setup`, `task serve`. Then cut `v4.1.0-rc.2` and verify both `task update` and the web **Update** button (note: `releases/latest` ignores pre-releases, so for this check temporarily point `TASK_RELEASES_API` at a stub or publish rc.2 as a normal release).
5. Set the final `version = "4.1.0"`, tag `v4.1.0`, and push.
6. Decide the open item: whether `task update` prints a once-a-day "update available" notice (spec default: off — nothing in this plan implements it).

---

## Self-Review

**Spec coverage**
- §1 pipeline: tag/version check, verify, 4 native targets, web-then-cargo build, rustls, artifacts + `SHA256SUMS`, publish with scoped token → Tasks 1, 10, 11.
- §2 layout: subcommands, embedded dist/AGENTS.md/skills, debug reads disk, `task_server` alias → Tasks 2, 3, 5.
- §3 `install.sh`: all six steps incl. idempotent profile edit, no config logic → Task 10.
- §4 `task setup`: three keys, never overwrite without asking, AGENTS.md + skills from embedded copies, config path, `deploy.sh` stays → Tasks 4, 5.
- §5 `task serve` → Task 3.
- §6 updater: latest/semver/download/verify/atomic replace/refresh files/typed errors/CLI incl. `--check` and "restart `task serve`" notice → Tasks 6, 7 (+ clarification 1 for the refresh mechanism).
- §7 endpoints, `update_supported`, CSRF header, async 202 + status, re-exec, UI polling/reload/timeout/inline errors → Tasks 8, 9.
- §8 testing: updater cases (up-to-date, newer, checksum mismatch, missing asset, no network, non-writable dir, atomic replace) → Task 7; version cache → Task 8; 403/`update_supported=false` → Task 8; setup idempotency → Task 5; shellcheck + clean-runner install + pre-release dry run → Tasks 10, 11.
- §9 docs → Task 11. Open items → Task 11 Step 5.

**Placeholders:** none; Task 5 Step 6 and Task 3's "match the existing status" note are explicit verify-and-adjust instructions with the expected outcome, not deferred work.

**Type consistency:** `UpdateError`/`Phase` defined in Task 6 and used unchanged in Tasks 7–8; `EditedPolicy`/`install_managed_files`/`refresh` from Task 4 used in Tasks 5 and 8; `AppState.update: Arc<UpdateState>` introduced in Task 8 and referenced only after; `perform_update(api_base, install_dir, on_phase)` signature identical in Tasks 7 and 8.
