use std::path::Path;
use std::str::FromStr;

use chrono::Utc;

use crate::db;
use crate::parser;
use crate::task::{Effort, Priority, Status, Task};

fn parse_csv(s: Option<&str>) -> Vec<String> {
    s.unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub struct AddArgs {
    pub title: String,
    pub priority: String,
    pub due: Option<String>,
    pub project: Option<String>,
    pub tags: Option<String>,
    pub agent: Option<String>,
    pub description: Option<String>,
    pub instructions: Option<String>,
}

pub struct ListArgs {
    pub status: Option<String>,
    pub agent: Option<String>,
    pub project: Option<String>,
    pub tag: Option<String>,
    pub due_before: Option<String>,
}

pub struct EditArgs {
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
    pub recur: Option<String>,
}

pub fn add(path: &Path, args: AddArgs) -> Result<String, (i32, String)> {
    let mut task_file = db::begin(path).map_err(|e| (1, e))?;
    let priority = Priority::from_str(&args.priority).map_err(|e| (1, e))?;
    let today = chrono::Local::now().date_naive();
    let due_date = args.due.as_deref().and_then(|d| parser::parse_due_date_input(d, today));
    let tags = parse_csv(args.tags.as_deref());

    let id = task_file.next_id;
    task_file.next_id += 1;
    task_file.tasks.push(Task {
        id,
        title: args.title.clone(),
        status: Status::Open,
        priority,
        tags,
        created: Utc::now(),
        updated: None,
        description: args.description,
        instructions: args.instructions,
        due_date,
        project: args.project,
        recurrence: None,
        notes: Vec::new(),
        agent: args.agent,
        effort: None,
        work_status: None,
    });
    task_file.commit().map_err(|e| (1, e))?;
    Ok(format!("Created task {}: {}", id, args.title))
}

pub fn filter_and_sort_tasks<'a>(
    tasks: &'a [Task],
    status: Option<Status>,
    agent: Option<&str>,
    project: Option<&str>,
    tag: Option<&str>,
    due_before: Option<chrono::NaiveDate>,
) -> Vec<&'a Task> {
    let mut result: Vec<&Task> = tasks
        .iter()
        .filter(|t| status.is_none_or(|s| t.status == s))
        .filter(|t| agent.is_none_or(|a| t.agent.as_deref() == Some(a)))
        .filter(|t| project.is_none_or(|p| t.project.as_deref() == Some(p)))
        .filter(|t| tag.is_none_or(|tag| t.tags.iter().any(|x| x == tag)))
        .filter(|t| due_before.is_none_or(|d| t.due_date.is_some_and(|td| td <= d)))
        .collect();

    result.sort_by(|a, b| {
        let date_cmp = match (a.due_date, b.due_date) {
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        };
        date_cmp.then(a.priority.cmp(&b.priority))
    });

    result
}

pub fn list(path: &Path, args: ListArgs) -> Result<String, (i32, String)> {
    let task_file = db::load(path).map_err(|e| (1, e))?;
    let status_filter = args
        .status
        .as_deref()
        .map(Status::from_str)
        .transpose()
        .map_err(|e| (1, e))?;
    let due_before = args
        .due_before
        .as_deref()
        .map(|s| {
            chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map_err(|_| (1, format!("Invalid date: '{}'. Expected YYYY-MM-DD.", s)))
        })
        .transpose()?;

    let tasks = filter_and_sort_tasks(
        &task_file.tasks,
        status_filter,
        args.agent.as_deref(),
        args.project.as_deref(),
        args.tag.as_deref(),
        due_before,
    );

    if tasks.is_empty() {
        return Ok("No matching tasks.".to_string());
    }
    let mut lines = Vec::new();
    for t in tasks {
        let due = t.due_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "-".to_string());
        let status = if t.status == Status::Done { "x" } else { " " };
        lines.push(format!("[{}] {:>4}  {:<8} {:<10}  {}", status, t.id, t.priority, due, t.title));
    }
    Ok(lines.join("\n"))
}

