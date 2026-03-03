use serde_json::{json, Value};

use crate::task::{Task, Status};

pub fn print_task_table(tasks: &[Task], json_mode: bool) {
    if json_mode {
        let task_values: Vec<Value> = tasks.iter().map(|t| task_to_json(t)).collect();
        let out = json!({ "ok": true, "tasks": task_values });
        println!("{}", serde_json::to_string(&out).unwrap());
        return;
    }

    if tasks.is_empty() {
        println!("No tasks found.");
        return;
    }

    let show_due = tasks.iter().any(|t| t.due_date.is_some());
    let show_project = tasks.iter().any(|t| t.project.is_some());

    let id_w = 4;
    let status_w = 6;
    let pri_w = 8;
    let title_w = 28;
    let due_w = 12;
    let proj_w = 15;

    // Build header
    let mut header = format!(
        "{:>id_w$}  {:status_w$}  {:pri_w$}  {:title_w$}",
        "ID", "Status", "Pri", "Title",
        id_w = id_w, status_w = status_w, pri_w = pri_w, title_w = title_w,
    );
    let mut sep = format!(
        "{:>id_w$}  {:status_w$}  {:pri_w$}  {:title_w$}",
        "──", "──────", "───", "─────",
        id_w = id_w, status_w = status_w, pri_w = pri_w, title_w = title_w,
    );
    if show_due {
        header.push_str(&format!("  {:due_w$}", "Due", due_w = due_w));
        sep.push_str(&format!("  {:due_w$}", "───", due_w = due_w));
    }
    if show_project {
        header.push_str(&format!("  {:proj_w$}", "Project", proj_w = proj_w));
        sep.push_str(&format!("  {:proj_w$}", "───────", proj_w = proj_w));
    }
    header.push_str("  Tags");
    sep.push_str("  ────");
    println!("{}", header);
    println!("{}", sep);

    for task in tasks {
        let status_str = match task.status {
            Status::Open => "[ ]",
            Status::Done => "[x]",
        };
        let pri_str = format!("{}", task.priority);
        let tags_str = if task.tags.is_empty() {
            String::new()
        } else {
            task.tags.join(", ")
        };
        let title = if task.title.chars().count() > title_w {
            let truncated: String = task.title.chars().take(title_w - 1).collect();
            format!("{}…", truncated)
        } else {
            task.title.clone()
        };

        let mut row = format!(
            "{:>id_w$}  {:status_w$}  {:pri_w$}  {:title_w$}",
            task.id, status_str, pri_str, title,
            id_w = id_w, status_w = status_w, pri_w = pri_w, title_w = title_w,
        );
        if show_due {
            let due_str = task.due_date
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default();
            row.push_str(&format!("  {:due_w$}", due_str, due_w = due_w));
        }
        if show_project {
            let proj_str = task.project.as_deref().unwrap_or("");
            let proj_display = if proj_str.chars().count() > proj_w {
                let t: String = proj_str.chars().take(proj_w - 1).collect();
                format!("{}…", t)
            } else {
                proj_str.to_string()
            };
            row.push_str(&format!("  {:proj_w$}", proj_display, proj_w = proj_w));
        }
        row.push_str(&format!("  {}", tags_str));
        println!("{}", row);
    }
}

pub fn print_task_detail(task: &Task, json_mode: bool) {
    if json_mode {
        let out = json!({ "ok": true, "task": task_to_json(task) });
        println!("{}", serde_json::to_string(&out).unwrap());
        return;
    }

    let status_str = match task.status {
        Status::Open => "open",
        Status::Done => "done",
    };
    println!("ID:       {}", task.id);
    println!("Title:    {}", task.title);
    println!("Status:   {}", status_str);
    println!("Priority: {}", task.priority);
    if !task.tags.is_empty() {
        println!("Tags:     {}", task.tags.join(", "));
    }
    if let Some(due) = task.due_date {
        println!("Due:      {}", due.format("%Y-%m-%d"));
    }
    if let Some(ref proj) = task.project {
        println!("Project:  {}", proj);
    }
    println!("Created:  {}", task.created.to_rfc3339());
    if let Some(updated) = task.updated {
        println!("Updated:  {}", updated.to_rfc3339());
    }
    if let Some(ref desc) = task.description {
        println!();
        println!("{}", desc);
    }
}

pub fn print_success(message: &str, json_mode: bool) {
    if json_mode {
        let out = json!({ "ok": true, "message": message });
        println!("{}", serde_json::to_string(&out).unwrap());
    } else {
        println!("{}", message);
    }
}

pub fn print_error(message: &str, json_mode: bool) {
    if json_mode {
        let out = json!({ "ok": false, "error": message });
        eprintln!("{}", serde_json::to_string(&out).unwrap());
    } else {
        eprintln!("Error: {}", message);
    }
}

