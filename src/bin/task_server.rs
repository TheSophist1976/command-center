use task::server::{router_with_static, AppState};

#[tokio::main]
async fn main() {
    let db_path = task::db::resolve_file_path(std::env::args().nth(1).as_deref());
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

    let app = router_with_static(
        AppState { db_path, write_lock: std::sync::Arc::new(tokio::sync::Mutex::new(())) },
        static_dir,
    );
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
