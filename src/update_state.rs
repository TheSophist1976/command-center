use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::update::{self, Phase, UpdateError};

const LATEST_CACHE_TTL: Duration = Duration::from_secs(300);
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

    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
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
        self.status.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Claims the single update slot. False if an update is already in flight.
    pub fn try_begin(&self) -> bool {
        let mut s = self.status.lock().unwrap_or_else(|e| e.into_inner());
        if matches!(s.phase, Phase::Downloading | Phase::Verifying | Phase::Installing | Phase::Restarting) {
            return false;
        }
        *s = StatusSnapshot { phase: Phase::Downloading, error: None, skipped_files: Vec::new() };
        true
    }

    pub fn set_phase(&self, phase: Phase) {
        self.status.lock().unwrap_or_else(|e| e.into_inner()).phase = phase;
    }

    pub fn set_skipped(&self, files: Vec<String>) {
        self.status.lock().unwrap_or_else(|e| e.into_inner()).skipped_files = files;
    }

    pub fn fail(&self, error: &UpdateError) {
        let mut s = self.status.lock().unwrap_or_else(|e| e.into_inner());
        s.phase = Phase::Failed;
        s.error = Some(ErrorInfo { kind: error.kind().to_string(), message: error.to_string() });
    }
}

/// Ends the update job in a terminal state even if the job thread panics.
struct JobGuard<'a>(&'a UpdateState);

impl Drop for JobGuard<'_> {
    fn drop(&mut self) {
        // Restarting is excluded: after it we exec and never return.
        let in_progress = matches!(self.0.status().phase, Phase::Downloading | Phase::Verifying | Phase::Installing);
        if in_progress {
            self.0.fail(&UpdateError::Install("update job ended unexpectedly".to_string()));
        }
    }
}

/// Runs the whole update on a background thread: install, refresh managed files with the
/// new binary, then re-exec it as `task serve`.
pub fn run_update_job(update: &UpdateState, db_path: &Path) {
    let _guard = JobGuard(update);
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
    fn job_guard_fails_a_job_left_in_progress() {
        let s = UpdateState::for_tests("http://127.0.0.1:1", true);
        assert!(s.try_begin());
        drop(JobGuard(&s));
        let snap = s.status();
        assert_eq!(snap.phase, crate::update::Phase::Failed);
        assert_eq!(snap.error.unwrap().kind, "install");
        assert!(s.try_begin());
    }

    #[test]
    fn job_guard_leaves_terminal_phases_alone() {
        let s = UpdateState::for_tests("http://127.0.0.1:1", true);
        assert!(s.try_begin());
        s.set_phase(crate::update::Phase::Idle);
        drop(JobGuard(&s));
        assert_eq!(s.status().phase, crate::update::Phase::Idle);
        assert!(s.status().error.is_none());

        assert!(s.try_begin());
        s.fail(&crate::update::UpdateError::ChecksumMismatch("x".into()));
        drop(JobGuard(&s));
        assert_eq!(s.status().error.unwrap().kind, "checksum_mismatch");
    }

    #[test]
    fn job_guard_recovers_from_a_panicking_job_thread() {
        let s = std::sync::Arc::new(UpdateState::for_tests("http://127.0.0.1:1", true));
        assert!(s.try_begin());
        let s2 = s.clone();
        let joined = std::thread::spawn(move || {
            let _guard = JobGuard(&s2);
            s2.set_phase(crate::update::Phase::Installing);
            panic!("boom");
        })
        .join();
        assert!(joined.is_err());
        assert_eq!(s.status().phase, crate::update::Phase::Failed);
        assert!(s.try_begin());
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
