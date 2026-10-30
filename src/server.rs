use std::path::{Path, PathBuf};
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
use crate::lifecycle::{self, Actor, Event};
use crate::task::{Status, Task};

#[derive(Clone)]
pub struct AppState {
    pub db_path: PathBuf,
    /// Where Notes live. Resolved once at startup (see `db::resolve_notes_dir`) so
    /// the `notes-dir` config key is honored the same way the CLI and TUI honor it.
    pub notes_dir: PathBuf,
    pub write_lock: Arc<tokio::sync::Mutex<()>>,
    /// Broadcasts a signal whenever tasks.db changes on disk, from any writer
    /// (this server's own handlers, the CLI, or any other process). Populated
    /// by `crate::watch::spawn` in the task_server binary; in tests it's just
    /// an unused channel with no watcher attached.
    pub change_tx: tokio::sync::broadcast::Sender<()>,
    /// Version cache, update status and the background update job.
    pub update: Arc<crate::update_state::UpdateState>,
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
) -> Result<Json<Vec<TaskView>>, (StatusCode, Json<serde_json::Value>)> {
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

    // Services live on the Services screen (/api/services), never in task lists.
    let rows: Vec<Task> = task_file.tasks.into_iter().filter(|t| !t.is_service()).collect();
    let tasks: Vec<TaskView> = filter_and_sort_tasks(
        &rows,
        status,
        query.agent.as_deref(),
        query.project.as_deref(),
        query.tag.as_deref(),
        due_before,
    )
    .into_iter()
    .map(|t| t.clone().into())
    .collect();

    Ok(Json(tasks))
}

/// A task plus its lifecycle state and the moves the human may make now. The
/// web UI shows controls from `allowed_actions` and never decides them itself.
#[derive(Serialize)]
pub struct TaskView {
    #[serde(flatten)]
    pub task: Task,
    pub state: lifecycle::State,
    pub allowed_actions: Vec<Event>,
}

impl From<Task> for TaskView {
    fn from(task: Task) -> Self {
        TaskView { state: lifecycle::state(&task), allowed_actions: lifecycle::allowed_events(&task, Actor::Human), task }
    }
}

#[derive(Serialize)]
pub struct ServiceView {
    #[serde(flatten)]
    pub task: TaskView,
    pub last_run: Option<db::ServiceRun>,
    pub run_count: i64,
}

async fn list_services(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ServiceView>>, (StatusCode, Json<serde_json::Value>)> {
    let task_file = db::load(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let mut runs = db::run_summaries(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let services: Vec<&Task> = task_file.tasks.iter().filter(|t| t.is_service()).collect();
    let out = crate::commands::sort_by_due_then_priority(services)
        .into_iter()
        .map(|t| {
            let (last_run, run_count) = match runs.remove(&t.id) {
                Some((r, n)) => (Some(r), n),
                None => (None, 0),
            };
            ServiceView { task: t.clone().into(), last_run, run_count }
        })
        .collect();
    Ok(Json(out))
}

#[derive(Deserialize)]
pub struct AddServiceRequest {
    pub title: String,
    pub recurrence: String,
    pub due: Option<String>,
    pub priority: Option<String>,
    pub project: Option<String>,
    pub tags: Option<String>,
    pub agent: Option<String>,
    pub description: Option<String>,
    pub instructions: Option<String>,
}

fn cli_error((_, msg): (i32, String)) -> (StatusCode, Json<serde_json::Value>) {
    let status = if msg.contains("not found") { StatusCode::NOT_FOUND } else { StatusCode::BAD_REQUEST };
    app_error(status, msg)
}

async fn add_service(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AddServiceRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let today = chrono::Local::now().date_naive();
    let args = crate::commands::AddArgs {
        title: req.title,
        priority: req.priority.unwrap_or_else(|| "medium".to_string()),
        due: req.due.filter(|d| !d.is_empty()),
        project: req.project.filter(|p| !p.is_empty()),
        tags: req.tags,
        agent: req.agent.filter(|a| !a.is_empty()),
        description: req.description,
        instructions: req.instructions,
    };
    let _guard = state.write_lock.lock().await;
    let msg = crate::commands::service_add(&state.db_path, args, &req.recurrence, today).map_err(cli_error)?;
    Ok(Json(serde_json::json!({ "message": msg })))
}

async fn pause_service(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let _guard = state.write_lock.lock().await;
    let today = chrono::Local::now().date_naive();
    let msg = crate::commands::set_paused(&state.db_path, id, true, today).map_err(cli_error)?;
    Ok(Json(serde_json::json!({ "message": msg })))
}

async fn resume_service(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let _guard = state.write_lock.lock().await;
    let today = chrono::Local::now().date_naive();
    let msg = crate::commands::set_paused(&state.db_path, id, false, today).map_err(cli_error)?;
    Ok(Json(serde_json::json!({ "message": msg })))
}

#[derive(Deserialize)]
pub struct RunsQuery {
    pub limit: Option<usize>,
    pub before: Option<i64>,
}

async fn list_service_runs(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
    Query(q): Query<RunsQuery>,
) -> Result<Json<Vec<db::ServiceRun>>, (StatusCode, Json<serde_json::Value>)> {
    let runs = db::list_runs(&state.db_path, id, q.limit.unwrap_or(50).min(500), q.before)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(runs))
}

async fn get_task(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<TaskView>, (StatusCode, Json<serde_json::Value>)> {
    let task_file = db::load(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    task_file
        .find_task(id)
        .cloned()
        .map(|t| Json(t.into()))
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
    pub instructions: Option<String>,
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
        .map(|d| crate::parser::parse_new_due_date(d, today))
        .transpose()
        .map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?;

    let _guard = state.write_lock.lock().await;
    let mut task_file = db::begin(&state.db_path)
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
        instructions: req.instructions,
        due_date,
        project: req.project,
        recurrence: None,
        notes: Vec::new(),
        agent: req.agent,
        effort: None,
        work_status: None, kind: Default::default(), paused: false,
    };
    task_file.tasks.push(new_task.clone());
    task_file.commit().map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
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
    pub instructions: Option<String>,
    pub effort: Option<String>,
    pub work_status: Option<String>,
    pub recurrence: Option<String>,
    /// "service" converts the task to a service, "task" turns it back.
    pub kind: Option<String>,
}

async fn edit_task(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
    Json(req): Json<EditTaskRequest>,
) -> Result<Json<TaskView>, (StatusCode, Json<serde_json::Value>)> {
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
                crate::parser::parse_new_due_date(d, today).map(Some)
            }
        })
        .transpose()
        .map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?;
    // recurrence: "" is an explicit clear, same pattern as work_status/due/agent.
    let recurrence: Option<Option<crate::task::Recurrence>> = req
        .recurrence
        .as_deref()
        .map(|s| {
            if s.is_empty() {
                Ok(None)
            } else {
                crate::task::Recurrence::from_str(s).map(Some)
            }
        })
        .transpose()
        .map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?;
    let kind = req
        .kind
        .as_deref()
        .map(crate::task::TaskKind::from_str)
        .transpose()
        .map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?;
    let tags = req.tags.as_deref().map(|s| parse_csv(Some(s)));

    let _guard = state.write_lock.lock().await;
    let mut task_file = db::begin(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let updated_task = {
        let t = task_file
            .find_task_mut(id)
            .ok_or_else(|| app_error(StatusCode::NOT_FOUND, format!("Task {} not found", id)))?;
        let review_transition = if work_status.is_none()
            && t.work_status == Some(crate::task::WorkStatus::WaitingForReview)
        {
            review_handoff(t, req.agent.as_deref(), due_date)
        } else {
            None
        };
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
        if let Some(i) = req.instructions {
            t.instructions = if i.is_empty() { None } else { Some(i) };
        }
        if let Some(e) = effort {
            t.effort = Some(e);
        }
        if let Some(r) = recurrence {
            t.recurrence = r;
        }
        // State moves go through the lifecycle, in order: review handoff,
        // kind conversion, then an explicit work status.
        let mut events = Vec::new();
        match review_transition {
            Some(ReviewHandoff::Defer) => events.push(Event::Defer),
            Some(ReviewHandoff::Reassign) => events.push(Event::Reset),
            None => {}
        }
        match kind {
            Some(k) if k != t.kind => {
                events.push(if k == crate::task::TaskKind::Service { Event::ToService } else { Event::ToTask })
            }
            _ => {}
        }
        // Resending the current status (a full edit form) is not a move.
        if let Some(w) = work_status.filter(|w| *w != t.work_status) {
            events.push(lifecycle::event_for_work_status(t, w).map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?);
        }
        for event in events {
            lifecycle::apply(t, event, Actor::Human, false, today).map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?;
        }
        t.validate_kind().map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?;
        t.updated = Some(chrono::Utc::now());
        t.clone()
    };

    task_file.commit().map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(updated_task.into()))
}

