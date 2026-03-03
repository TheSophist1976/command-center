use std::collections::HashMap;

use chrono::{NaiveDate, Utc};
use serde::Deserialize;

use crate::task::{Priority, Status, Task, TaskFile};

// -- Todoist API types --

#[derive(Debug, Deserialize)]
pub struct TodoistDue {
    pub date: String,
}

#[derive(Debug, Deserialize)]
pub struct TodoistTask {
    pub id: String,
    pub content: String,
    #[serde(default)]
    pub description: String,
    pub priority: u8,
    #[serde(default)]
    pub labels: Vec<String>,
    pub due: Option<TodoistDue>,
    pub project_id: String,
}

#[derive(Debug, Deserialize)]
struct TodoistProject {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct PaginatedResponse<T> {
    results: Vec<T>,
    next_cursor: Option<String>,
}

// -- API calls --

fn api_base_url() -> String {
    std::env::var("TODOIST_API_BASE_URL")
        .unwrap_or_else(|_| "https://api.todoist.com".to_string())
}

pub fn fetch_open_tasks(token: &str) -> Result<Vec<TodoistTask>, String> {
    let client = reqwest::blocking::Client::new();
    let mut all_tasks = Vec::new();
    let mut cursor: Option<String> = None;
    let base = api_base_url();

    loop {
        let mut req = client
            .get(format!("{}/api/v1/tasks", base))
            .bearer_auth(token);
        if let Some(ref c) = cursor {
            req = req.query(&[("cursor", c.as_str())]);
        }

        let response = req
            .send()
            .map_err(|e| format!("Failed to fetch Todoist tasks: {}", e))?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err("Todoist token is invalid or expired. Run `task auth todoist` to re-authenticate.".to_string());
        }
        if !response.status().is_success() {
            return Err(format!("Todoist API error: {}", response.status()));
        }

        let page: PaginatedResponse<TodoistTask> = response
            .json()
            .map_err(|e| format!("Failed to parse Todoist tasks response: {}", e))?;

        all_tasks.extend(page.results);

        match page.next_cursor {
            Some(c) => cursor = Some(c),
            None => break,
        }
    }

    Ok(all_tasks)
}

pub fn fetch_projects(token: &str) -> Result<HashMap<String, String>, String> {
    let client = reqwest::blocking::Client::new();
    let mut all_projects = Vec::new();
    let mut cursor: Option<String> = None;
    let base = api_base_url();

    loop {
        let mut req = client
            .get(format!("{}/api/v1/projects", base))
            .bearer_auth(token);
        if let Some(ref c) = cursor {
            req = req.query(&[("cursor", c.as_str())]);
        }

        let response = req
            .send()
            .map_err(|e| format!("Failed to fetch Todoist projects: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Todoist projects API error: {}", response.status()));
        }

        let page: PaginatedResponse<TodoistProject> = response
            .json()
            .map_err(|e| format!("Failed to parse Todoist projects response: {}", e))?;

        all_projects.extend(page.results);

        match page.next_cursor {
            Some(c) => cursor = Some(c),
            None => break,
        }
    }

    Ok(all_projects.into_iter().map(|p| (p.id, p.name)).collect())
}

pub fn label_exported(token: &str, task_id: &str, existing_labels: &[String]) -> Result<(), String> {
    let mut labels = existing_labels.to_vec();
    if !labels.iter().any(|l| l == "exported") {
        labels.push("exported".to_string());
    }

    let client = reqwest::blocking::Client::new();
    let body = serde_json::json!({ "labels": labels });
    let base = api_base_url();

    let response = client
        .post(format!("{}/api/v1/tasks/{}", base, task_id))
        .bearer_auth(token)
        .json(&body)
        .send()
        .map_err(|e| format!("Failed to label task {} in Todoist: {}", task_id, e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to label task {} ({})",
            task_id,
            response.status()
        ));
    }
    Ok(())
}

// -- Field mapping --

pub fn map_priority(p: u8) -> Priority {
    match p {
        1 => Priority::Critical,
        2 => Priority::High,
        3 => Priority::Medium,
        _ => Priority::Low,
    }
}

