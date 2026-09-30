use task::server::{router_with_static, AppState};

#[tokio::main]
async fn main() {
    let db_path = task::db::resolve_file_path(std::env::args().nth(1).as_deref());
    task::db::backup_daily(&db_path);
    let notes_dir = task::db::resolve_notes_dir(&db_path);
    let port: u16 = std::env::var("TASK_SERVER_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4287);
    let static_dir = std::path::PathBuf::from("web/dist");
    let static_dir = if static_dir.exists() { Some(static_dir) } else { None };

    println!("task_server: serving {} on http://127.0.0.1:{}", db_path.display(), port);
    if static_dir.is_none() {
        println!("task_server: web/dist not found — API only, no static frontend (run `npm run build` in web/ first)");
    }

    let (change_tx, _) = tokio::sync::broadcast::channel(16);
    // Held for the lifetime of the process — dropping it would stop the watch.
    let _watcher = match task::watch::spawn(db_path.clone(), change_tx.clone()) {
        Ok(w) => Some(w),
        Err(e) => {
            eprintln!("task_server: failed to watch {} for changes — live updates disabled: {}", db_path.display(), e);
            None
        }
    };

    let app = router_with_static(
        AppState { notes_dir, db_path, write_lock: std::sync::Arc::new(tokio::sync::Mutex::new(())), change_tx },
        static_dir,
    );
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
