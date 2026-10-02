#[tokio::main]
async fn main() {
    let db_path = task::db::resolve_file_path(std::env::args().nth(1).as_deref());
    if let Err(e) = task::server::serve(db_path).await {
        eprintln!("task_server: {}", e);
        std::process::exit(1);
    }
}