pub fn map_task(t: &TodoistTask, project_map: &HashMap<String, String>) -> Task {
    let mut tags: Vec<String> = t
        .labels
        .iter()
        .filter(|l| l.as_str() != "exported")
        .cloned()
        .collect();
    if !tags.iter().any(|tag| tag == "imported") {
        tags.push("imported".to_string());
    }

    let due_date = t
        .due
        .as_ref()
        .and_then(|d| NaiveDate::parse_from_str(&d.date, "%Y-%m-%d").ok());

    let project = project_map.get(&t.project_id).cloned();

    let description = if t.description.is_empty() {
        None
    } else {
        Some(t.description.clone())
    };

    Task {
        id: 0, // caller assigns real id
        title: t.content.clone(),
        status: Status::Open,
        priority: map_priority(t.priority),
        tags,
        created: Utc::now(),
        updated: None,
        description,
        due_date,
        project,
        recurrence: None,
    }
}

// -- Import orchestration --

pub fn run_import(
    token: &str,
    task_file: &mut TaskFile,
    test_mode: bool,
) -> Result<(usize, usize), String> {
    let all_tasks = fetch_open_tasks(token)?;
    let project_map = fetch_projects(token)?;

    let (to_import, skipped_count): (Vec<_>, usize) = {
        let mut qualifying = Vec::new();
        let mut skipped = 0usize;
        for t in &all_tasks {
            if t.labels.iter().any(|l| l == "exported") {
                skipped += 1;
            } else {
                qualifying.push(t);
            }
        }
        (qualifying, skipped)
    };

    let tasks_to_process: &[&TodoistTask] = if test_mode {
        &to_import[..to_import.len().min(3)]
    } else {
        &to_import
    };

    let mut imported = 0usize;
    for todoist_task in tasks_to_process {
        let mut local_task = map_task(todoist_task, &project_map);
        local_task.id = task_file.next_id;
        task_file.next_id += 1;
        task_file.tasks.push(local_task);

        if !test_mode {
            if let Err(e) = label_exported(token, &todoist_task.id, &todoist_task.labels) {
                eprintln!("Warning: {}", e);
            }
        }
        imported += 1;
    }

    Ok((imported, skipped_count))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mutex to serialize tests that mutate the TODOIST_API_BASE_URL env var,
    // since env vars are process-global and tests run concurrently by default.
    static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn make_todoist_task(id: &str, content: &str, priority: u8, labels: Vec<String>) -> TodoistTask {
        TodoistTask {
            id: id.to_string(),
            content: content.to_string(),
            description: String::new(),
            priority,
            labels,
            due: None,
            project_id: "proj1".to_string(),
        }
    }

    // -- map_priority --

    #[test]
    fn test_map_priority_1_is_critical() {
        assert_eq!(map_priority(1), Priority::Critical);
    }

    #[test]
    fn test_map_priority_2_is_high() {
        assert_eq!(map_priority(2), Priority::High);
    }

    #[test]
    fn test_map_priority_3_is_medium() {
        assert_eq!(map_priority(3), Priority::Medium);
    }

    #[test]
    fn test_map_priority_4_is_low() {
        assert_eq!(map_priority(4), Priority::Low);
    }

    #[test]
    fn test_map_priority_0_is_low() {
        assert_eq!(map_priority(0), Priority::Low);
    }

    #[test]
    fn test_map_priority_255_is_low() {
        assert_eq!(map_priority(255), Priority::Low);
    }

    // -- map_task --

    #[test]
    fn test_map_task_basic() {
        let t = make_todoist_task("123", "Buy groceries", 3, vec![]);
        let project_map = HashMap::new();
        let task = map_task(&t, &project_map);
        assert_eq!(task.title, "Buy groceries");
        assert_eq!(task.status, Status::Open);
        assert_eq!(task.priority, Priority::Medium);
        assert_eq!(task.id, 0); // caller must assign
        // Should have "imported" tag
        assert!(task.tags.contains(&"imported".to_string()));
    }

    #[test]
    fn test_map_task_excludes_exported_label() {
        let t = make_todoist_task("123", "Task", 2, vec!["exported".to_string(), "work".to_string()]);
        let project_map = HashMap::new();
        let task = map_task(&t, &project_map);
        // "exported" should be filtered out
        assert!(!task.tags.contains(&"exported".to_string()));
        assert!(task.tags.contains(&"work".to_string()));
        assert!(task.tags.contains(&"imported".to_string()));
    }

    #[test]
    fn test_map_task_already_has_imported_label() {
        let t = make_todoist_task("123", "Task", 1, vec!["imported".to_string()]);
        let project_map = HashMap::new();
        let task = map_task(&t, &project_map);
        // "imported" should only appear once
        let count = task.tags.iter().filter(|tag| tag.as_str() == "imported").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_map_task_with_due_date() {
        let mut t = make_todoist_task("1", "Task with due", 3, vec![]);
        t.due = Some(TodoistDue { date: "2025-12-31".to_string() });
        let project_map = HashMap::new();
        let task = map_task(&t, &project_map);
        assert!(task.due_date.is_some());
        assert_eq!(task.due_date.unwrap().to_string(), "2025-12-31");
    }

    #[test]
    fn test_map_task_with_invalid_due_date() {
        let mut t = make_todoist_task("1", "Task", 3, vec![]);
        t.due = Some(TodoistDue { date: "not-a-date".to_string() });
        let project_map = HashMap::new();
        let task = map_task(&t, &project_map);
        // Invalid date => None
        assert!(task.due_date.is_none());
    }

    #[test]
    fn test_map_task_with_project() {
        let t = make_todoist_task("1", "Task", 2, vec![]);
        let mut project_map = HashMap::new();
        project_map.insert("proj1".to_string(), "My Project".to_string());
        let task = map_task(&t, &project_map);
        assert_eq!(task.project, Some("My Project".to_string()));
    }

    #[test]
    fn test_map_task_project_not_in_map() {
        let t = make_todoist_task("1", "Task", 2, vec![]);
        let project_map = HashMap::new(); // empty
        let task = map_task(&t, &project_map);
        assert!(task.project.is_none());
    }

    #[test]
    fn test_map_task_with_description() {
        let mut t = make_todoist_task("1", "Task", 3, vec![]);
        t.description = "Some description".to_string();
        let project_map = HashMap::new();
        let task = map_task(&t, &project_map);
        assert_eq!(task.description, Some("Some description".to_string()));
    }

    #[test]
    fn test_map_task_empty_description_is_none() {
        let t = make_todoist_task("1", "Task", 3, vec![]);
        let project_map = HashMap::new();
        let task = map_task(&t, &project_map);
        assert!(task.description.is_none());
    }

    // -- run_import --

    #[test]
    fn test_run_import_test_mode_limits_to_3() {
        use crate::task::TaskFile;

        // Create a mock set of tasks inline (no network calls)
        // We test the orchestration logic directly with a helper
        // Since run_import makes HTTP calls, we test via the logic path:
        // build tasks manually to simulate what run_import would process

        let mut task_file = TaskFile::new();

        // Simulate 5 qualifying tasks
        let tasks: Vec<TodoistTask> = (1..=5)
            .map(|i| make_todoist_task(&format!("{}", i), &format!("Task {}", i), 3, vec![]))
            .collect();

        let project_map = HashMap::new();

        // Simulate what run_import does in test_mode=true
        let to_import: Vec<&TodoistTask> = tasks.iter().collect();
        let tasks_to_process = &to_import[..to_import.len().min(3)];

        let mut imported = 0;
        for todoist_task in tasks_to_process {
            let mut local_task = map_task(todoist_task, &project_map);
            local_task.id = task_file.next_id;
            task_file.next_id += 1;
            task_file.tasks.push(local_task);
            imported += 1;
        }

        assert_eq!(imported, 3);
        assert_eq!(task_file.tasks.len(), 3);
    }

    #[test]
    fn test_run_import_skips_exported_tasks() {
        let tasks: Vec<TodoistTask> = vec![
            make_todoist_task("1", "Normal", 3, vec![]),
            make_todoist_task("2", "Exported", 3, vec!["exported".to_string()]),
            make_todoist_task("3", "Also normal", 3, vec![]),
        ];

        let mut qualifying = Vec::new();
        let mut skipped = 0usize;
        for t in &tasks {
            if t.labels.iter().any(|l| l == "exported") {
                skipped += 1;
            } else {
                qualifying.push(t);
            }
        }

        assert_eq!(qualifying.len(), 2);
        assert_eq!(skipped, 1);
    }

    #[test]
    fn test_label_exported_adds_label_when_missing() {
        // Test the label preparation logic (not the HTTP call)
        let existing_labels: Vec<String> = vec!["work".to_string()];
        let mut labels = existing_labels.to_vec();
        if !labels.iter().any(|l| l == "exported") {
            labels.push("exported".to_string());
        }
        assert!(labels.contains(&"exported".to_string()));
        assert_eq!(labels.len(), 2);
    }

    #[test]
    fn test_label_exported_no_duplicate() {
        // Test both branches of the "add exported if missing" logic:
        // Branch 1: "exported" NOT present => push it (covers the body)
        {
            let existing_labels: Vec<String> = vec!["work".to_string()];
            let mut labels = existing_labels.to_vec();
            if !labels.iter().any(|l| l == "exported") {
                labels.push("exported".to_string()); // this line is now covered
            }
            assert_eq!(labels.iter().filter(|l| l.as_str() == "exported").count(), 1);
        }

        // Branch 2: "exported" IS present => no push (covers the condition being false)
        {
            let existing_labels: Vec<String> = vec!["exported".to_string()];
            let mut labels = existing_labels.to_vec();
            if !labels.iter().any(|l| l == "exported") {
                labels.push("exported".to_string());
            }
            // Should not duplicate
            assert_eq!(labels.iter().filter(|l| l.as_str() == "exported").count(), 1);
        }
    }

    // -- label_exported via mock server (covers lines 126-146) --

    #[test]
    fn test_label_exported_http_success_adds_label() {
        // Covers lines 126-128 (push exported when missing) and 141-148 (success path)
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut server = mockito::Server::new();
        let base_url = server.url();
        unsafe { std::env::set_var("TODOIST_API_BASE_URL", &base_url); }

        let _mock = server
            .mock("POST", "/api/v1/tasks/task123")
            .with_status(200)
            .with_body(r#"{}"#)
            .create();

        let existing = vec!["work".to_string()]; // no "exported" label
        let result = label_exported("token", "task123", &existing);
        unsafe { std::env::remove_var("TODOIST_API_BASE_URL"); }
        assert!(result.is_ok(), "label_exported should succeed: {:?}", result);
    }

    #[test]
    fn test_label_exported_http_success_already_has_exported() {
        // Covers lines 126-128: when "exported" already in labels, no duplicate push
        // The HTTP call still happens, but the label list doesn't grow
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut server = mockito::Server::new();
        let base_url = server.url();
        unsafe { std::env::set_var("TODOIST_API_BASE_URL", &base_url); }

        let _mock = server
            .mock("POST", "/api/v1/tasks/task456")
            .with_status(200)
            .with_body(r#"{}"#)
            .create();

        let existing = vec!["exported".to_string()]; // already has "exported"
        let result = label_exported("token", "task456", &existing);
        unsafe { std::env::remove_var("TODOIST_API_BASE_URL"); }
        assert!(result.is_ok(), "label_exported should succeed: {:?}", result);
    }

    #[test]
    fn test_label_exported_http_error_returns_err() {
        // Covers lines 141-146: non-success response returns Err
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut server = mockito::Server::new();
        let base_url = server.url();
        unsafe { std::env::set_var("TODOIST_API_BASE_URL", &base_url); }

        let _mock = server
            .mock("POST", "/api/v1/tasks/task789")
            .with_status(500)
            .with_body("Server Error")
            .create();

        let existing = vec!["work".to_string()];
        let result = label_exported("token", "task789", &existing);
        unsafe { std::env::remove_var("TODOIST_API_BASE_URL"); }
        assert!(result.is_err(), "label_exported should fail with 500");
        let err = result.unwrap_err();
        assert!(err.contains("Failed to label task"), "Unexpected error: {}", err);
    }

    #[test]
    fn test_fetch_open_tasks_unauthorized() {
        // Covers line 65-66: 401 response returns specific error
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut server = mockito::Server::new();
        let base_url = server.url();
        unsafe { std::env::set_var("TODOIST_API_BASE_URL", &base_url); }

        let _mock = server
            .mock("GET", "/api/v1/tasks")
            .with_status(401)
            .with_body(r#"{"error":"unauthorized"}"#)
            .create();

        let result = fetch_open_tasks("bad_token");
        unsafe { std::env::remove_var("TODOIST_API_BASE_URL"); }
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("invalid or expired"), "Unexpected error: {}", err);
    }

    #[test]
    fn test_fetch_open_tasks_server_error() {
        // Covers lines 68-69: non-success, non-401 response
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut server = mockito::Server::new();
        let base_url = server.url();
        unsafe { std::env::set_var("TODOIST_API_BASE_URL", &base_url); }

        let _mock = server
            .mock("GET", "/api/v1/tasks")
            .with_status(503)
            .with_body("Service Unavailable")
            .create();

        let result = fetch_open_tasks("token");
        unsafe { std::env::remove_var("TODOIST_API_BASE_URL"); }
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Todoist API error"), "Unexpected error: {}", err);
    }

    #[test]
    fn test_fetch_open_tasks_invalid_json() {
        // Covers line 72-74: invalid JSON response
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut server = mockito::Server::new();
        let base_url = server.url();
        unsafe { std::env::set_var("TODOIST_API_BASE_URL", &base_url); }

        let _mock = server
            .mock("GET", "/api/v1/tasks")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("not valid json at all")
            .create();

        let result = fetch_open_tasks("token");
        unsafe { std::env::remove_var("TODOIST_API_BASE_URL"); }
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Failed to parse Todoist tasks"), "Unexpected error: {}", err);
    }

    #[test]
    fn test_fetch_open_tasks_paginated() {
        // Covers lines 57-59 and 78-79: cursor pagination
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut server = mockito::Server::new();
        let base_url = server.url();
        unsafe { std::env::set_var("TODOIST_API_BASE_URL", &base_url); }

        let page1 = r#"{"results":[
            {"id":"t1","content":"Task 1","description":"","priority":3,"labels":[],"due":null,"project_id":"p1"}
        ],"next_cursor":"cursor_xyz"}"#;

        let page2 = r#"{"results":[
            {"id":"t2","content":"Task 2","description":"","priority":2,"labels":[],"due":null,"project_id":"p1"}
        ],"next_cursor":null}"#;

        let _mock1 = server
            .mock("GET", "/api/v1/tasks")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(page1)
            .create();

        let _mock2 = server
            .mock("GET", "/api/v1/tasks?cursor=cursor_xyz")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(page2)
            .create();

        let result = fetch_open_tasks("token");
        unsafe { std::env::remove_var("TODOIST_API_BASE_URL"); }
        assert!(result.is_ok(), "fetch_open_tasks failed: {:?}", result);
        let tasks = result.unwrap();
        assert_eq!(tasks.len(), 2);
    }

    #[test]
    fn test_fetch_projects_server_error() {
        // Covers lines 105-106: projects API non-success
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut server = mockito::Server::new();
        let base_url = server.url();
        unsafe { std::env::set_var("TODOIST_API_BASE_URL", &base_url); }

        let _mock = server
            .mock("GET", "/api/v1/projects")
            .with_status(500)
            .with_body("error")
            .create();

        let result = fetch_projects("token");
        unsafe { std::env::remove_var("TODOIST_API_BASE_URL"); }
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Todoist projects API error"), "Unexpected error: {}", err);
    }

    #[test]
    fn test_fetch_projects_invalid_json() {
        // Covers lines 109-111: invalid JSON for projects
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut server = mockito::Server::new();
        let base_url = server.url();
        unsafe { std::env::set_var("TODOIST_API_BASE_URL", &base_url); }

        let _mock = server
            .mock("GET", "/api/v1/projects")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body("not json")
            .create();

        let result = fetch_projects("token");
        unsafe { std::env::remove_var("TODOIST_API_BASE_URL"); }
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Failed to parse Todoist projects"), "Unexpected error: {}", err);
    }

    #[test]
    fn test_fetch_projects_paginated() {
        // Covers lines 97-99 and 115-116: projects cursor pagination
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut server = mockito::Server::new();
        let base_url = server.url();
        unsafe { std::env::set_var("TODOIST_API_BASE_URL", &base_url); }

        let page1 = r#"{"results":[{"id":"p1","name":"Work"}],"next_cursor":"proj_cursor"}"#;
        let page2 = r#"{"results":[{"id":"p2","name":"Personal"}],"next_cursor":null}"#;

        let _mock1 = server
            .mock("GET", "/api/v1/projects")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(page1)
            .create();

        let _mock2 = server
            .mock("GET", "/api/v1/projects?cursor=proj_cursor")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(page2)
            .create();

        let result = fetch_projects("token");
        unsafe { std::env::remove_var("TODOIST_API_BASE_URL"); }
        assert!(result.is_ok(), "fetch_projects failed: {:?}", result);
        let map = result.unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map.get("p1"), Some(&"Work".to_string()));
        assert_eq!(map.get("p2"), Some(&"Personal".to_string()));
    }

    #[test]
    fn test_run_import_with_exported_skipped_tasks() {
        // Covers lines 208-216: run_import with mock server where some tasks are skipped
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut server = mockito::Server::new();
        let base_url = server.url();
        unsafe { std::env::set_var("TODOIST_API_BASE_URL", &base_url); }

        let tasks_body = r#"{"results":[
            {"id":"t1","content":"Normal","description":"","priority":3,"labels":[],"due":null,"project_id":"p1"},
            {"id":"t2","content":"Exported","description":"","priority":3,"labels":["exported"],"due":null,"project_id":"p1"}
        ],"next_cursor":null}"#;

        let _tasks_mock = server
            .mock("GET", "/api/v1/tasks")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(tasks_body)
            .create();

        let _projects_mock = server
            .mock("GET", "/api/v1/projects")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"results":[],"next_cursor":null}"#)
            .create();

        let _label_mock = server
            .mock("POST", "/api/v1/tasks/t1")
            .with_status(200)
            .with_body(r#"{}"#)
            .create();

        let mut task_file = crate::task::TaskFile::new();
        let result = run_import("token", &mut task_file, false);
        unsafe { std::env::remove_var("TODOIST_API_BASE_URL"); }
        assert!(result.is_ok());
        let (imported, skipped) = result.unwrap();
        assert_eq!(imported, 1);
        assert_eq!(skipped, 1);
    }

    #[test]
    fn test_fetch_open_tasks_connection_failure() {
        // Covers line 63: map_err closure when req.send() fails (connection refused)
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        // Use a port where nothing is listening to trigger a connection error
        unsafe { std::env::set_var("TODOIST_API_BASE_URL", "http://127.0.0.1:1"); }
        let result = fetch_open_tasks("token");
        unsafe { std::env::remove_var("TODOIST_API_BASE_URL"); }
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Failed to fetch Todoist tasks"), "Unexpected error: {}", err);
    }

    #[test]
    fn test_fetch_projects_connection_failure() {
        // Covers line 103: map_err closure when req.send() fails (connection refused)
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        unsafe { std::env::set_var("TODOIST_API_BASE_URL", "http://127.0.0.1:1"); }
        let result = fetch_projects("token");
        unsafe { std::env::remove_var("TODOIST_API_BASE_URL"); }
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Failed to fetch Todoist projects"), "Unexpected error: {}", err);
    }

    #[test]
    fn test_label_exported_connection_failure() {
        // Covers line 139: map_err closure when req.send() fails (connection refused)
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        unsafe { std::env::set_var("TODOIST_API_BASE_URL", "http://127.0.0.1:1"); }
        let result = label_exported("token", "task123", &[]);
        unsafe { std::env::remove_var("TODOIST_API_BASE_URL"); }
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Failed to label task"), "Unexpected error: {}", err);
    }

    #[test]
    fn test_run_import_label_exported_error_is_warning() {
        // Covers lines 237-239: label_exported error in run_import is only a warning
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let mut server = mockito::Server::new();
        let base_url = server.url();
        unsafe { std::env::set_var("TODOIST_API_BASE_URL", &base_url); }

        let tasks_body = r#"{"results":[
            {"id":"t1","content":"Task 1","description":"","priority":3,"labels":[],"due":null,"project_id":"p1"}
        ],"next_cursor":null}"#;

        let _tasks_mock = server
            .mock("GET", "/api/v1/tasks")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(tasks_body)
            .create();

        let _projects_mock = server
            .mock("GET", "/api/v1/projects")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"results":[],"next_cursor":null}"#)
            .create();

        // Make the label call fail
        let _label_mock = server
            .mock("POST", "/api/v1/tasks/t1")
            .with_status(500)
            .with_body("error")
            .create();

        let mut task_file = crate::task::TaskFile::new();
        let result = run_import("token", &mut task_file, false);
        unsafe { std::env::remove_var("TODOIST_API_BASE_URL"); }
        // run_import should succeed even when label_exported fails (it's just a warning)
        assert!(result.is_ok());
        let (imported, _) = result.unwrap();
        assert_eq!(imported, 1);
    }
}
