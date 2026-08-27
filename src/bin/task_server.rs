use task::server::{router, AppState};

#[tokio::main]
async fn main() {
    let db_path = task::db::resolve_file_path(std::env::args().nth(1).as_deref());
    let port: u16 = std::env::var("TASK_SERVER_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4287);

    println!("task_server: serving {} on http://127.0.0.1:{}", db_path.display(), port);

    let app = router(AppState { db_path });
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port)).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
