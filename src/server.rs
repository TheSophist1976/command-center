use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::get;
use axum::Router;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::commands::filter_and_sort_tasks;
use crate::db;
use crate::task::{Status, Task};

#[derive(Clone)]
pub struct AppState {
    pub db_path: PathBuf,
    pub write_lock: Arc<tokio::sync::Mutex<()>>,
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub status: Option<String>,
    pub agent: Option<String>,
    pub project: Option<String>,
    pub tag: Option<String>,
    pub due_before: Option<String>,
}

fn app_error(status: StatusCode, message: impl Into<String>) -> (StatusCode, Json<serde_json::Value>) {
    let message = message.into();
    if status == StatusCode::INTERNAL_SERVER_ERROR {
        eprintln!("task_server error: {}", message);
        return (status, Json(serde_json::json!({ "error": "internal server error" })));
    }
    (status, Json(serde_json::json!({ "error": message })))
}

fn host_is_allowed(host_header: &str) -> bool {
    let hostname = host_header.split(':').next().unwrap_or("");
    hostname == "127.0.0.1" || hostname == "localhost"
}

async fn validate_host(request: axum::extract::Request, next: Next) -> Response {
    let allowed = request
        .headers()
        .get(axum::http::header::HOST)
        .and_then(|v| v.to_str().ok())
        .is_some_and(host_is_allowed);

    if !allowed {
        return (StatusCode::FORBIDDEN, "Forbidden").into_response();
    }
    next.run(request).await
}