fn task_to_json(task: &Task) -> Value {
    serde_json::to_value(task).unwrap_or(json!(null))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn make_task(id: u32, title: &str, status: Status, priority: crate::task::Priority) -> Task {
        Task {
            id,
            title: title.to_string(),
            status,
            priority,
            tags: Vec::new(),
            created: Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
            updated: None,
            description: None,
            due_date: None,
            project: None,
            recurrence: None,
        }
    }

    // -- print_success --

    #[test]
    fn test_print_success_text_mode() {
        // Just call it; if no panic it works
        print_success("Operation successful", false);
    }

    #[test]
    fn test_print_success_json_mode() {
        print_success("Operation successful", true);
    }

    // -- print_error --

    #[test]
    fn test_print_error_text_mode() {
        print_error("Something went wrong", false);
    }

    #[test]
    fn test_print_error_json_mode() {
        print_error("Something went wrong", true);
    }

    // -- print_task_table --

    #[test]
    fn test_print_task_table_empty_non_json() {
        print_task_table(&[], false);
    }

    #[test]
    fn test_print_task_table_empty_json() {
        print_task_table(&[], true);
    }

    #[test]
    fn test_print_task_table_with_tasks_text() {
        use crate::task::Priority;
        let tasks = vec![
            make_task(1, "Task A", Status::Open, Priority::High),
            make_task(2, "Task B", Status::Done, Priority::Low),
        ];
        print_task_table(&tasks, false);
    }

    #[test]
    fn test_print_task_table_with_tasks_json() {
        use crate::task::Priority;
        let tasks = vec![
            make_task(1, "Task A", Status::Open, Priority::High),
        ];
        print_task_table(&tasks, true);
    }

    #[test]
    fn test_print_task_table_with_due_date() {
        use crate::task::Priority;
        use chrono::NaiveDate;
        let mut task = make_task(1, "Due task", Status::Open, Priority::Medium);
        task.due_date = Some(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap());
        print_task_table(&[task], false);
    }

    #[test]
    fn test_print_task_table_with_project() {
        use crate::task::Priority;
        let mut task = make_task(1, "Proj task", Status::Open, Priority::Medium);
        task.project = Some("myproject".to_string());
        print_task_table(&[task], false);
    }

    #[test]
    fn test_print_task_table_long_title_truncated() {
        use crate::task::Priority;
        let mut task = make_task(1, &"A".repeat(50), Status::Open, Priority::Medium);
        task.tags = vec!["tag1".to_string(), "tag2".to_string()];
        print_task_table(&[task], false);
    }

    #[test]
    fn test_print_task_table_long_project_truncated() {
        use crate::task::Priority;
        let mut task = make_task(1, "Task", Status::Open, Priority::Medium);
        task.project = Some("A".repeat(20));
        print_task_table(&[task], false);
    }

    #[test]
    fn test_print_task_table_with_due_and_project() {
        use crate::task::Priority;
        use chrono::NaiveDate;
        let mut task = make_task(1, "Full task", Status::Open, Priority::High);
        task.due_date = Some(NaiveDate::from_ymd_opt(2025, 6, 15).unwrap());
        task.project = Some("alpha".to_string());
        task.tags = vec!["web".to_string()];
        print_task_table(&[task], false);
    }

    #[test]
    fn test_print_task_table_done_status_symbol() {
        use crate::task::Priority;
        let task = make_task(1, "Done task", Status::Done, Priority::Medium);
        // Just verify no panic; we can't easily capture stdout in unit tests
        print_task_table(&[task], false);
    }

    #[test]
    fn test_print_task_table_critical_priority() {
        use crate::task::Priority;
        let task = make_task(1, "Critical", Status::Open, Priority::Critical);
        print_task_table(&[task], false);
    }

    // -- print_task_detail --

    #[test]
    fn test_print_task_detail_text_mode() {
        use crate::task::Priority;
        let task = make_task(1, "Detail task", Status::Open, Priority::Medium);
        print_task_detail(&task, false);
    }

    #[test]
    fn test_print_task_detail_json_mode() {
        use crate::task::Priority;
        let task = make_task(1, "Detail task", Status::Done, Priority::High);
        print_task_detail(&task, true);
    }

    #[test]
    fn test_print_task_detail_with_tags() {
        use crate::task::Priority;
        let mut task = make_task(1, "Tagged task", Status::Open, Priority::Low);
        task.tags = vec!["alpha".to_string(), "beta".to_string()];
        print_task_detail(&task, false);
    }

    #[test]
    fn test_print_task_detail_with_due_date() {
        use crate::task::Priority;
        use chrono::NaiveDate;
        let mut task = make_task(1, "Due task", Status::Open, Priority::Medium);
        task.due_date = Some(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap());
        print_task_detail(&task, false);
    }

    #[test]
    fn test_print_task_detail_with_project() {
        use crate::task::Priority;
        let mut task = make_task(1, "Proj task", Status::Open, Priority::Medium);
        task.project = Some("myproject".to_string());
        print_task_detail(&task, false);
    }

    #[test]
    fn test_print_task_detail_with_updated() {
        use crate::task::Priority;
        let mut task = make_task(1, "Updated task", Status::Done, Priority::Medium);
        task.updated = Some(Utc.with_ymd_and_hms(2025, 6, 1, 12, 0, 0).unwrap());
        print_task_detail(&task, false);
    }

    #[test]
    fn test_print_task_detail_with_description() {
        use crate::task::Priority;
        let mut task = make_task(1, "Task with desc", Status::Open, Priority::Medium);
        task.description = Some("This is a description.".to_string());
        print_task_detail(&task, false);
    }

    #[test]
    fn test_print_task_detail_done_text() {
        use crate::task::Priority;
        let task = make_task(1, "Task", Status::Done, Priority::Medium);
        print_task_detail(&task, false);
    }

    #[test]
    fn test_print_task_table_no_due_no_project_no_extra_columns() {
        use crate::task::Priority;
        // Mix: some tasks with due, some without — only one needs due to show column
        let mut task1 = make_task(1, "No due", Status::Open, Priority::Medium);
        let mut task2 = make_task(2, "Has due", Status::Open, Priority::High);
        use chrono::NaiveDate;
        task2.due_date = Some(NaiveDate::from_ymd_opt(2025, 6, 1).unwrap());
        // task1 has no due, task2 does -> column should appear, task1 row has empty due cell
        print_task_table(&[task1, task2], false);
    }
}