pub fn show(path: &Path, id: u32) -> Result<String, (i32, String)> {
    let task_file = db::load(path).map_err(|e| (1, e))?;
    let t = task_file.find_task(id).ok_or_else(|| (1, format!("Task {} not found", id)))?;

    let mut out = format!(
        "[{}] {} ({})\n",
        if t.status == Status::Done { "x" } else { " " },
        t.title,
        t.id
    );
    out.push_str(&format!("priority: {}\n", t.priority));
    if !t.tags.is_empty() {
        out.push_str(&format!("tags: {}\n", t.tags.join(",")));
    }
    if let Some(d) = t.due_date {
        out.push_str(&format!("due: {}\n", d.format("%Y-%m-%d")));
    }
    if let Some(ref p) = t.project {
        out.push_str(&format!("project: {}\n", p));
    }
    if let Some(ref r) = t.recurrence {
        out.push_str(&format!("recur: {}\n", r));
    }
    if !t.notes.is_empty() {
        out.push_str(&format!("notes: {}\n", t.notes.join(",")));
    }
    if let Some(ref a) = t.agent {
        out.push_str(&format!("agent: {}\n", a));
    }
    if let Some(ref e) = t.effort {
        out.push_str(&format!("effort: {}\n", e));
    }
    if let Some(ref w) = t.work_status {
        out.push_str(&format!("work_status: {}\n", w));
    }
    out.push_str(&format!("created: {}\n", t.created.to_rfc3339()));
    if let Some(u) = t.updated {
        out.push_str(&format!("updated: {}\n", u.to_rfc3339()));
    }
    if let Some(ref desc) = t.description {
        out.push('\n');
        out.push_str(desc);
        out.push('\n');
    }
    if let Some(ref ins) = t.instructions {
        out.push_str("\n## Instructions\n");
        out.push_str(ins);
        out.push('\n');
    }
    Ok(out.trim_end().to_string())
}

pub fn edit(path: &Path, id: u32, args: EditArgs) -> Result<String, (i32, String)> {
    let mut task_file = db::begin(path).map_err(|e| (1, e))?;
    let today = chrono::Local::now().date_naive();
    let priority = args.priority.as_deref().map(Priority::from_str).transpose().map_err(|e| (1, e))?;
    let effort = args.effort.as_deref().map(Effort::from_str).transpose().map_err(|e| (1, e))?;
    let work_status = args.work_status.as_deref().map(crate::task::WorkStatus::from_str).transpose().map_err(|e| (1, e))?;
    let recur = args.recur.as_deref().map(crate::task::Recurrence::from_str).transpose().map_err(|e| (1, e))?;
    let due_date = args
        .due
        .as_deref()
        .map(|d| {
            parser::parse_due_date_input(d, today).ok_or_else(|| (1, format!("Invalid due date: '{}'", d)))
        })
        .transpose()?;
    let tags = args.tags.as_deref().map(|s| parse_csv(Some(s)));

    let t = task_file.find_task_mut(id).ok_or_else(|| (1, format!("Task {} not found", id)))?;
    if let Some(title) = args.title {
        t.title = title;
    }
    if let Some(p) = priority {
        t.priority = p;
    }
    if let Some(d) = due_date {
        t.due_date = Some(d);
    }
    if let Some(p) = args.project {
        t.project = Some(p);
    }
    if let Some(tg) = tags {
        t.tags = tg;
    }
    if let Some(a) = args.agent {
        t.agent = Some(a);
    }
    if let Some(d) = args.description {
        t.description = Some(d);
    }
    if let Some(i) = args.instructions {
        t.instructions = if i.is_empty() { None } else { Some(i) };
    }
    if let Some(e) = effort {
        t.effort = Some(e);
    }
    if let Some(w) = work_status {
        t.work_status = Some(w);
    }
    if let Some(r) = recur {
        t.recurrence = Some(r);
    }
    t.updated = Some(Utc::now());
    let title = t.title.clone();

    task_file.commit().map_err(|e| (1, e))?;
    Ok(format!("Updated task {}: {}", id, title))
}

pub fn done(path: &Path, id: u32) -> Result<String, (i32, String)> {
    let mut task_file = db::begin(path).map_err(|e| (1, e))?;
    let idx = task_file
        .tasks
        .iter()
        .position(|t| t.id == id)
        .ok_or_else(|| (1, format!("Task {} not found", id)))?;

    if task_file.tasks[idx].status == Status::Done {
        return Ok(format!("Task {} already done", id));
    }

    task_file.tasks[idx].status = Status::Done;
    task_file.tasks[idx].updated = Some(Utc::now());

    let mut spawned = None;
    if let Some(recur) = task_file.tasks[idx].recurrence {
        let parent = task_file.tasks[idx].clone();
        let next_due = crate::task::next_due_date(&recur, parent.due_date);
        let new_id = task_file.next_id;
        task_file.next_id += 1;
        task_file.tasks.push(Task {
            id: new_id,
            title: parent.title.clone(),
            status: Status::Open,
            priority: parent.priority,
            tags: parent.tags.clone(),
            created: Utc::now(),
            updated: None,
            description: parent.description.clone(),
            instructions: parent.instructions.clone(),
            due_date: Some(next_due),
            project: parent.project.clone(),
            recurrence: Some(recur),
            notes: parent.notes.clone(),
            agent: parent.agent.clone(),
            effort: parent.effort,
            work_status: None,
        });
        spawned = Some((new_id, next_due));
    }

    task_file.commit().map_err(|e| (1, e))?;
    match spawned {
        Some((new_id, next_due)) => Ok(format!(
            "Completed task {}. Next occurrence: task {}, due {}",
            id, new_id, next_due
        )),
        None => Ok(format!("Completed task {}", id)),
    }
}

