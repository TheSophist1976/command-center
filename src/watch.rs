//! Watches `tasks.db` (and its WAL/SHM sidecar files) for changes made by
//! any process — the CLI, another `task_server` handler, an external
//! script — and broadcasts a signal so connected web clients can refresh
//! over SSE. See `server::AppState::change_tx` and `server::task_events`.

use std::path::{Path, PathBuf};
use std::sync::mpsc as std_mpsc;
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

/// How long to wait after a filesystem event before broadcasting, so a
/// burst of writes (SQLite WAL commits touch the file multiple times per
/// `db::save`) collapses into a single signal instead of a flood.
const DEBOUNCE: Duration = Duration::from_millis(300);

/// Spawns a background thread that watches `db_path`'s parent directory
/// and broadcasts on `tx` whenever `db_path` or its `-wal`/`-shm` sidecar
/// files change. Watching the directory (rather than the file directly)
/// is necessary because SQLite in WAL mode writes to the sidecar files,
/// not the main file, on most commits.
///
/// Returns the `notify::RecommendedWatcher` — the caller must keep it
/// alive for as long as watching should continue; dropping it stops the
/// watch.
pub fn spawn(db_path: PathBuf, tx: tokio::sync::broadcast::Sender<()>) -> notify::Result<RecommendedWatcher> {
    let watch_dir = db_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let relevant_names = relevant_file_names(&db_path);

    let (raw_tx, raw_rx) = std_mpsc::channel::<notify::Result<notify::Event>>();
    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = raw_tx.send(res);
    })?;
    watcher.watch(&watch_dir, RecursiveMode::NonRecursive)?;

    std::thread::spawn(move || {
        loop {
            // Block for the first event in this batch.
            let first = match raw_rx.recv() {
                Ok(event) => event,
                Err(_) => return, // watcher dropped, stop the thread
            };
            if !event_touches(&first, &relevant_names) {
                continue;
            }
            // Drain and debounce: keep consuming events for DEBOUNCE after
            // the last relevant one, so a burst collapses into one signal.
            loop {
                match raw_rx.recv_timeout(DEBOUNCE) {
                    Ok(event) => {
                        if event_touches(&event, &relevant_names) {
                            continue;
                        }
                    }
                    Err(std_mpsc::RecvTimeoutError::Timeout) => break,
                    Err(std_mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
            // No receivers is not an error — it just means no browser tab
            // is currently subscribed to /api/events.
            let _ = tx.send(());
        }
    });

    Ok(watcher)
}

fn relevant_file_names(db_path: &Path) -> Vec<String> {
    let Some(name) = db_path.file_name().and_then(|n| n.to_str()) else {
        return Vec::new();
    };
    vec![name.to_string(), format!("{name}-wal"), format!("{name}-shm")]
}

fn event_touches(result: &notify::Result<notify::Event>, relevant_names: &[String]) -> bool {
    let Ok(event) = result else { return false };
    event.paths.iter().any(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| relevant_names.iter().any(|rn| rn == n))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_watch_broadcasts_on_db_file_write() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tasks.db");
        fs::write(&db_path, "initial").unwrap();

        let (tx, mut rx) = tokio::sync::broadcast::channel(16);
        let _watcher = spawn(db_path.clone(), tx).unwrap();

        // Give the watcher a moment to start before writing.
        tokio::time::sleep(Duration::from_millis(100)).await;
        fs::write(&db_path, "changed").unwrap();

        let result = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await;
        assert!(result.is_ok(), "expected a broadcast within 2s of writing the watched file");
    }

    #[tokio::test]
    async fn test_watch_ignores_unrelated_file_in_same_directory() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tasks.db");
        fs::write(&db_path, "initial").unwrap();

        let (tx, mut rx) = tokio::sync::broadcast::channel(16);
        let _watcher = spawn(db_path.clone(), tx).unwrap();

        tokio::time::sleep(Duration::from_millis(100)).await;
        fs::write(dir.path().join("unrelated.txt"), "noise").unwrap();

        // No broadcast should arrive for the unrelated file within the
        // debounce window.
        let result = tokio::time::timeout(Duration::from_millis(600), rx.recv()).await;
        assert!(result.is_err(), "did not expect a broadcast for an unrelated file");
    }

    #[test]
    fn test_relevant_file_names_includes_wal_and_shm() {
        let names = relevant_file_names(Path::new("/some/dir/tasks.db"));
        assert_eq!(names, vec!["tasks.db", "tasks.db-wal", "tasks.db-shm"]);
    }
}