enum ReviewHandoff {
    /// Due date pushed later: the human takes the task back and its work status is cleared.
    Defer,
    /// A (non-human) agent was assigned: the task goes back on that agent's queue.
    Reassign,
}

/// Decides how an edit to a `waiting-for-review` task moves it out of review (task 36).
/// Only called when the edit does not set `work_status` itself; an explicit status wins.
/// Deferring takes precedence over assigning an agent in the same edit.
fn review_handoff(
    t: &Task,
    new_agent: Option<&str>,
    new_due: Option<Option<chrono::NaiveDate>>,
) -> Option<ReviewHandoff> {
    if let Some(Some(due)) = new_due {
        if t.due_date.map_or(true, |old| due > old) {
            return Some(ReviewHandoff::Defer);
        }
    }
    match new_agent {
        Some(a)
            if !a.is_empty()
                && !a.eq_ignore_ascii_case("human")
                && !t.agent.as_deref().is_some_and(|old| old.eq_ignore_ascii_case(a)) =>
        {
            Some(ReviewHandoff::Reassign)
        }
        _ => None,
    }
}

#[derive(Serialize)]
pub struct DoneResponse {
    pub completed: TaskView,
    pub spawned: Option<TaskView>,
}

async fn done_task(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<DoneResponse>, (StatusCode, Json<serde_json::Value>)> {
    let _guard = state.write_lock.lock().await;
    let mut task_file = db::begin(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    if task_file.find_task(id).is_none() {
        return Err(app_error(StatusCode::NOT_FOUND, format!("Task {} not found", id)));
    }
    let c = lifecycle::complete(&mut task_file, id, Actor::Human, chrono::Local::now().date_naive())
        .map_err(|e| app_error(StatusCode::BAD_REQUEST, e))?;
    let completed = task_file.find_task(id).cloned().expect("checked above");
    if !c.already_done {
        task_file.commit().map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    }
    Ok(Json(DoneResponse { completed: completed.into(), spawned: c.spawned.map(Into::into) }))
}

async fn reopen_task(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<TaskView>, (StatusCode, Json<serde_json::Value>)> {
    let _guard = state.write_lock.lock().await;
    let mut task_file = db::begin(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let updated_task = {
        let t = task_file
            .find_task_mut(id)
            .ok_or_else(|| app_error(StatusCode::NOT_FOUND, format!("Task {} not found", id)))?;
        lifecycle::apply(t, Event::Reopen, Actor::Human, false, chrono::Local::now().date_naive())
            .map_err(|e| app_error(StatusCode::CONFLICT, e))?;
        t.clone()
    };
    task_file.commit().map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(updated_task.into()))
}

async fn delete_task(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let _guard = state.write_lock.lock().await;
    let mut task_file = db::begin(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let removed = task_file
        .remove_task(id)
        .ok_or_else(|| app_error(StatusCode::NOT_FOUND, format!("Task {} not found", id)))?;
    if removed.is_service() {
        task_file.delete_runs(id).map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    }
    task_file.commit().map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_agents() -> Json<Vec<AgentProfile>> {
    let profiles = crate::config::list_agent_profiles()
        .into_iter()
        .map(|(name, dir)| AgentProfile { name, dir })
        .collect();
    Json(profiles)
}

/// Mirrors the CLI's `task agent instructions`/`task agent memory` slug resolution
/// (src/bin/task.rs), so the web UI edits the exact same files.
fn agent_notes_dir(state: &AppState, slug: &str) -> PathBuf {
    crate::agent::agent_dir(&state.notes_dir, slug)
}

fn agent_exists(name: &str) -> bool {
    crate::config::list_agent_profiles()
        .into_iter()
        .any(|(n, _)| n == name)
}

fn instructions_slug(name: &str) -> String {
    crate::agent::instructions_slug(name)
}

async fn get_agent_instructions(
    State(state): State<Arc<AppState>>,
    AxumPath(name): AxumPath<String>,
) -> Result<Json<Option<NoteResponse>>, (StatusCode, Json<serde_json::Value>)> {
    if !agent_exists(&name) {
        return Err(app_error(StatusCode::NOT_FOUND, format!("Unknown agent '{}'", name)));
    }
    let slug = instructions_slug(&name);
    let agents_dir = agent_notes_dir(&state, &slug);
    let note_path = agents_dir.join("instructions.md");
    let legacy_path = state
        .db_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("Notes")
        .join("Instructions")
        .join(format!("{}.md", slug));
    let read_path = if note_path.exists() { &note_path } else { &legacy_path };
    let note = crate::note::read_note(read_path)
        .ok()
        .map(|n| note_response(n, modified_time(read_path)));
    Ok(Json(note))
}

async fn edit_agent_instructions(
    State(state): State<Arc<AppState>>,
    AxumPath(name): AxumPath<String>,
    Json(req): Json<EditNoteRequest>,
) -> Result<Json<NoteResponse>, (StatusCode, Json<serde_json::Value>)> {
    if !agent_exists(&name) {
        return Err(app_error(StatusCode::NOT_FOUND, format!("Unknown agent '{}'", name)));
    }
    let _guard = state.write_lock.lock().await;
    let slug = instructions_slug(&name);
    let agents_dir = agent_notes_dir(&state, &slug);
    let note_path = agents_dir.join("instructions.md");
    let legacy_path = state
        .db_path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join("Notes")
        .join("Instructions")
        .join(format!("{}.md", slug));
    let read_path = if note_path.exists() { &note_path } else { &legacy_path };
    let existing = crate::note::read_note(read_path).ok();

    let title = req
        .title
        .unwrap_or_else(|| existing.as_ref().map(|n| n.title.clone()).unwrap_or_else(|| format!("{} Instructions", name)));
    let body = req.body.unwrap_or_else(|| existing.as_ref().map(|n| n.body.clone()).unwrap_or_default());
    let note = crate::note::Note { slug: "instructions".to_string(), title, body };
    let note_path = crate::note::write_note(&agents_dir, &note)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let modified = modified_time(&note_path);
    Ok(Json(note_response(note, modified)))
}

async fn get_agent_memory(
    State(state): State<Arc<AppState>>,
    AxumPath(name): AxumPath<String>,
) -> Result<Json<Option<NoteResponse>>, (StatusCode, Json<serde_json::Value>)> {
    if !agent_exists(&name) {
        return Err(app_error(StatusCode::NOT_FOUND, format!("Unknown agent '{}'", name)));
    }
    let slug = crate::note::slugify(&name);
    let agents_dir = agent_notes_dir(&state, &slug);
    let note_path = agents_dir.join("memory.md");
    let note = crate::note::read_note(&note_path)
        .ok()
        .map(|n| note_response(n, modified_time(&note_path)));
    Ok(Json(note))
}

async fn edit_agent_memory(
    State(state): State<Arc<AppState>>,
    AxumPath(name): AxumPath<String>,
    Json(req): Json<EditNoteRequest>,
) -> Result<Json<NoteResponse>, (StatusCode, Json<serde_json::Value>)> {
    if !agent_exists(&name) {
        return Err(app_error(StatusCode::NOT_FOUND, format!("Unknown agent '{}'", name)));
    }
    let _guard = state.write_lock.lock().await;
    let slug = crate::note::slugify(&name);
    let agents_dir = agent_notes_dir(&state, &slug);
    let note_path = agents_dir.join("memory.md");
    let existing = crate::note::read_note(&note_path).ok();

    let title = req
        .title
        .unwrap_or_else(|| existing.as_ref().map(|n| n.title.clone()).unwrap_or_else(|| format!("{} Memory", name)));
    let body = req.body.unwrap_or_else(|| existing.as_ref().map(|n| n.body.clone()).unwrap_or_default());
    let note = crate::note::Note { slug: "memory".to_string(), title, body };
    let note_path = crate::note::write_note(&agents_dir, &note)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let modified = modified_time(&note_path);
    Ok(Json(note_response(note, modified)))
}

/// Server-Sent Events stream that emits one event whenever tasks.db changes
/// on disk, from any writer — this server's own handlers, the CLI, or any
/// other process. The event carries no payload; clients are expected to
/// simply refetch on receipt, same as `AppState::change_tx`'s doc comment.
async fn task_events(
    State(state): State<Arc<AppState>>,
) -> axum::response::sse::Sse<impl futures_core::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>> {
    use tokio_stream::StreamExt as _;
    let rx = state.change_tx.subscribe();
    let stream = tokio_stream::wrappers::BroadcastStream::new(rx)
        .filter_map(|result| result.ok().map(|_| Ok(axum::response::sse::Event::default().data("changed"))));
    axum::response::sse::Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

fn notes_dir(state: &AppState) -> PathBuf {
    state.notes_dir.clone()
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
    /// Last-modified time of the note file, RFC 3339. Empty if it couldn't be read.
    pub updated: String,
}

fn note_response(n: crate::note::Note, modified: std::time::SystemTime) -> NoteResponse {
    let updated = chrono::DateTime::<chrono::Utc>::from(modified).to_rfc3339();
    NoteResponse { slug: n.slug, title: n.title, body: n.body, updated }
}

fn modified_time(path: &Path) -> std::time::SystemTime {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
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
    let mut notes: Vec<(std::time::SystemTime, NoteResponse)> = task
        .notes
        .iter()
        .filter(|slug| is_valid_slug(slug))
        .filter_map(|slug| {
            let path = dir.join(format!("{}.md", slug));
            let note = crate::note::read_note(&path).ok()?;
            let modified = modified_time(&path);
            Some((modified, note_response(note, modified)))
        })
        .collect();
    // Newest file (by last-modified time) first.
    notes.sort_by(|a, b| b.0.cmp(&a.0));
    let notes: Vec<NoteResponse> = notes.into_iter().map(|(_, n)| n).collect();
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
    let mut task_file = db::begin(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    if task_file.find_task(id).is_none() {
        return Err(app_error(StatusCode::NOT_FOUND, format!("Task {} not found", id)));
    }

    let dir = notes_dir(&state);
    let base_slug = crate::note::slugify(&req.title);
    let slug = crate::note::unique_slug(&dir, &base_slug);
    let note = crate::note::Note { slug: slug.clone(), title: req.title.clone(), body: String::new() };
    let note_path = crate::note::write_note(&dir, &note).map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let t = task_file.find_task_mut(id).expect("checked above");
    t.notes.push(slug.clone());
    task_file.commit().map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let modified = modified_time(&note_path);
    Ok(Json(note_response(note, modified)))
}

#[derive(Serialize)]
pub struct ThreadResponse {
    pub note: Option<NoteResponse>,
    /// What the composer offers right now, derived from the task's lifecycle state.
    pub allowed_action: crate::thread::Allowed,
    /// The human wrote last and the agent has not answered yet.
    pub unread: bool,
}

#[derive(Deserialize)]
pub struct ThreadPost {
    /// `answer`, `feedback`, `message` or `approve`.
    pub kind: String,
    #[serde(default)]
    pub text: String,
}

fn thread_response(dir: &Path, task: &Task) -> ThreadResponse {
    let view = crate::thread::view(dir, task.id);
    let unread = view.as_ref().is_some_and(|v| v.unread());
    let note = view.map(|v| {
        let slug = crate::thread::slug(task.id);
        let modified = modified_time(&dir.join(format!("{}.md", slug)));
        note_response(crate::note::Note { slug, title: format!("Conversation — {}", task.title), body: v.body }, modified)
    });
    ThreadResponse { note, allowed_action: crate::thread::allowed(task), unread }
}

/// Folds any old review/question notes into the thread the first time a task is touched.
async fn migrate_legacy_notes(state: &AppState, id: u32) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let dir = notes_dir(state);
    if crate::thread::pending_legacy(&dir, id).is_empty() {
        return Ok(());
    }
    let _guard = state.write_lock.lock().await;
    let mut task_file = db::begin(&state.db_path).map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let Some(t) = task_file.find_task_mut(id) else { return Ok(()) };
    let archived = crate::thread::migrate(&dir, t).map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    task_file.commit().map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    crate::thread::archive(&dir, &archived).map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))
}

async fn get_task_thread(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<ThreadResponse>, (StatusCode, Json<serde_json::Value>)> {
    migrate_legacy_notes(&state, id).await?;
    let task_file = db::load(&state.db_path).map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let task = task_file
        .find_task(id)
        .ok_or_else(|| app_error(StatusCode::NOT_FOUND, format!("Task {} not found", id)))?;
    Ok(Json(thread_response(&notes_dir(&state), task)))
}

/// Deprecated: the old review/question reads now return the unified thread note.
async fn get_task_thread_note(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<Option<NoteResponse>>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(get_task_thread(State(state), AxumPath(id)).await?.0.note))
}

async fn post_task_thread(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
    Json(req): Json<ThreadPost>,
) -> Result<Json<ThreadResponse>, (StatusCode, Json<serde_json::Value>)> {
    post_thread(&state, id, &req.kind, req.text.trim()).await.map(Json)
}

async fn post_thread(
    state: &AppState,
    id: u32,
    kind: &str,
    text: &str,
) -> Result<ThreadResponse, (StatusCode, Json<serde_json::Value>)> {
    use crate::thread::Kind;
    let note_kind = match kind {
        "answer" => Kind::Answer,
        "feedback" => Kind::Feedback,
        "message" => Kind::Message,
        "approve" => Kind::Approval,
        other => return Err(app_error(StatusCode::BAD_REQUEST, format!("unknown kind '{}': use answer, feedback, message or approve", other))),
    };
    if kind != "approve" && text.is_empty() {
        return Err(app_error(StatusCode::BAD_REQUEST, format!("{} text must not be empty", kind)));
    }

    let _guard = state.write_lock.lock().await;
    let mut task_file = db::begin(&state.db_path).map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let current = task_file
        .find_task(id)
        .cloned()
        .ok_or_else(|| app_error(StatusCode::NOT_FOUND, format!("Task {} not found", id)))?;

    let verb = match kind {
        "answer" => "answer a question on",
        "feedback" => "post review feedback on",
        "message" => "message the agent on",
        _ => "approve",
    };
    let allowed = crate::thread::allowed(&current);
    let permitted = if kind == "approve" { allowed.approve } else { allowed.send == Some(kind) };
    if !permitted {
        return Err(app_error(
            StatusCode::CONFLICT,
            format!("cannot {} task {}: it is {}", verb, id, crate::lifecycle::state_label(&current)),
        ));
    }

    let dir = notes_dir(state);
    let today = chrono::Local::now().date_naive();
    let archived;
    match kind {
        "approve" => {
            crate::lifecycle::complete(&mut task_file, id, Actor::Human, today)
                .map_err(|e| app_error(StatusCode::CONFLICT, e))?;
            let t = task_file.find_task_mut(id).expect("checked above");
            let body = if text.is_empty() { "Approved." } else { text };
            archived = crate::thread::post(&dir, t, note_kind, body, today)
                .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
        }
        _ => {
            // answer and feedback move the task; a message does not.
            let mut updated = match kind {
                "answer" => apply_event_to_copy(&task_file, id, Event::Answer)?,
                "feedback" => apply_event_to_copy(&task_file, id, Event::Feedback)?,
                _ => current,
            };
            archived = crate::thread::post(&dir, &mut updated, note_kind, text, today)
                .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
            if let Some(t) = task_file.find_task_mut(id) {
                *t = updated;
            }
        }
    }
    let task = task_file.find_task(id).cloned().expect("checked above");
    task_file.commit().map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    crate::thread::archive(&dir, &archived).map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(thread_response(&dir, &task))
}

/// Deprecated alias for `POST /thread` with kind `feedback` (or `message` when feedback is not open).
async fn add_task_review_feedback(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
    Json(req): Json<FeedbackRequest>,
) -> Result<Json<NoteResponse>, (StatusCode, Json<serde_json::Value>)> {
    legacy_post(&state, id, "feedback", &req.text).await
}

/// Deprecated alias for `POST /thread` with kind `answer`.
async fn answer_task_question(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
    Json(req): Json<FeedbackRequest>,
) -> Result<Json<NoteResponse>, (StatusCode, Json<serde_json::Value>)> {
    legacy_post(&state, id, "answer", &req.text).await
}

async fn legacy_post(
    state: &AppState,
    id: u32,
    kind: &str,
    text: &str,
) -> Result<Json<NoteResponse>, (StatusCode, Json<serde_json::Value>)> {
    let task_file = db::load(&state.db_path).map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    // Feedback on a task already in changes-requested is now a plain message.
    let in_changes = task_file.find_task(id).is_some_and(|t| crate::lifecycle::state_label(t) == "changes-requested");
    let kind = if in_changes { "message" } else { kind };
    let r = post_thread(state, id, kind, text.trim()).await?;
    r.note.map(Json).ok_or_else(|| app_error(StatusCode::INTERNAL_SERVER_ERROR, "thread was not written"))
}

#[derive(Deserialize)]
pub struct FeedbackRequest {
    pub text: String,
}

#[derive(Deserialize)]
pub struct EditNoteRequest {
    pub title: Option<String>,
    pub body: Option<String>,
}

/// Check a human event against the lifecycle on a copy of the task, so a
/// refused move is reported before any note is written.
fn apply_event_to_copy(
    task_file: &crate::task::TaskFile,
    id: u32,
    event: Event,
) -> Result<Task, (StatusCode, Json<serde_json::Value>)> {
    let mut t = task_file
        .find_task(id)
        .cloned()
        .ok_or_else(|| app_error(StatusCode::NOT_FOUND, format!("Task {} not found", id)))?;
    lifecycle::apply(&mut t, event, Actor::Human, false, chrono::Local::now().date_naive())
        .map_err(|e| app_error(StatusCode::CONFLICT, e))?;
    Ok(t)
}

/// Mirrors tui.rs's `open_note_external`, minus the raw-mode terminal handling that has
/// no equivalent in a headless server process — every spawn here is fire-and-forget.
/// Priority: Obsidian (if configured) > $EDITOR/$VISUAL.
fn open_note_external(note_path: &std::path::Path) -> Result<(), String> {
    if let Some(uri) = crate::note::obsidian_uri(note_path) {
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
    open_note_external(&note_path).map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn edit_note(
    State(state): State<Arc<AppState>>,
    AxumPath(slug): AxumPath<String>,
    Json(req): Json<EditNoteRequest>,
) -> Result<Json<NoteResponse>, (StatusCode, Json<serde_json::Value>)> {
    if !is_valid_slug(&slug) {
        return Err(app_error(StatusCode::BAD_REQUEST, "invalid note slug"));
    }
    let _guard = state.write_lock.lock().await;
    let dir = notes_dir(&state);
    let note_path = dir.join(format!("{}.md", slug));
    let existing = crate::note::read_note(&note_path)
        .map_err(|_| app_error(StatusCode::NOT_FOUND, format!("Note '{}' not found", slug)))?;

    let note = crate::note::Note {
        slug: slug.clone(),
        title: req.title.unwrap_or(existing.title),
        body: req.body.unwrap_or(existing.body),
    };
    let note_path = crate::note::write_note(&dir, &note)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let modified = modified_time(&note_path);
    Ok(Json(note_response(note, modified)))
}

async fn unlink_task_note(
    State(state): State<Arc<AppState>>,
    AxumPath((id, slug)): AxumPath<(u32, String)>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    if !is_valid_slug(&slug) {
        return Err(app_error(StatusCode::BAD_REQUEST, "invalid note slug"));
    }
    let _guard = state.write_lock.lock().await;
    let mut task_file = db::begin(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let t = task_file
        .find_task_mut(id)
        .ok_or_else(|| app_error(StatusCode::NOT_FOUND, format!("Task {} not found", id)))?;
    t.notes.retain(|s| s != &slug);
    task_file.commit().map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn router(state: AppState) -> Router {
    router_with_static(state, None)
}

#[derive(Serialize)]
struct VersionResponse {
    current: String,
    latest: Option<String>,
    update_available: bool,
    update_supported: bool,
}

async fn get_version(State(state): State<Arc<AppState>>) -> Json<VersionResponse> {
    let update = state.update.clone();
    let latest = tokio::task::spawn_blocking(move || update.latest_version()).await.ok().flatten();
    let current = crate::update::current_version();
    let update_available = latest
        .as_deref()
        .and_then(|l| semver::Version::parse(l).ok())
        .is_some_and(|l| crate::update::is_newer(&current, &l));
    Json(VersionResponse {
        current: current.to_string(),
        latest,
        update_available,
        update_supported: state.update.supported,
    })
}

async fn post_update(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    // A cross-site page cannot set this header without a CORS preflight, which we never allow.
    if headers.get("x-command-center").and_then(|v| v.to_str().ok()) != Some("update") {
        return Err(app_error(StatusCode::FORBIDDEN, "missing X-Command-Center header"));
    }
    if !state.update.supported {
        return Err(app_error(StatusCode::BAD_REQUEST, "updates are not supported for this build"));
    }
    if !state.update.try_begin() {
        return Err(app_error(StatusCode::CONFLICT, "an update is already running"));
    }
    let update = state.update.clone();
    let db_path = state.db_path.clone();
    std::thread::spawn(move || crate::update_state::run_update_job(&update, &db_path));
    Ok(StatusCode::ACCEPTED)
}

async fn get_update_status(State(state): State<Arc<AppState>>) -> Json<crate::update_state::StatusSnapshot> {
    Json(state.update.status())
}

fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/version", get(get_version))
        .route("/api/update", axum::routing::post(post_update))
        .route("/api/update/status", get(get_update_status))
        .route("/api/tasks", get(list_tasks).post(add_task))
        .route("/api/tasks/:id", get(get_task).patch(edit_task).delete(delete_task))
        .route("/api/services", get(list_services).post(add_service))
        .route("/api/services/:id/pause", axum::routing::post(pause_service))
        .route("/api/services/:id/resume", axum::routing::post(resume_service))
        .route("/api/services/:id/runs", get(list_service_runs))
        .route("/api/tasks/:id/done", axum::routing::post(done_task))
        .route("/api/tasks/:id/reopen", axum::routing::post(reopen_task))
        .route("/api/tasks/:id/notes", get(list_task_notes).post(create_task_note))
        .route("/api/tasks/:id/notes/:slug", axum::routing::delete(unlink_task_note))
        .route("/api/tasks/:id/thread", get(get_task_thread).post(post_task_thread))
        // Deprecated aliases for /thread, kept until nothing calls them.
        .route("/api/tasks/:id/review", get(get_task_thread_note).post(add_task_review_feedback))
        .route("/api/tasks/:id/question", get(get_task_thread_note).post(answer_task_question))
        .route("/api/notes/:slug", axum::routing::put(edit_note))
        .route("/api/notes/:slug/open", axum::routing::post(open_note))
        .route("/api/agents", get(list_agents))
        .route("/api/agents/:name/instructions", get(get_agent_instructions).put(edit_agent_instructions))
        .route("/api/agents/:name/memory", get(get_agent_memory).put(edit_agent_memory))
        .route("/api/events", get(task_events))
}

pub fn router_with_static(state: AppState, static_dir: Option<std::path::PathBuf>) -> Router {
    let mut app = api_routes();
    if let Some(dir) = static_dir {
        app = app.fallback_service(tower_http::services::ServeDir::new(dir));
    }
    app.layer(middleware::from_fn(validate_host)).with_state(Arc::new(state))
}

/// Serves the web UI embedded in the binary (read from `web/dist` in debug builds).
pub fn router_embedded(state: AppState) -> Router {
    api_routes()
        .fallback(embedded_handler)
        .layer(middleware::from_fn(validate_host))
        .with_state(Arc::new(state))
}

async fn embedded_handler(uri: axum::http::Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    match crate::assets::web_asset(path) {
        Some((bytes, mime)) => ([(axum::http::header::CONTENT_TYPE, mime)], bytes).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Runs the web server in the foreground until the process exits.
pub async fn serve(db_path: PathBuf) -> Result<(), String> {
    db::backup_daily(&db_path);
    let notes_dir = db::resolve_notes_dir(&db_path);
    let port: u16 = std::env::var("TASK_SERVER_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4287);

    println!("task serve: serving {} on http://127.0.0.1:{}", db_path.display(), port);

    let (change_tx, _) = tokio::sync::broadcast::channel(16);
    // Held for the lifetime of the process — dropping it would stop the watch.
    let _watcher = match crate::watch::spawn(db_path.clone(), change_tx.clone()) {
        Ok(w) => Some(w),
        Err(e) => {
            eprintln!("task serve: failed to watch {} for changes — live updates disabled: {}", db_path.display(), e);
            None
        }
    };

    let app = router_embedded(AppState {
        notes_dir,
        db_path,
        write_lock: Arc::new(tokio::sync::Mutex::new(())),
        change_tx,
        update: Arc::new(crate::update_state::UpdateState::from_env()),
    });
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .map_err(|e| format!("Failed to bind 127.0.0.1:{}: {}", port, e))?;
    axum::serve(listener, app).await.map_err(|e| e.to_string())
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
        let notes_dir = dir.path().join("Notes");
        (dir, AppState { db_path, notes_dir, write_lock: Arc::new(tokio::sync::Mutex::new(())), change_tx: tokio::sync::broadcast::channel(16).0, update: Arc::new(crate::update_state::UpdateState::for_tests("http://127.0.0.1:1", false)) })
    }

    /// `TASK_CONFIG_FILE` is process-global, so tests that set it must not run concurrently
    /// with each other — guard every such test with this lock for its whole duration.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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
                description: None, instructions: None,
                due_date: None,
                project: None,
                recurrence: None,
                notes: Vec::new(),
                agent: None,
                effort: None,
                work_status: None, kind: Default::default(), paused: false,
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
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None, work_status: None, kind: Default::default(), paused: false,
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
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None, work_status: None, kind: Default::default(), paused: false,
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
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tasks.db");
        let config_path = dir.path().join("config.md");
        std::fs::write(&config_path, "agent-bot: /code/bot\n").unwrap();
        unsafe { std::env::set_var("TASK_CONFIG_FILE", &config_path) };
        let app = router(AppState { notes_dir: dir.path().join("Notes"), db_path, write_lock: Arc::new(tokio::sync::Mutex::new(())), change_tx: tokio::sync::broadcast::channel(16).0, update: Arc::new(crate::update_state::UpdateState::for_tests("http://127.0.0.1:1", false)) });
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
    async fn test_agent_instructions_roundtrip_via_api() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tasks.db");
        let config_path = dir.path().join("config.md");
        std::fs::write(&config_path, "agent-bot: /code/bot\n").unwrap();
        unsafe { std::env::set_var("TASK_CONFIG_FILE", &config_path) };
        let app = router(AppState { notes_dir: dir.path().join("Notes"), db_path, write_lock: Arc::new(tokio::sync::Mutex::new(())), change_tx: tokio::sync::broadcast::channel(16).0, update: Arc::new(crate::update_state::UpdateState::for_tests("http://127.0.0.1:1", false)) });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/agents/bot/instructions")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert!(json.is_null());

        let body = serde_json::json!({ "title": "Bot Instructions", "body": "Do the thing." });
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/agents/bot/instructions")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["title"], "Bot Instructions");
        assert_eq!(json["body"], "Do the thing.");

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/agents/bot/instructions")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        unsafe { std::env::remove_var("TASK_CONFIG_FILE") };
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["title"], "Bot Instructions");
        assert_eq!(json["body"], "Do the thing.");
    }

    #[tokio::test]
    async fn test_agent_memory_roundtrip_via_api() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tasks.db");
        let config_path = dir.path().join("config.md");
        std::fs::write(&config_path, "agent-bot: /code/bot\n").unwrap();
        unsafe { std::env::set_var("TASK_CONFIG_FILE", &config_path) };
        let app = router(AppState { notes_dir: dir.path().join("Notes"), db_path, write_lock: Arc::new(tokio::sync::Mutex::new(())), change_tx: tokio::sync::broadcast::channel(16).0, update: Arc::new(crate::update_state::UpdateState::for_tests("http://127.0.0.1:1", false)) });

        let body = serde_json::json!({ "body": "Remembered fact." });
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/agents/bot/memory")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["title"], "bot Memory");
        assert_eq!(json["body"], "Remembered fact.");

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/agents/bot/memory")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        unsafe { std::env::remove_var("TASK_CONFIG_FILE") };
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["body"], "Remembered fact.");
    }

    #[tokio::test]
    async fn test_agent_instructions_unknown_agent_returns_404() {
        let _guard = ENV_LOCK.lock().unwrap();
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tasks.db");
        let config_path = dir.path().join("config.md");
        std::fs::write(&config_path, "agent-bot: /code/bot\n").unwrap();
        unsafe { std::env::set_var("TASK_CONFIG_FILE", &config_path) };
        let app = router(AppState { notes_dir: dir.path().join("Notes"), db_path, write_lock: Arc::new(tokio::sync::Mutex::new(())), change_tx: tokio::sync::broadcast::channel(16).0, update: Arc::new(crate::update_state::UpdateState::for_tests("http://127.0.0.1:1", false)) });
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/agents/ghost/instructions")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        unsafe { std::env::remove_var("TASK_CONFIG_FILE") };
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_internal_error_returns_generic_message() {
        let dir = tempdir().unwrap();
        // A directory can't be opened as a sqlite file, so db::load fails here,
        // giving us a real internal error whose raw message must not leak to the client.
        let state = AppState {
            db_path: dir.path().to_path_buf(),
            notes_dir: dir.path().join("Notes"),
            write_lock: Arc::new(tokio::sync::Mutex::new(())),
            change_tx: tokio::sync::broadcast::channel(16).0,
            update: Arc::new(crate::update_state::UpdateState::for_tests("http://127.0.0.1:1", false)),
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
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None, work_status: None, kind: Default::default(), paused: false,
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
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 5), project: None, recurrence: None,
                notes: Vec::new(), agent: None, effort: None, work_status: None, kind: Default::default(), paused: false,
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

    async fn patch_task_1(state: AppState, body: serde_json::Value) -> axum::response::Response {
        router(state)
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
            .unwrap()
    }

    #[tokio::test]
    async fn test_edit_service_schedule_and_agent() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Nightly sync".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: chrono::NaiveDate::from_ymd_opt(2030, 1, 1), project: None,
                recurrence: Some(crate::task::Recurrence::from_str("daily").unwrap()), notes: Vec::new(),
                agent: Some("bot".to_string()), effort: None, work_status: None,
                kind: crate::task::TaskKind::Service, paused: false,
            });
            db::save(&state.db_path, &tf).unwrap();
        }

        // What the Services screen sends when its schedule row is saved.
        let body = serde_json::json!({ "recurrence": "weekly:fri", "due": "2030-01-04", "agent": "other" });
        let response = patch_task_1(state.clone(), body).await;
        assert_eq!(response.status(), StatusCode::OK);
        let t = db::load(&state.db_path).unwrap().find_task(1).unwrap().clone();
        assert!(t.is_service());
        assert_eq!(t.recurrence.unwrap().to_string(), "weekly:fri");
        assert_eq!(t.due_date, chrono::NaiveDate::from_ymd_opt(2030, 1, 4));
        assert_eq!(t.agent.as_deref(), Some("other"));

        // An empty agent unassigns the service.
        let response = patch_task_1(state.clone(), serde_json::json!({ "agent": "" })).await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(db::load(&state.db_path).unwrap().find_task(1).unwrap().agent, None);
    }

    #[tokio::test]
    async fn test_edit_task_kind_converts_to_service_and_back() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Nightly sync".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: Some("bot".to_string()),
                effort: None, work_status: Some(crate::task::WorkStatus::WaitingForReview), kind: Default::default(), paused: false,
            });
            db::save(&state.db_path, &tf).unwrap();
        }

        // Without a schedule the conversion is refused and nothing changes.
        let response = patch_task_1(state.clone(), serde_json::json!({ "kind": "service" })).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(!db::load(&state.db_path).unwrap().find_task(1).unwrap().is_service());

        let body = serde_json::json!({ "kind": "service", "recurrence": "daily", "due": "2030-01-02", "work_status": "todo" });
        let response = patch_task_1(state.clone(), body).await;
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["kind"], "service");
        assert_eq!(json["recurrence"], "daily");
        assert_eq!(json["due_date"], "2030-01-02");
        // A new service always starts scheduled; services never use todo.
        assert!(json["work_status"].is_null());

        // Turning it back into a task clears paused and failing.
        {
            let mut tf = db::load(&state.db_path).unwrap();
            let t = tf.find_task_mut(1).unwrap();
            t.paused = true;
            t.work_status = Some(crate::task::WorkStatus::Failing);
            db::save(&state.db_path, &tf).unwrap();
        }
        let response = patch_task_1(state.clone(), serde_json::json!({ "kind": "task" })).await;
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["kind"], "task");
        assert_eq!(json["paused"], false);
        assert_eq!(json["work_status"], "todo");

        let response = patch_task_1(state, serde_json::json!({ "kind": "bogus" })).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_edit_task_empty_agent_clears_it() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Has an agent".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(),
                agent: Some("bot".to_string()), effort: None, work_status: None, kind: Default::default(), paused: false,
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
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None,
                work_status: None, kind: Default::default(), paused: false,
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

    fn days_from_today(days: i64) -> chrono::NaiveDate {
        chrono::Local::now().date_naive() + chrono::Duration::days(days)
    }

    /// Seeds one waiting-for-review task assigned to "bot", due 10 days from today, then PATCHes it.
    async fn patch_review_task(body: serde_json::Value) -> serde_json::Value {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "In review".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: Some(days_from_today(10)), project: None, recurrence: None,
                notes: Vec::new(), agent: Some("bot".to_string()), effort: None,
                work_status: Some(crate::task::WorkStatus::WaitingForReview), kind: Default::default(), paused: false,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let response = router(state)
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
        body_json(response).await
    }

    #[tokio::test]
    async fn test_assigning_agent_to_review_task_moves_it_to_todo() {
        let json = patch_review_task(serde_json::json!({ "agent": "other-bot" })).await;
        assert_eq!(json["agent"], "other-bot");
        assert_eq!(json["work_status"], "todo");
    }

    #[tokio::test]
    async fn test_assigning_human_to_review_task_keeps_status() {
        let json = patch_review_task(serde_json::json!({ "agent": "human" })).await;
        assert_eq!(json["work_status"], "waiting-for-review");
    }

    #[tokio::test]
    async fn test_explicit_work_status_wins_over_review_handoff() {
        let due = days_from_today(20).format("%Y-%m-%d").to_string();
        let json = patch_review_task(serde_json::json!({ "due": due, "work_status": "todo" })).await;
        assert_eq!(json["work_status"], "todo");
        assert_eq!(json["agent"], "bot");
    }

    #[tokio::test]
    async fn test_deferring_review_task_hands_it_to_human() {
        let later = days_from_today(20).format("%Y-%m-%d").to_string();
        let json = patch_review_task(serde_json::json!({ "due": later })).await;
        assert_eq!(json["due_date"], later);
        assert_eq!(json["agent"], "human");
        assert!(json["work_status"].is_null());
    }

    #[tokio::test]
    async fn test_pulling_review_task_earlier_is_not_a_defer() {
        let earlier = days_from_today(5).format("%Y-%m-%d").to_string();
        let json = patch_review_task(serde_json::json!({ "due": earlier })).await;
        assert_eq!(json["agent"], "bot");
        assert_eq!(json["work_status"], "waiting-for-review");
    }

    #[tokio::test]
    async fn test_edit_task_past_due_date_returns_400() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "T".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: None, project: None, recurrence: None,
                notes: Vec::new(), agent: None, effort: None, work_status: None, kind: Default::default(), paused: false,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let past = days_from_today(-1).format("%Y-%m-%d").to_string();
        let body = serde_json::json!({ "due": past });
        let response = router(state)
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
    async fn test_api_refuses_a_human_setting_waiting_for_review() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "No project here".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None,
                work_status: None, kind: Default::default(), paused: false,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state.clone());
        let body = serde_json::json!({ "work_status": "waiting-for-review" });
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
        let json = body_json(response).await;
        assert!(json["error"].as_str().unwrap().contains("set by the agent"));
        assert_eq!(db::load(&state.db_path).unwrap().tasks[0].work_status, None);
    }

    #[tokio::test]
    async fn test_edit_task_invalid_work_status_returns_400() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Track me".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None,
                work_status: None, kind: Default::default(), paused: false,
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
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None,
                work_status: Some(crate::task::WorkStatus::WaitingForReview), kind: Default::default(), paused: false,
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
        // An explicit clear is a reset, which leaves the task todo.
        assert_eq!(json["work_status"], "todo");
    }

    #[tokio::test]
    async fn test_edit_task_sets_recurrence() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Recur me".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None,
                work_status: None, kind: Default::default(), paused: false,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let body = serde_json::json!({ "recurrence": "weekly:fri" });
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
        assert_eq!(json["recurrence"], "weekly:fri");
    }

    #[tokio::test]
    async fn test_edit_task_invalid_recurrence_returns_400() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Recur me".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None,
                work_status: None, kind: Default::default(), paused: false,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let body = serde_json::json!({ "recurrence": "bogus" });
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
    async fn test_edit_task_empty_recurrence_clears_it() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Recur me".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: None, project: None,
                recurrence: Some(crate::task::Recurrence::from_str("weekly").unwrap()),
                notes: Vec::new(), agent: None, effort: None, work_status: None, kind: Default::default(), paused: false,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let body = serde_json::json!({ "recurrence": "" });
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
        assert!(json["recurrence"].is_null());
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
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None, work_status: None, kind: Default::default(), paused: false,
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
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 5), project: None,
                recurrence: Some(crate::task::Recurrence::from_str("weekly").unwrap()),
                notes: Vec::new(), agent: None, effort: None, work_status: None, kind: Default::default(), paused: false,
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
    async fn test_done_task_recurring_spawn_keeps_same_agent() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Recurring".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 5), project: None,
                recurrence: Some(crate::task::Recurrence::from_str("weekly").unwrap()),
                notes: Vec::new(), agent: Some("bot".to_string()), effort: None, work_status: None, kind: Default::default(), paused: false,
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
        assert_eq!(json["spawned"]["agent"], "bot");
    }

    #[tokio::test]
    async fn test_done_task_already_done_does_not_spawn_duplicate() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Recurring".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 5), project: None,
                recurrence: Some(crate::task::Recurrence::from_str("weekly").unwrap()),
                notes: Vec::new(), agent: None, effort: None, work_status: None, kind: Default::default(), paused: false,
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
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None, work_status: None, kind: Default::default(), paused: false,
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
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None, work_status: None, kind: Default::default(), paused: false,
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
    async fn test_list_task_notes_orders_newest_file_first() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Has notes".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: None, project: None, recurrence: None,
                notes: vec!["oldest".to_string(), "newest".to_string(), "middle".to_string()],
                agent: None, effort: None, work_status: None, kind: Default::default(), paused: false,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let notes_dir = notes_dir(&state);
        std::fs::create_dir_all(&notes_dir).unwrap();
        // Write in a different order than the link order above, and set
        // explicit mtimes so the test doesn't depend on write timing.
        for (slug, offset_secs) in [("middle", 100), ("newest", 200), ("oldest", 0)] {
            let path = notes_dir.join(format!("{}.md", slug));
            std::fs::write(&path, format!("# {}\n", slug)).unwrap();
            let mtime = filetime::FileTime::from_unix_time(1_700_000_000 + offset_secs, 0);
            filetime::set_file_mtime(&path, mtime).unwrap();
        }

        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tasks/1/notes")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        let slugs: Vec<&str> = json.as_array().unwrap().iter().map(|n| n["slug"].as_str().unwrap()).collect();
        assert_eq!(slugs, vec!["newest", "middle", "oldest"]);
    }

    #[tokio::test]
    async fn test_task_notes_read_from_configured_notes_dir() {
        let (dir, mut state) = make_state();
        let custom = dir.path().join("vault").join("Notes");
        std::fs::create_dir_all(&custom).unwrap();
        std::fs::write(custom.join("linked.md"), "# Linked\n").unwrap();
        state.notes_dir = custom;
        let mut tf = crate::task::TaskFile::new();
        tf.tasks.push(crate::task::Task {
            id: 1, title: "Has note".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
            tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
            due_date: None, project: None, recurrence: None,
            notes: vec!["linked".to_string()],
            agent: None, effort: None, work_status: None, kind: Default::default(), paused: false,
        });
        db::save(&state.db_path, &tf).unwrap();

        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tasks/1/notes")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        let slugs: Vec<&str> = json.as_array().unwrap().iter().map(|n| n["slug"].as_str().unwrap()).collect();
        assert_eq!(slugs, vec!["linked"]);
    }

    #[tokio::test]
    async fn test_edit_note_updates_body_and_keeps_title() {
        let (_dir, state) = make_state();
        std::fs::create_dir_all(&state.notes_dir).unwrap();
        std::fs::write(state.notes_dir.join("n.md"), "# Original\n\nold body\n").unwrap();
        let notes_dir = state.notes_dir.clone();
        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/notes/n")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"body":"new body"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["title"], "Original");
        assert_eq!(json["body"], "new body");
        let on_disk = std::fs::read_to_string(notes_dir.join("n.md")).unwrap();
        assert_eq!(on_disk, "# Original\n\nnew body\n");
    }

    #[tokio::test]
    async fn test_edit_note_missing_note_is_404() {
        let (_dir, state) = make_state();
        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/notes/ghost")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"body":"x"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_edit_note_rejects_path_traversal_slug() {
        let (_dir, state) = make_state();
        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/notes/..%2f..%2fetc%2fpasswd")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"body":"x"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
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
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: None, project: None, recurrence: None, notes: vec!["real-note".to_string()], agent: None, effort: None, work_status: None, kind: Default::default(), paused: false,
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
    async fn test_get_task_review_returns_null_when_no_thread_exists() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Has no thread yet".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None, work_status: None, kind: Default::default(), paused: false,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tasks/1/review")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert!(json.is_null());
    }

    #[tokio::test]
    async fn test_get_task_question_returns_null_when_no_question_exists() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "No question yet".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None, work_status: None, kind: Default::default(), paused: false,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tasks/1/question")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert!(json.is_null());
    }

    #[tokio::test]
    async fn test_get_task_question_missing_task_returns_404() {
        let (_dir, state) = make_state();
        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tasks/999/question")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_task_question_returns_note_when_it_exists() {
        let (dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Needs input".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: None, project: None, recurrence: None, notes: vec!["task-1-question".to_string()],
                agent: Some("bot".to_string()), effort: None, work_status: Some(crate::task::WorkStatus::NeedsInput), kind: Default::default(), paused: false,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let notes_dir = dir.path().join("Notes");
        crate::note::write_note(
            &notes_dir,
            &crate::note::Note {
                slug: "task-1-question".to_string(),
                title: "Question — Needs input".to_string(),
                body: "Which environment should this target, staging or prod?".to_string(),
            },
        )
        .unwrap();
        let app = router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tasks/1/question")
                    .header("host", "127.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["slug"], "task-1-thread");
        assert!(json["body"].as_str().unwrap().contains("Which environment should this target, staging or prod?"));
        // Reading folds the old note into the thread and keeps the original under .migrated.
        assert!(!notes_dir.join("task-1-question.md").exists());
        assert!(notes_dir.join(".migrated/task-1-question.md").is_file());
    }

    fn needs_input_task() -> crate::task::Task {
        crate::task::Task {
            id: 1, title: "Blocked".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
            tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
            due_date: None, project: None, recurrence: None, notes: vec!["task-1-question".to_string()],
            agent: Some("bot".to_string()), effort: None, work_status: Some(crate::task::WorkStatus::NeedsInput), kind: Default::default(), paused: false,
        }
    }

    fn post_question_answer(text: &str) -> Request<Body> {
        Request::builder()
            .method("POST")
            .uri("/api/tasks/1/question")
            .header("host", "127.0.0.1")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&serde_json::json!({ "text": text })).unwrap()))
            .unwrap()
    }

    #[tokio::test]
    async fn test_post_task_question_appends_answer_and_sets_changes_requested() {
        let (_dir, state) = make_state();
        let mut tf = crate::task::TaskFile::new();
        tf.tasks.push(needs_input_task());
        db::save(&state.db_path, &tf).unwrap();
        crate::note::write_note(
            &notes_dir(&state),
            &crate::note::Note {
                slug: "task-1-question".to_string(),
                title: "Question — Blocked".to_string(),
                body: "Which branch should I use?".to_string(),
            },
        )
        .unwrap();

        let response = router(state.clone()).oneshot(post_question_answer("Use main.")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["slug"], "task-1-thread");
        let body = json["body"].as_str().unwrap();
        assert!(body.contains("Which branch should I use?"));
        assert!(body.contains("## Answer —"));
        assert!(body.contains("Use main."));

        let tf = db::load(&state.db_path).unwrap();
        assert_eq!(tf.find_task(1).unwrap().work_status, Some(crate::task::WorkStatus::ChangesRequested));
    }

    #[tokio::test]
    async fn test_post_task_question_empty_text_returns_400() {
        let (_dir, state) = make_state();
        let mut tf = crate::task::TaskFile::new();
        tf.tasks.push(needs_input_task());
        db::save(&state.db_path, &tf).unwrap();

        let response = router(state.clone()).oneshot(post_question_answer("   ")).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let tf = db::load(&state.db_path).unwrap();
        assert_eq!(tf.find_task(1).unwrap().work_status, Some(crate::task::WorkStatus::NeedsInput));
    }

    #[tokio::test]
    async fn test_post_task_question_missing_task_returns_404() {
        let (_dir, state) = make_state();
        let response = router(state).oneshot(post_question_answer("hi")).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_post_task_review_creates_thread_and_sets_changes_requested() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Waiting on review".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: Some("bot".to_string()),
                effort: None, work_status: Some(crate::task::WorkStatus::WaitingForReview), kind: Default::default(), paused: false,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state.clone());
        let body = serde_json::json!({ "text": "Please fix the error handling." });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks/1/review")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["slug"], "task-1-thread");
        assert!(json["body"].as_str().unwrap().contains("## Feedback —"));
        assert!(json["body"].as_str().unwrap().contains("Please fix the error handling."));

        let tf = db::load(&state.db_path).unwrap();
        let task = tf.find_task(1).unwrap();
        assert_eq!(task.work_status, Some(crate::task::WorkStatus::ChangesRequested));
    }

    #[tokio::test]
    async fn test_post_task_review_appends_to_existing_thread() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "Multi-round".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None, work_status: Some(crate::task::WorkStatus::WaitingForReview), kind: Default::default(), paused: false,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state.clone());
        let body1 = serde_json::json!({ "text": "First round of feedback." });
        app.clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks/1/review")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body1).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body2 = serde_json::json!({ "text": "Second round of feedback." });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks/1/review")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body2).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let json = body_json(response).await;
        let full_body = json["body"].as_str().unwrap();
        assert!(full_body.contains("First round of feedback."));
        assert!(full_body.contains("Second round of feedback."));
        // First round's text must appear before the second's.
        let first_pos = full_body.find("First round of feedback.").unwrap();
        let second_pos = full_body.find("Second round of feedback.").unwrap();
        assert!(first_pos < second_pos);
    }

    #[tokio::test]
    async fn test_post_task_review_empty_text_returns_400() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "T".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None, work_status: None, kind: Default::default(), paused: false,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state);
        let body = serde_json::json!({ "text": "   " });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks/1/review")
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
    async fn test_post_task_review_refused_on_a_todo_task_without_writing_a_note() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            let mut t = sample_task_with_note();
            t.work_status = None;
            tf.tasks.push(t);
            db::save(&state.db_path, &tf).unwrap();
        }
        let response = router(state.clone())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/tasks/1/review")
                    .header("host", "127.0.0.1")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&serde_json::json!({ "text": "hi" })).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert!(!state.db_path.parent().unwrap().join("Notes").join("task-1-thread.md").exists());
        assert_eq!(db::load(&state.db_path).unwrap().tasks[0].work_status, None);
    }

    fn sample_task_with_note() -> crate::task::Task {
        crate::task::Task {
            id: 1, title: "T".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
            tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
            due_date: None, project: None, recurrence: None, notes: vec!["result".to_string()], agent: Some("bot".to_string()),
            effort: None, work_status: Some(crate::task::WorkStatus::WaitingForReview), kind: Default::default(), paused: false,
        }
    }

    #[tokio::test]
    async fn test_task_api_reports_state_and_allowed_actions() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(sample_task_with_note());
            db::save(&state.db_path, &tf).unwrap();
        }
        let response = router(state)
            .oneshot(Request::builder().uri("/api/tasks/1").header("host", "127.0.0.1").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let json = body_json(response).await;
        assert_eq!(json["state"], "waiting-for-review");
        assert_eq!(json["allowed_actions"], serde_json::json!(["feedback", "done", "reset", "defer"]));
    }

    #[tokio::test]
    async fn test_post_task_review_links_note_to_task_without_duplicating() {
        let (_dir, state) = make_state();
        {
            let mut tf = crate::task::TaskFile::new();
            tf.tasks.push(crate::task::Task {
                id: 1, title: "T".to_string(), status: Status::Open, priority: crate::task::Priority::Medium,
                tags: Vec::new(), created: chrono::Utc::now(), updated: None, description: None, instructions: None,
                due_date: None, project: None, recurrence: None, notes: Vec::new(), agent: None, effort: None, work_status: Some(crate::task::WorkStatus::WaitingForReview), kind: Default::default(), paused: false,
            });
            db::save(&state.db_path, &tf).unwrap();
        }
        let app = router(state.clone());
        for text in ["Round one.", "Round two."] {
            let body = serde_json::json!({ "text": text });
            app.clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/tasks/1/review")
                        .header("host", "127.0.0.1")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&body).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();
        }
        let tf = db::load(&state.db_path).unwrap();
        let task = tf.find_task(1).unwrap();
        let occurrences = task.notes.iter().filter(|s| s.as_str() == "task-1-thread").count();
        assert_eq!(occurrences, 1, "the thread slug must be linked exactly once, not duplicated on the second post");
    }

    fn thread_request(method: &str, kind: Option<&str>, text: &str) -> Request<Body> {
        let b = Request::builder().method(method).uri("/api/tasks/1/thread").header("host", "127.0.0.1");
        match kind {
            Some(k) => b
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&serde_json::json!({ "kind": k, "text": text })).unwrap()))
                .unwrap(),
            None => b.body(Body::empty()).unwrap(),
        }
    }

    fn seed(state: &AppState, ws: Option<crate::task::WorkStatus>, agent: Option<&str>) {
        let mut tf = crate::task::TaskFile::new();
        let mut t = sample_task_with_note();
        t.work_status = ws;
        t.agent = agent.map(String::from);
        tf.tasks.push(t);
        db::save(&state.db_path, &tf).unwrap();
    }

    #[tokio::test]
    async fn test_thread_get_reports_allowed_action_and_unread() {
        let (_dir, state) = make_state();
        seed(&state, Some(crate::task::WorkStatus::WaitingForReview), Some("bot"));
        let json = body_json(router(state.clone()).oneshot(thread_request("GET", None, "")).await.unwrap()).await;
        assert!(json["note"].is_null());
        assert_eq!(json["allowed_action"], serde_json::json!({ "send": "feedback", "approve": true }));
        assert_eq!(json["unread"], false);

        let r = router(state).oneshot(thread_request("POST", Some("feedback"), "tighten it")).await.unwrap();
        let json = body_json(r).await;
        assert_eq!(json["unread"], true);
        assert_eq!(json["allowed_action"]["send"], "message");
        assert!(json["note"]["body"].as_str().unwrap().contains("## Feedback —"));
    }

    #[tokio::test]
    async fn test_thread_message_while_working_keeps_the_state() {
        let (_dir, state) = make_state();
        seed(&state, Some(crate::task::WorkStatus::InProgress), Some("bot"));
        let r = router(state.clone()).oneshot(thread_request("POST", Some("message"), "also handle empty input")).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        let tf = db::load(&state.db_path).unwrap();
        let t = tf.find_task(1).unwrap();
        assert_eq!(t.work_status, Some(crate::task::WorkStatus::InProgress));
        assert!(t.notes.contains(&"task-1-thread".to_string()));
    }

    #[tokio::test]
    async fn test_thread_rejects_a_kind_the_state_does_not_allow() {
        let (_dir, state) = make_state();
        seed(&state, Some(crate::task::WorkStatus::WaitingForReview), Some("bot"));
        for kind in ["answer", "message"] {
            let r = router(state.clone()).oneshot(thread_request("POST", Some(kind), "x")).await.unwrap();
            assert_eq!(r.status(), StatusCode::CONFLICT, "{}", kind);
        }
        let r = router(state.clone()).oneshot(thread_request("POST", Some("shout"), "x")).await.unwrap();
        assert_eq!(r.status(), StatusCode::BAD_REQUEST);
        let r = router(state.clone()).oneshot(thread_request("POST", Some("feedback"), "  ")).await.unwrap();
        assert_eq!(r.status(), StatusCode::BAD_REQUEST);
        assert!(!state.db_path.parent().unwrap().join("Notes").join("task-1-thread.md").exists());

        // A human-owned task has nobody to message.
        seed(&state, Some(crate::task::WorkStatus::InProgress), Some("human"));
        let r = router(state).oneshot(thread_request("POST", Some("message"), "x")).await.unwrap();
        assert_eq!(r.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_thread_approve_marks_the_task_done_and_records_it() {
        let (_dir, state) = make_state();
        seed(&state, Some(crate::task::WorkStatus::WaitingForReview), Some("bot"));
        let r = router(state.clone()).oneshot(thread_request("POST", Some("approve"), "")).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        let json = body_json(r).await;
        assert!(json["note"]["body"].as_str().unwrap().contains("## Approval —"));
        let tf = db::load(&state.db_path).unwrap();
        assert_eq!(tf.find_task(1).unwrap().status, Status::Done);

        // Approving is only offered while the work waits for review.
        seed(&state, Some(crate::task::WorkStatus::InProgress), Some("bot"));
        let r = router(state).oneshot(thread_request("POST", Some("approve"), "")).await.unwrap();
        assert_eq!(r.status(), StatusCode::CONFLICT);
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

    #[tokio::test]
    async fn test_embedded_router_unknown_asset_is_404_and_api_still_works() {
        let (_dir, state) = make_state();
        let app = router_embedded(state);
        let missing = app.clone()
            .oneshot(Request::builder().uri("/no-such-file.js").header("host", "127.0.0.1").body(Body::empty()).unwrap())
            .await.unwrap();
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);
        let api = app
            .oneshot(Request::builder().uri("/api/tasks").header("host", "127.0.0.1").body(Body::empty()).unwrap())
            .await.unwrap();
        assert_eq!(api.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_embedded_router_still_rejects_spoofed_host() {
        let (_dir, state) = make_state();
        let response = router_embedded(state)
            .oneshot(Request::builder().uri("/").header("host", "evil.com").body(Body::empty()).unwrap())
            .await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    fn get_req(uri: &str) -> Request<Body> {
        Request::builder().uri(uri).header("host", "127.0.0.1").body(Body::empty()).unwrap()
    }

    fn post_update_req(with_header: bool) -> Request<Body> {
        let mut b = Request::builder().method("POST").uri("/api/update").header("host", "127.0.0.1");
        if with_header {
            b = b.header("x-command-center", "update");
        }
        b.body(Body::empty()).unwrap()
    }

    #[tokio::test]
    async fn test_version_reports_newer_release_but_unsupported_in_test_builds() {
        let (_dir, mut state) = make_state();
        let mut server = mockito::Server::new_async().await;
        server.mock("GET", "/releases/latest").with_status(200)
            .with_body(r#"{"tag_name":"v99.0.0","assets":[]}"#).create_async().await;
        state.update = Arc::new(crate::update_state::UpdateState::for_tests(&server.url(), false));
        let response = router(state).oneshot(get_req("/api/version")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert_eq!(json["current"], env!("CARGO_PKG_VERSION"));
        assert_eq!(json["latest"], "99.0.0");
        assert_eq!(json["update_available"], true);
        assert_eq!(json["update_supported"], false);
    }

    #[tokio::test]
    async fn test_version_survives_rate_limited_release_api() {
        let (_dir, mut state) = make_state();
        let mut server = mockito::Server::new_async().await;
        server.mock("GET", "/releases/latest").with_status(403).create_async().await;
        state.update = Arc::new(crate::update_state::UpdateState::for_tests(&server.url(), true));
        let response = router(state).oneshot(get_req("/api/version")).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let json = body_json(response).await;
        assert!(json["latest"].is_null());
        assert_eq!(json["update_available"], false);
    }

    #[tokio::test]
    async fn test_post_update_without_custom_header_is_403() {
        let (_dir, mut state) = make_state();
        state.update = Arc::new(crate::update_state::UpdateState::for_tests("http://127.0.0.1:1", true));
        let update = state.update.clone();
        let response = router(state).oneshot(post_update_req(false)).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(update.status().phase, crate::update::Phase::Idle);
    }

    #[tokio::test]
    async fn test_post_update_unsupported_build_is_400() {
        let (_dir, state) = make_state(); // for_tests(.., supported = false)
        let response = router(state).oneshot(post_update_req(true)).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_post_update_while_running_is_409() {
        let (_dir, mut state) = make_state();
        state.update = Arc::new(crate::update_state::UpdateState::for_tests("http://127.0.0.1:1", true));
        assert!(state.update.try_begin()); // an update is "already running"
        let response = router(state).oneshot(post_update_req(true)).await.unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_update_status_reports_phase_and_typed_error() {
        let (_dir, state) = make_state();
        state.update.fail(&crate::update::UpdateError::NoPlatformAsset("x".into()));
        let response = router(state).oneshot(get_req("/api/update/status")).await.unwrap();
        let json = body_json(response).await;
        assert_eq!(json["phase"], "failed");
        assert_eq!(json["error"]["kind"], "no_platform_asset");
    }
}
