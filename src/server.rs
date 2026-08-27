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
    let mut task_file = db::load(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let priority_str = req.priority.as_deref().unwrap_or("medium");
    let priority = crate::task::Priority::from_str(priority_str)
        .map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?;
    let today = chrono::Local::now().date_naive();
    let due_date = req.due.as_deref().and_then(|d| crate::parser::parse_due_date_input(d, today));

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
}

async fn edit_task(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
    Json(req): Json<EditTaskRequest>,
) -> Result<Json<Task>, (StatusCode, Json<serde_json::Value>)> {
    let mut task_file = db::load(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;

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
    let due_date = req
        .due
        .as_deref()
        .map(|d| {
            crate::parser::parse_due_date_input(d, today)
                .ok_or_else(|| format!("Invalid due date: '{}'", d))
        })
        .transpose()
        .map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?;
    let tags = req.tags.as_deref().map(|s| parse_csv(Some(s)));

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
            t.due_date = Some(d);
        }
        if let Some(p) = req.project {
            t.project = Some(p);
        }
        if let Some(tg) = tags {
            t.tags = tg;
        }
        if let Some(a) = req.agent {
            t.agent = Some(a);
        }
        if let Some(d) = req.description {
            t.description = Some(d);
        }
        if let Some(e) = effort {
            t.effort = Some(e);
        }
        t.updated = Some(chrono::Utc::now());
        t.clone()
    };

    db::save(&state.db_path, &task_file).map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(updated_task))
}

async fn list_agents() -> Json<Vec<AgentProfile>> {
    let profiles = crate::config::list_agent_profiles()
        .into_iter()
        .map(|(name, dir)| AgentProfile { name, dir })
        .collect();
    Json(profiles)
}

pub fn router(state: AppState) -> Router {
    let state = Arc::new(state);
    Router::new()
        .route("/api/tasks", get(list_tasks).post(add_task))
        .route("/api/tasks/:id", get(get_task).patch(edit_task))
        .route("/api/agents", get(list_agents))
        .layer(middleware::from_fn(validate_host))
        .with_state(state)
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
        (dir, AppState { db_path })
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
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None,
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
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None,
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
        let app = router(AppState { db_path });
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
        let state = AppState { db_path: dir.path().to_path_buf() };
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
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None,
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
}
