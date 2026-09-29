use task::server::{router_with_static, AppState};

#[tokio::main]
async fn main() {
    // Usage: task_server [--profile NAME] [DB_PATH]
    let mut file_arg: Option<String> = None;
    let mut profile_arg: Option<String> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--profile" {
            profile_arg = args.next();
        } else if let Some(name) = arg.strip_prefix("--profile=") {
            profile_arg = Some(name.to_string());
        } else {
            file_arg = Some(arg);
        }
    }
    let ws = task::workspace::resolve(file_arg.as_deref(), profile_arg.as_deref()).unwrap_or_else(|e| {
        eprintln!("task_server: {}", e);
        std::process::exit(1);
    });
    match task::db::prepare(&ws) {
        Ok(Some(msg)) => println!("task_server: {}", msg),
        Ok(None) => {}
        Err(e) => {
            eprintln!("task_server: {}", e);
            std::process::exit(1);
        }
    }
    let db_path = ws.db_path.clone();
    task::db::backup_daily(&db_path);
    let port: u16 = std::env::var("TASK_SERVER_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4287);
    let static_dir = std::path::PathBuf::from("web/dist");
    let static_dir = if static_dir.exists() { Some(static_dir) } else { None };

    if let Some(profile) = &ws.profile {
        println!("task_server: profile '{}' (notes: {})", profile, ws.notes_dir().display());
    }
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
        AppState { db_path, task_dir: ws.task_dir, write_lock: std::sync::Arc::new(tokio::sync::Mutex::new(())), change_tx },
        static_dir,
    );
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
