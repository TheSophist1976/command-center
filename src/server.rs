use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::get;
use axum::Router;
use serde::Deserialize;
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
    (status, Json(serde_json::json!({ "error": message.into() })))
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

pub fn router(state: AppState) -> Router {
    let state = Arc::new(state);
    Router::new()
        .route("/api/tasks", get(list_tasks))
        .route("/api/tasks/:id", get(get_task))
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
            .oneshot(Request::builder().uri("/api/tasks").body(Body::empty()).unwrap())
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
            .oneshot(Request::builder().uri("/api/tasks").body(Body::empty()).unwrap())
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
            .oneshot(Request::builder().uri("/api/tasks?status=open").body(Body::empty()).unwrap())
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
            .oneshot(Request::builder().uri("/api/tasks/7").body(Body::empty()).unwrap())
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
            .oneshot(Request::builder().uri("/api/tasks/999").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