pub fn reopen(path: &Path, id: u32) -> Result<String, (i32, String)> {
    let mut task_file = db::begin(path).map_err(|e| (1, e))?;
    let t = task_file.find_task_mut(id).ok_or_else(|| (1, format!("Task {} not found", id)))?;
    t.status = Status::Open;
    t.updated = Some(Utc::now());
    task_file.commit().map_err(|e| (1, e))?;
    Ok(format!("Reopened task {}", id))
}

pub fn rm(path: &Path, id: u32) -> Result<String, (i32, String)> {
    let mut task_file = db::begin(path).map_err(|e| (1, e))?;
    let removed = task_file.remove_task(id).ok_or_else(|| (1, format!("Task {} not found", id)))?;
    task_file.commit().map_err(|e| (1, e))?;
    Ok(format!("Deleted task {}: {}", id, removed.title))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn setup() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("tasks.db");
        (dir, path)
    }

    #[test]
    fn test_filter_and_sort_tasks_by_status_and_agent() {
        let mut tf = crate::task::TaskFile::new();
        tf.tasks.push(sample_task_for_filter(1, "Open task", Status::Open, "bot"));
        tf.tasks.push(sample_task_for_filter(2, "Other agent", Status::Done, "human"));
        let result = filter_and_sort_tasks(&tf.tasks, Some(Status::Open), Some("bot"), None, None, None);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
    }

    #[test]
    fn test_filter_and_sort_tasks_sorts_by_due_then_priority() {
        let mut tf = crate::task::TaskFile::new();
        let mut t1 = sample_task_for_filter(1, "No due, critical", Status::Open, "bot");
        t1.priority = Priority::Critical;
        t1.due_date = None;
        let mut t2 = sample_task_for_filter(2, "Due today, low", Status::Open, "bot");
        t2.priority = Priority::Low;
        t2.due_date = chrono::NaiveDate::from_ymd_opt(2026, 1, 1);
        tf.tasks.push(t1);
        tf.tasks.push(t2);
        let result = filter_and_sort_tasks(&tf.tasks, None, None, None, None, None);
        // Due date ascending, None last — task 2 (has a due date) sorts before task 1 (no due date)
        assert_eq!(result[0].id, 2);
        assert_eq!(result[1].id, 1);
    }

    fn sample_task_for_filter(id: u32, title: &str, status: Status, agent: &str) -> Task {
        Task {
            id,
            title: title.to_string(),
            status,
            priority: Priority::Medium,
            tags: Vec::new(),
            created: Utc::now(),
            updated: None,
            description: None, instructions: None,
            due_date: None,
            project: None,
            recurrence: None,
            notes: Vec::new(),
            agent: Some(agent.to_string()),
            effort: None,
            work_status: None,
        }
    }

    #[test]
    fn test_add_creates_task() {
        let (_dir, path) = setup();
        let msg = add(&path, AddArgs {
            title: "New task".to_string(),
            priority: "high".to_string(),
            due: Some("2026-12-31".to_string()),
            project: Some("Work".to_string()),
            tags: Some("a,b".to_string()),
            agent: Some("bot".to_string()),
            description: Some("desc".to_string()), instructions: None,
        }).unwrap();
        assert!(msg.contains("Created task 1"));

        let tf = db::load(&path).unwrap();
        assert_eq!(tf.tasks.len(), 1);
        assert_eq!(tf.tasks[0].priority, Priority::High);
        assert_eq!(tf.tasks[0].tags, vec!["a", "b"]);
    }

    #[test]
    fn test_add_invalid_priority_errors() {
        let (_dir, path) = setup();
        let err = add(&path, AddArgs {
            title: "X".to_string(), priority: "urgent".to_string(), due: None,
            project: None, tags: None, agent: None, description: None, instructions: None,
        }).unwrap_err();
        assert_eq!(err.0, 1);
        assert!(err.1.contains("Invalid priority"));
    }

    #[test]
    fn test_list_filters_by_status_and_agent() {
        let (_dir, path) = setup();
        add(&path, AddArgs { title: "Open task".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: Some("bot".to_string()), description: None, instructions: None }).unwrap();
        add(&path, AddArgs { title: "Other agent".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: Some("human".to_string()), description: None, instructions: None }).unwrap();
        done(&path, 2).unwrap();

        let out = list(&path, ListArgs { status: Some("open".to_string()), agent: Some("bot".to_string()), project: None, tag: None, due_before: None }).unwrap();
        assert!(out.contains("Open task"));
        assert!(!out.contains("Other agent"));
    }

    #[test]
    fn test_list_no_matches() {
        let (_dir, path) = setup();
        let out = list(&path, ListArgs { status: None, agent: Some("nobody".to_string()), project: None, tag: None, due_before: None }).unwrap();
        assert_eq!(out, "No matching tasks.");
    }

    #[test]
    fn test_show_existing_task() {
        let (_dir, path) = setup();
        add(&path, AddArgs { title: "Show me".to_string(), priority: "low".to_string(), due: None, project: None, tags: None, agent: None, description: Some("body text".to_string()), instructions: None }).unwrap();
        let out = show(&path, 1).unwrap();
        assert!(out.contains("Show me"));
        assert!(out.contains("priority: low"));
        assert!(out.contains("body text"));
    }

    #[test]
    fn test_instructions_add_show_edit_roundtrip() {
        let (_dir, path) = setup();
        add(&path, AddArgs { title: "Ins".to_string(), priority: "low".to_string(), due: None, project: None, tags: None, agent: None, description: Some("the desc".to_string()), instructions: Some("do step 1".to_string()) }).unwrap();
        let out = show(&path, 1).unwrap();
        assert!(out.contains("the desc"));
        assert!(out.contains("## Instructions\ndo step 1"));
        edit(&path, 1, EditArgs { title: None, priority: None, due: None, project: None, tags: None, agent: None, description: None, instructions: Some("do step 2".to_string()), effort: None, work_status: None, recur: None }).unwrap();
        let tf = db::load(&path).unwrap();
        assert_eq!(tf.tasks[0].instructions.as_deref(), Some("do step 2"));
        assert_eq!(tf.tasks[0].description.as_deref(), Some("the desc"));
    }

    #[test]
    fn test_show_missing_task_errors() {
        let (_dir, path) = setup();
        let err = show(&path, 99).unwrap_err();
        assert_eq!(err.0, 1);
        assert!(err.1.contains("not found"));
    }

    #[test]
    fn test_edit_updates_only_given_fields() {
        let (_dir, path) = setup();
        add(&path, AddArgs { title: "Original".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: None, description: None, instructions: None }).unwrap();
        edit(&path, 1, EditArgs { title: None, priority: Some("critical".to_string()), due: None, project: None, tags: None, agent: None, description: None, instructions: None, effort: None, work_status: None, recur: None }).unwrap();

        let tf = db::load(&path).unwrap();
        assert_eq!(tf.tasks[0].title, "Original");
        assert_eq!(tf.tasks[0].priority, Priority::Critical);
        assert!(tf.tasks[0].updated.is_some());
    }

    #[test]
    fn test_edit_missing_task_errors() {
        let (_dir, path) = setup();
        let err = edit(&path, 1, EditArgs { title: Some("x".to_string()), priority: None, due: None, project: None, tags: None, agent: None, description: None, instructions: None, effort: None, work_status: None, recur: None }).unwrap_err();
        assert_eq!(err.0, 1);
    }

    #[test]
    fn test_done_marks_status_done() {
        let (_dir, path) = setup();
        add(&path, AddArgs { title: "Finish me".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: None, description: None, instructions: None }).unwrap();
        let msg = done(&path, 1).unwrap();
        assert_eq!(msg, "Completed task 1");

        let tf = db::load(&path).unwrap();
        assert_eq!(tf.tasks[0].status, Status::Done);
    }

    #[test]
    fn test_done_recurring_task_spawns_next_occurrence() {
        let (_dir, path) = setup();
        add(&path, AddArgs { title: "Recurring".to_string(), priority: "medium".to_string(), due: Some("2026-01-05".to_string()), project: None, tags: None, agent: None, description: None, instructions: None }).unwrap();
        {
            let mut tf = db::load(&path).unwrap();
            tf.tasks[0].recurrence = Some(crate::task::Recurrence::from_str("weekly").unwrap());
            db::save(&path, &tf).unwrap();
        }

        let msg = done(&path, 1).unwrap();
        assert!(msg.contains("Next occurrence: task 2"));

        let tf = db::load(&path).unwrap();
        assert_eq!(tf.tasks.len(), 2);
        let original = tf.find_task(1).unwrap();
        assert_eq!(original.status, Status::Done);
        let spawned = tf.find_task(2).unwrap();
        assert_eq!(spawned.status, Status::Open);
        assert_eq!(spawned.title, "Recurring");
        assert_eq!(spawned.due_date, chrono::NaiveDate::from_ymd_opt(2026, 1, 12));
    }

    #[test]
    fn test_done_recurring_task_spawns_next_occurrence_with_same_agent() {
        let (_dir, path) = setup();
        add(&path, AddArgs {
            title: "Recurring".to_string(), priority: "medium".to_string(),
            due: Some("2026-01-05".to_string()), project: None, tags: None,
            agent: Some("bot".to_string()), description: None, instructions: None,
        }).unwrap();
        {
            let mut tf = db::load(&path).unwrap();
            tf.tasks[0].recurrence = Some(crate::task::Recurrence::from_str("weekly").unwrap());
            db::save(&path, &tf).unwrap();
        }

        done(&path, 1).unwrap();

        let tf = db::load(&path).unwrap();
        let spawned = tf.find_task(2).unwrap();
        assert_eq!(spawned.agent, Some("bot".to_string()));
    }

    #[test]
    fn test_done_already_done_is_a_noop_message() {
        let (_dir, path) = setup();
        add(&path, AddArgs { title: "T".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: None, description: None, instructions: None }).unwrap();
        done(&path, 1).unwrap();
        let msg = done(&path, 1).unwrap();
        assert_eq!(msg, "Task 1 already done");
    }

    #[test]
    fn test_reopen_sets_status_open() {
        let (_dir, path) = setup();
        add(&path, AddArgs { title: "T".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: None, description: None, instructions: None }).unwrap();
        done(&path, 1).unwrap();
        reopen(&path, 1).unwrap();
        let tf = db::load(&path).unwrap();
        assert_eq!(tf.tasks[0].status, Status::Open);
    }

    #[test]
    fn test_rm_deletes_task() {
        let (_dir, path) = setup();
        add(&path, AddArgs { title: "Doomed".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: None, description: None, instructions: None }).unwrap();
        let msg = rm(&path, 1).unwrap();
        assert!(msg.contains("Doomed"));
        let tf = db::load(&path).unwrap();
        assert!(tf.tasks.is_empty());
    }

    #[test]
    fn test_rm_missing_task_errors() {
        let (_dir, path) = setup();
        let err = rm(&path, 1).unwrap_err();
        assert_eq!(err.0, 1);
    }

    #[test]
    fn test_edit_sets_work_status_and_show_prints_it() {
        let (_dir, path) = setup();
        add(&path, AddArgs { title: "Track me".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: None, description: None, instructions: None }).unwrap();

        edit(&path, 1, EditArgs {
            title: None, priority: None, due: None, project: None, tags: None,
            agent: None, description: None, instructions: None, effort: None,
            work_status: Some("in-progress".to_string()), recur: None,
        }).unwrap();

        let output = show(&path, 1).unwrap();
        assert!(output.contains("work_status: in-progress"));
    }

    #[test]
    fn test_edit_invalid_work_status_returns_error() {
        let (_dir, path) = setup();
        add(&path, AddArgs { title: "Track me".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: None, description: None, instructions: None }).unwrap();

        let result = edit(&path, 1, EditArgs {
            title: None, priority: None, due: None, project: None, tags: None,
            agent: None, description: None, instructions: None, effort: None,
            work_status: Some("bogus".to_string()), recur: None,
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_edit_sets_recurrence_and_show_prints_it() {
        let (_dir, path) = setup();
        add(&path, AddArgs { title: "Recur me".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: None, description: None, instructions: None }).unwrap();

        edit(&path, 1, EditArgs {
            title: None, priority: None, due: None, project: None, tags: None,
            agent: None, description: None, instructions: None, effort: None, work_status: None,
            recur: Some("weekly:fri".to_string()),
        }).unwrap();

        let output = show(&path, 1).unwrap();
        assert!(output.contains("recur: weekly:fri"));
    }

    #[test]
    fn test_edit_invalid_recurrence_returns_error() {
        let (_dir, path) = setup();
        add(&path, AddArgs { title: "Recur me".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: None, description: None, instructions: None }).unwrap();

        let result = edit(&path, 1, EditArgs {
            title: None, priority: None, due: None, project: None, tags: None,
            agent: None, description: None, instructions: None, effort: None, work_status: None,
            recur: Some("bogus".to_string()),
        });
        assert!(result.is_err());
    }
}