async fn list_tasks(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<Task>>, (StatusCode, Json<serde_json::Value>)> {
    let task_file = db::load(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let status = query
        .status
        .as_deref()
        .map(Status::from_str)
        .transpose()
        .map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?;
    let due_before = query
        .due_before
        .as_deref()
        .map(|s| {
            chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map_err(|_| format!("Invalid date: '{}'. Expected YYYY-MM-DD.", s))
        })
        .transpose()
        .map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?;

    let tasks: Vec<Task> = filter_and_sort_tasks(
        &task_file.tasks,
        status,
        query.agent.as_deref(),
        query.project.as_deref(),
        query.tag.as_deref(),
        due_before,
    )
    .into_iter()
    .cloned()
    .collect();

    Ok(Json(tasks))
}

async fn get_task(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<Task>, (StatusCode, Json<serde_json::Value>)> {
    let task_file = db::load(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    task_file
        .find_task(id)
        .cloned()
        .map(Json)
        .ok_or_else(|| app_error(StatusCode::NOT_FOUND, format!("Task {} not found", id)))
}

#[derive(Deserialize)]
pub struct AddTaskRequest {
    pub title: String,
    pub priority: Option<String>,
    pub due: Option<String>,
    pub project: Option<String>,
    pub tags: Option<String>,
    pub agent: Option<String>,
    pub description: Option<String>,
}

#[derive(Serialize)]
pub struct AgentProfile {
    pub name: String,
    pub dir: String,
}

fn parse_csv(s: Option<&str>) -> Vec<String> {
    s.unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

async fn add_task(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddTaskRequest>,
) -> Result<Json<Task>, (StatusCode, Json<serde_json::Value>)> {
    let priority_str = req.priority.as_deref().unwrap_or("medium");
    let priority = crate::task::Priority::from_str(priority_str)
        .map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?;
    let today = chrono::Local::now().date_naive();
    let due_date = req
        .due
        .as_deref()
        .map(|d| {
            crate::parser::parse_due_date_input(d, today)
                .ok_or_else(|| format!("Invalid due date: '{}'", d))
        })
        .transpose()
        .map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?;

    let _guard = state.write_lock.lock().await;
    let mut task_file = db::load(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let id = task_file.next_id;
    task_file.next_id += 1;
    let new_task = Task {
        id,
        title: req.title,
        status: Status::Open,
        priority,
        tags: parse_csv(req.tags.as_deref()),
        created: chrono::Utc::now(),
        updated: None,
        description: req.description,
        due_date,
        project: req.project,
        recurrence: None,
        notes: Vec::new(),
        agent: req.agent,
        effort: None,
        work_status: None,
    };
    task_file.tasks.push(new_task.clone());
    db::save(&state.db_path, &task_file).map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(new_task))
}

#[derive(Deserialize)]
pub struct EditTaskRequest {
    pub title: Option<String>,
    pub priority: Option<String>,
    pub due: Option<String>,
    pub project: Option<String>,
    pub tags: Option<String>,
    pub agent: Option<String>,
    pub description: Option<String>,
    pub effort: Option<String>,
    pub work_status: Option<String>,
}

async fn edit_task(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
    Json(req): Json<EditTaskRequest>,
) -> Result<Json<Task>, (StatusCode, Json<serde_json::Value>)> {
    let today = chrono::Local::now().date_naive();
    let priority = req
        .priority
        .as_deref()
        .map(crate::task::Priority::from_str)
        .transpose()
        .map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?;
    let effort = req
        .effort
        .as_deref()
        .map(crate::task::Effort::from_str)
        .transpose()
        .map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?;
    // work_status: "" is an explicit clear, same pattern as due/agent.
    let work_status: Option<Option<crate::task::WorkStatus>> = req
        .work_status
        .as_deref()
        .map(|s| {
            if s.is_empty() {
                Ok(None)
            } else {
                crate::task::WorkStatus::from_str(s).map(Some)
            }
        })
        .transpose()
        .map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?;
    // `due: ""` is an explicit clear, distinct from the field being absent entirely.
    let due_date: Option<Option<chrono::NaiveDate>> = req
        .due
        .as_deref()
        .map(|d| {
            if d.is_empty() {
                Ok(None)
            } else {
                crate::parser::parse_due_date_input(d, today)
                    .map(Some)
                    .ok_or_else(|| format!("Invalid due date: '{}'", d))
            }
        })
        .transpose()
        .map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?;
    let tags = req.tags.as_deref().map(|s| parse_csv(Some(s)));

    let _guard = state.write_lock.lock().await;
    let mut task_file = db::load(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let updated_task = {
        let t = task_file
            .find_task_mut(id)
            .ok_or_else(|| app_error(StatusCode::NOT_FOUND, format!("Task {} not found", id)))?;
        if let Some(title) = req.title {
            t.title = title;
        }
        if let Some(p) = priority {
            t.priority = p;
        }
        if let Some(d) = due_date {
            t.due_date = d;
        }
        if let Some(p) = req.project {
            t.project = if p.is_empty() { None } else { Some(p) };
        }
        if let Some(tg) = tags {
            t.tags = tg;
        }
        if let Some(a) = req.agent {
            t.agent = if a.is_empty() { None } else { Some(a) };
        }
        if let Some(d) = req.description {
            t.description = Some(d);
        }
        if let Some(e) = effort {
            t.effort = Some(e);
        }
        if let Some(w) = work_status {
            t.work_status = w;
        }
        t.updated = Some(chrono::Utc::now());
        t.clone()
    };

    db::save(&state.db_path, &task_file).map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(updated_task))
}

#[derive(Serialize)]
pub struct DoneResponse {
    pub completed: Task,
    pub spawned: Option<Task>,
}

async fn done_task(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<DoneResponse>, (StatusCode, Json<serde_json::Value>)> {
    let _guard = state.write_lock.lock().await;
    let mut task_file = db::load(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let idx = task_file
        .tasks
        .iter()
        .position(|t| t.id == id)
        .ok_or_else(|| app_error(StatusCode::NOT_FOUND, format!("Task {} not found", id)))?;

    if task_file.tasks[idx].status == Status::Done {
        return Ok(Json(DoneResponse { completed: task_file.tasks[idx].clone(), spawned: None }));
    }

    task_file.tasks[idx].status = Status::Done;
    task_file.tasks[idx].updated = Some(chrono::Utc::now());

    let mut spawned = None;
    if let Some(recur) = task_file.tasks[idx].recurrence {
        let parent = task_file.tasks[idx].clone();
        let next_due = crate::task::next_due_date(&recur, parent.due_date);
        let new_id = task_file.next_id;
        task_file.next_id += 1;
        let new_task = Task {
            id: new_id,
            title: parent.title.clone(),
            status: Status::Open,
            priority: parent.priority,
            tags: parent.tags.clone(),
            created: chrono::Utc::now(),
            updated: None,
            description: parent.description.clone(),
            due_date: Some(next_due),
            project: parent.project.clone(),
            recurrence: Some(recur),
            notes: parent.notes.clone(),
            agent: parent.agent.clone(),
            effort: parent.effort,
            work_status: None,
        };
        task_file.tasks.push(new_task.clone());
        spawned = Some(new_task);
    }

    let completed = task_file.tasks[idx].clone();
    db::save(&state.db_path, &task_file).map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(DoneResponse { completed, spawned }))
}

async fn reopen_task(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<Task>, (StatusCode, Json<serde_json::Value>)> {
    let _guard = state.write_lock.lock().await;
    let mut task_file = db::load(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let updated_task = {
        let t = task_file
            .find_task_mut(id)
            .ok_or_else(|| app_error(StatusCode::NOT_FOUND, format!("Task {} not found", id)))?;
        t.status = Status::Open;
        t.updated = Some(chrono::Utc::now());
        t.clone()
    };
    db::save(&state.db_path, &task_file).map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(updated_task))
}

async fn delete_task(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let _guard = state.write_lock.lock().await;
    let mut task_file = db::load(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    task_file
        .remove_task(id)
        .ok_or_else(|| app_error(StatusCode::NOT_FOUND, format!("Task {} not found", id)))?;
    db::save(&state.db_path, &task_file).map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_agents() -> Json<Vec<AgentProfile>> {
    let profiles = crate::config::list_agent_profiles()
        .into_iter()
        .map(|(name, dir)| AgentProfile { name, dir })
        .collect();
    Json(profiles)
}

fn notes_dir(state: &AppState) -> PathBuf {
    state
        .db_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("Notes")
}

/// Slugs become filenames (`{slug}.md`) joined onto the notes directory. Reject anything
/// that isn't alphanumeric/hyphen/underscore so a slug can never escape that directory
/// (e.g. via `..` path segments or an absolute path).
fn is_valid_slug(slug: &str) -> bool {
    !slug.is_empty() && slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[derive(Serialize)]
pub struct NoteResponse {
    pub slug: String,
    pub title: String,
    pub body: String,
}

impl From<crate::note::Note> for NoteResponse {
    fn from(n: crate::note::Note) -> Self {
        NoteResponse { slug: n.slug, title: n.title, body: n.body }
    }
}

async fn list_task_notes(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<Vec<NoteResponse>>, (StatusCode, Json<serde_json::Value>)> {
    let task_file = db::load(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let task = task_file
        .find_task(id)
        .ok_or_else(|| app_error(StatusCode::NOT_FOUND, format!("Task {} not found", id)))?;

    let dir = notes_dir(&state);
    let notes: Vec<NoteResponse> = task
        .notes
        .iter()
        .filter(|slug| is_valid_slug(slug))
        .filter_map(|slug| crate::note::read_note(&dir.join(format!("{}.md", slug))).ok())
        .map(NoteResponse::from)
        .collect();
    Ok(Json(notes))
}

#[derive(Deserialize)]
pub struct CreateNoteRequest {
    pub title: String,
}

async fn create_task_note(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
    Json(req): Json<CreateNoteRequest>,
) -> Result<Json<NoteResponse>, (StatusCode, Json<serde_json::Value>)> {
    let _guard = state.write_lock.lock().await;
    let mut task_file = db::load(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    if task_file.find_task(id).is_none() {
        return Err(app_error(StatusCode::NOT_FOUND, format!("Task {} not found", id)));
    }

    let dir = notes_dir(&state);
    let base_slug = crate::note::slugify(&req.title);
    let slug = crate::note::unique_slug(&dir, &base_slug);
    let note = crate::note::Note { slug: slug.clone(), title: req.title.clone(), body: String::new() };
    crate::note::write_note(&dir, &note).map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let t = task_file.find_task_mut(id).expect("checked above");
    t.notes.push(slug.clone());
    db::save(&state.db_path, &task_file).map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    Ok(Json(NoteResponse::from(note)))
}

/// Mirrors tui.rs's `build_obsidian_uri`: an Obsidian deep link when a vault is configured.
fn build_obsidian_uri(slug: &str) -> Option<String> {
    let vault = crate::config::read_config_value("obsidian-vault")?;
    let notes_dir = crate::config::read_config_value("obsidian-notes-dir");
    let file = match notes_dir {
        Some(ref dir) => format!("{}/{}", dir, slug),
        None => slug.to_string(),
    };
    Some(format!("obsidian://open?vault={}&file={}", vault, file))
}

/// Mirrors tui.rs's `open_note_external`, minus the raw-mode terminal handling that has
/// no equivalent in a headless server process — every spawn here is fire-and-forget.
/// Priority: Obsidian (if configured) > $EDITOR/$VISUAL.
fn open_note_external(note_path: &std::path::Path, slug: &str) -> Result<(), String> {
    if let Some(uri) = build_obsidian_uri(slug) {
        std::process::Command::new("open")
            .arg(&uri)
            .spawn()
            .map_err(|e| format!("Failed to open Obsidian: {}", e))?;
        return Ok(());
    }
    let editor = std::env::var("EDITOR").or_else(|_| std::env::var("VISUAL")).ok();
    match editor {
        Some(ed) => {
            std::process::Command::new(&ed)
                .arg(note_path)
                .spawn()
                .map_err(|e| format!("Failed to launch editor '{}': {}", ed, e))?;
            Ok(())
        }
        None => Err("No editor configured. Set $EDITOR or add obsidian-vault to config.".to_string()),
    }
}

async fn open_note(
    State(state): State<Arc<AppState>>,
    AxumPath(slug): AxumPath<String>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    if !is_valid_slug(&slug) {
        return Err(app_error(StatusCode::BAD_REQUEST, "invalid note slug"));
    }
    let dir = notes_dir(&state);
    let note_path = dir.join(format!("{}.md", slug));
    open_note_external(&note_path, &slug).map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn unlink_task_note(
    State(state): State<Arc<AppState>>,
    AxumPath((id, slug)): AxumPath<(u32, String)>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    if !is_valid_slug(&slug) {
        return Err(app_error(StatusCode::BAD_REQUEST, "invalid note slug"));
    }
    let _guard = state.write_lock.lock().await;
    let mut task_file = db::load(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let t = task_file
        .find_task_mut(id)
        .ok_or_else(|| app_error(StatusCode::NOT_FOUND, format!("Task {} not found", id)))?;
    t.notes.retain(|s| s != &slug);
    db::save(&state.db_path, &task_file).map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn router(state: AppState) -> Router {
    router_with_static(state, None)
}

pub fn router_with_static(state: AppState, static_dir: Option<std::path::PathBuf>) -> Router {
    let state = Arc::new(state);
    let mut app = Router::new()
        .route("/api/tasks", get(list_tasks).post(add_task))
        .route("/api/tasks/:id", get(get_task).patch(edit_task).delete(delete_task))
        .route("/api/tasks/:id/done", axum::routing::post(done_task))
        .route("/api/tasks/:id/reopen", axum::routing::post(reopen_task))
        .route("/api/tasks/:id/notes", get(list_task_notes).post(create_task_note))
        .route("/api/tasks/:id/notes/:slug", axum::routing::delete(unlink_task_note))
        .route("/api/notes/:slug/open", axum::routing::post(open_note))
        .route("/api/agents", get(list_agents));

    if let Some(dir) = static_dir {
        app = app.fallback_service(tower_http::services::ServeDir::new(dir));
    }

    app.layer(middleware::from_fn(validate_host)).with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tempfile::tempdir;
    use tower::ServiceExt;

    fn make_state() -> (tempfile::TempDir, AppState) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tasks.db");
        (dir, AppState { db_path, write_lock: Arc::new(tokio::sync::Mutex::new(())) })
    }

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn test_list_tasks_empty() {
        let (_dir, state) = make_state();
        let app = router(state);
        let response = app
            .oneshot(Request::builder().uri("/api/tasks").header("host", "127.0.0.1").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json, serde_json::json!([]));
    }

    #[tokio::test]
    async fn test_list_tasks_returns_seeded_data() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1,
                title: "Seeded task".to_string(),
                status: Status::Open,
                priority: crate::task::Priority::High,
                tags: vec!["x".to_string()],
                created: chrono::Utc::now(),
                updated: None,
                description: None,
                due_date: None,
                project: None,
                recurrence: None,
                notes: Vec::new(),
                agent: None,
                effort: None,
                work_status: None,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let response = app
            .oneshot(Request::builder().uri("/api/tasks").header("host", "127.0.0.1").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json[0]["title"], "Seeded task");
        assert_eq!(json[0]["id"], 1);
    }

    #[tokio::test]
    async fn test_list_tasks_filters_by_status_query_param() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            let t1 = crate::task::Task {
                id: 1, title: "Open".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None, work_status: None,
            };
            let mut t2 = t1.clone();
            t2.id = 2; t2.title = "Done".to_string(); t2.status = Status::Done;
            tf.tasks.push(t1);
            tf.tasks.push(t2);
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let response = app
            .oneshot(Request::builder().uri("/api/tasks?status=open").header("host", "127.0.0.1").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let json = body_json(response).await;
        assert_eq!(json.as_array().unwrap().len(), 1);
        assert_eq!(json[0]["title"], "Open");
    }

    #[tokio::test]
    async fn test_get_task_by_id() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 7, title: "Found me".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None, work_status: None,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let response = app
            .oneshot(Request::builder().uri("/api/tasks/7").header("host", "127.0.0.1").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["title"], "Found me");
    }

    #[tokio::test]
    async fn test_get_task_missing_returns_404() {
        let (_dir, state) = make_state();
        let app = router(state);
        let response = app
            .oneshot(Request::builder().uri("/api/tasks/999").header("host", "127.0.0.1").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_host_header_rejects_spoofed_host() {
        let (_dir, state) = make_state();
        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tasks")
                    .header("host", "evil.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_host_header_accepts_127_0_0_1_with_port() {
        let (_dir, state) = make_state();
        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tasks")
                    .header("host", "127.0.0.1:4287")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_add_task_creates_and_returns_task() {
        let (_dir, state) = make_state();
        let app = router(state);
        let body = serde_json::json!({ "title": "New task", "priority": "high" });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["title"], "New task");
        assert_eq!(json["priority"], "high");
        assert_eq!(json["id"], 1);
    }

    #[tokio::test]
    async fn test_add_task_invalid_due_date_returns_400() {
        let (_dir, state) = make_state();
        let app = router(state);
        let body = serde_json::json!({ "title": "Bad due", "due": "not-a-date" });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_concurrent_add_task_requests_do_not_lose_writes() {
        let (_dir, state) = make_state();
        let app = router(state.clone());

        let make_request = |title: &str| {
            let body = serde_json::json!({ "title": title });
            Request::builder()
                .method("POST")
                .uri("/api/tasks")
                .header("host", "127.0.0.1")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap()
        };

        let app1 = app.clone();
        let app2 = app.clone();
        let (resp1, resp2) = tokio::join!(
            app1.oneshot(make_request("Task A")),
            app2.oneshot(make_request("Task B")),
        );
        let resp1 = resp1.unwrap();
        let resp2 = resp2.unwrap();
        assert_eq!(resp1.status(), StatusCode::OK);
        assert_eq!(resp2.status(), StatusCode::OK);

        let json1 = body_json(resp1).await;
        let json2 = body_json(resp2).await;
        let id1 = json1["id"].as_u64().unwrap();
        let id2 = json2["id"].as_u64().unwrap();
        assert_ne!(id1, id2, "both concurrent requests were assigned the same id");

        let task_file = db::load(&state.db_path).unwrap();
        assert_eq!(task_file.tasks.len(), 2, "one of the two concurrent writes was lost");
        let mut ids: Vec<u32> = task_file.tasks.iter().map(|t| t.id).collect();
        ids.sort();
        assert_eq!(ids, vec![1, 2]);
    }

    #[tokio::test]
    async fn test_add_task_invalid_priority_returns_400() {
        let (_dir, state) = make_state();
        let app = router(state);
        let body = serde_json::json!({ "title": "Bad", "priority": "urgent" });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_list_agents_returns_configured_profiles() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tasks.db");
        let config_path = dir.path().join("config.md");
        std::fs::write(&config_path, "agent-bot: /code/bot\n").unwrap();
        unsafe { std::env::set_var("TASK_CONFIG_FILE", &config_path) };
        let app = router(AppState { db_path, write_lock: Arc::new(tokio::sync::Mutex::new(())) });
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/agents")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        unsafe { std::env::remove_var("TASK_CONFIG_FILE") };
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json[0]["name"], "bot");
        assert_eq!(json[0]["dir"], "/code/bot");
    }

    #[tokio::test]
    async fn test_internal_error_returns_generic_message() {
        let dir = tempdir().unwrap();
        // A directory can't be opened as a sqlite file, so db::load fails here,
        // giving us a real internal error whose raw message must not leak to the client.
        let state = AppState {
            db_path: dir.path().to_path_buf(),
            write_lock: Arc::new(tokio::sync::Mutex::new(())),
        };
        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tasks")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let json = body_json(response).await;
        assert_eq!(json["error"], "internal server error");
    }

    #[tokio::test]
    async fn test_edit_task_updates_only_given_fields() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Original".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None, work_status: None,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let body = serde_json::json!({ "priority": "critical" });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/tasks/1")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["title"], "Original");
        assert_eq!(json["priority"], "critical");
        assert!(json["updated"].is_string());
    }

    #[tokio::test]
    async fn test_edit_task_empty_due_clears_it() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Has a due date".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 5), project: None, recurrence: None,
                notes: Vec::new(), agent: None, effort: None, work_status: None,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let body = serde_json::json!({ "due": "" });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/tasks/1")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert!(json["due_date"].is_null());
    }

    #[tokio::test]
    async fn test_edit_task_empty_agent_clears_it() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Has an agent".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(),
                agent: Some("bot".to_string()), effort: None, work_status: None,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let body = serde_json::json!({ "agent": "" });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/tasks/1")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert!(json["agent"].is_null());
    }

    #[tokio::test]
    async fn test_edit_task_sets_work_status() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Track me".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None,
                work_status: None,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let body = serde_json::json!({ "work_status": "in-progress" });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/tasks/1")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["work_status"], "in-progress");
    }

    #[tokio::test]
    async fn test_edit_task_invalid_work_status_returns_400() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Track me".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None,
                work_status: None,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let body = serde_json::json!({ "work_status": "bogus" });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/tasks/1")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_edit_task_empty_work_status_clears_it() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Track me".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None,
                work_status: Some(crate::task::WorkStatus::Complete),
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let body = serde_json::json!({ "work_status": "" });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/tasks/1")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert!(json["work_status"].is_null());
    }

    #[tokio::test]
    async fn test_edit_task_missing_id_returns_404() {
        let (_dir, state) = make_state();
        let app = router(state);
        let body = serde_json::json!({ "title": "x" });
        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/tasks/999")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_done_task_marks_status_done() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Finish me".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None, work_status: None,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks/1/done")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["completed"]["status"], "done");
        assert!(json["spawned"].is_null());
    }

    #[tokio::test]
    async fn test_done_task_recurring_spawns_next_occurrence() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Recurring".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 5), project: None,
                recurrence: Some(crate::task::Recurrence::from_str("weekly").unwrap()),
                notes: Vec::new(), agent: None, effort: None, work_status: None,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks/1/done")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["completed"]["status"], "done");
        assert_eq!(json["spawned"]["id"], 2);
        assert_eq!(json["spawned"]["due_date"], "2026-01-12");
    }

    #[tokio::test]
    async fn test_done_task_already_done_does_not_spawn_duplicate() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Recurring".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 5), project: None,
                recurrence: Some(crate::task::Recurrence::from_str("weekly").unwrap()),
                notes: Vec::new(), agent: None, effort: None, work_status: None,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks/1/done")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let tasks_after_first = db::load(&state.db_path).unwrap().tasks.len();

        let app = router(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks/1/done")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["completed"]["status"], "done");
        assert!(json["spawned"].is_null());

        let tasks_after_second = db::load(&state.db_path).unwrap().tasks.len();
        assert_eq!(tasks_after_first, tasks_after_second);
    }

    #[tokio::test]
    async fn test_reopen_task_sets_status_open() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Reopen me".to_string(), status: Status::Done, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None, work_status: None,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks/1/reopen")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["status"], "open");
    }

    #[tokio::test]
    async fn test_delete_task_removes_it() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Doomed".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None, work_status: None,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let db_path = state.db_path.clone();
        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/tasks/1")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        let tf = db::load(&db_path).unwrap();
        assert!(tf.tasks.is_empty());
    }

    #[tokio::test]
    async fn test_delete_task_missing_returns_404() {
        let (_dir, state) = make_state();
        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/tasks/999")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_static_fallback_rejects_spoofed_host() {
        let (_dir, state) = make_state();
        let static_dir = tempdir().unwrap();
        std::fs::write(static_dir.path().join("index.html"), "<html>hi</html>").unwrap();
        let app = router_with_static(state, Some(static_dir.path().to_path_buf()));
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/index.html")
                    .header("host", "evil.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_open_note_rejects_path_traversal_slug() {
        let (_dir, state) = make_state();
        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/notes/..%2f..%2f..%2fetc%2fpasswd/open")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_unlink_task_note_rejects_path_traversal_slug() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Has notes".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None,
                due_date: None, project: None, recurrence: None, notes: vec!["real-note".to_string()], agent: None, effort: None, work_status: None,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/tasks/1/notes/..%2f..%2fetc%2fpasswd")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_static_fallback_serves_file_with_valid_host() {
        let (_dir, state) = make_state();
        let static_dir = tempdir().unwrap();
        std::fs::write(static_dir.path().join("index.html"), "<html>hi</html>").unwrap();
        let app = router_with_static(state, Some(static_dir.path().to_path_buf()));
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/index.html")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(bytes, "<html>hi</html>".as_bytes());
    }
}
