use std::path::Path;
use std::str::FromStr;

use chrono::Utc;

use crate::db;
use crate::lifecycle::{self, Actor, Event};
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
    /// List services instead of tasks.
    pub services: bool,
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
    /// `Some(true)` makes the task a service, `Some(false)` a normal task.
    pub service: Option<bool>,
}

pub fn add(path: &Path, args: AddArgs) -> Result<String, (i32, String)> {
    let mut task_file = db::begin(path).map_err(|e| (1, e))?;
    let priority = Priority::from_str(&args.priority).map_err(|e| (1, e))?;
    let today = chrono::Local::now().date_naive();
    let due_date = args
        .due
        .as_deref()
        .map(|d| parser::parse_new_due_date(d, today).map_err(|e| (1, e)))
        .transpose()?;
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
        work_status: None, kind: Default::default(), paused: false,
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
    let result: Vec<&Task> = tasks
        .iter()
        .filter(|t| status.is_none_or(|s| t.status == s))
        .filter(|t| agent.is_none_or(|a| t.agent.as_deref().is_some_and(|x| x.eq_ignore_ascii_case(a))))
        .filter(|t| project.is_none_or(|p| t.project.as_deref() == Some(p)))
        .filter(|t| tag.is_none_or(|tag| t.tags.iter().any(|x| x == tag)))
        .filter(|t| due_before.is_none_or(|d| t.due_date.is_some_and(|td| td <= d)))
        .collect();

    sort_by_due_then_priority(result)
}

/// Earliest due date first (undated last), then highest priority first.
pub fn sort_by_due_then_priority(mut tasks: Vec<&Task>) -> Vec<&Task> {
    tasks.sort_by(|a, b| {
        let date_cmp = match (a.due_date, b.due_date) {
            (Some(x), Some(y)) => x.cmp(&y),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        };
        date_cmp.then(a.priority.cmp(&b.priority))
    });
    tasks
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

    let rows: Vec<Task> = task_file.tasks.into_iter().filter(|t| t.is_service() == args.services).collect();
    let tasks = filter_and_sort_tasks(
        &rows,
        status_filter,
        args.agent.as_deref(),
        args.project.as_deref(),
        args.tag.as_deref(),
        due_before,
    );

    if tasks.is_empty() {
        return Ok(if args.services { "No matching services." } else { "No matching tasks." }.to_string());
    }
    let mut lines = Vec::new();
    for t in tasks {
        let due = t.due_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "-".to_string());
        let status = if t.status == Status::Done { "x" } else { " " };
        lines.push(format!("[{}] {:>4}  {:<8} {:<10}  {}", status, t.id, t.priority, due, t.title));
    }
    Ok(lines.join("\n"))
}

pub struct NextArgs {
    pub agent: String,
    pub limit: Option<usize>,
    pub json: bool,
}

/// The tasks `agent` should work now, in order. With `json`, the full work queue
/// (instructions, memory, review threads, answered questions, linked notes).
pub fn next(path: &Path, notes_dir: &Path, args: NextArgs) -> Result<String, (i32, String)> {
    let task_file = db::load(path).map_err(|e| (1, e))?;
    let profiles = crate::config::list_agent_profiles();
    let agent = crate::agent::resolve_name(&profiles, &args.agent);
    let today = chrono::Local::now().date_naive();
    let tasks = crate::agent::eligible(&task_file.tasks, &agent, today);

    if args.json {
        let v = crate::agent::queue_json(&agent, notes_dir, &tasks, args.limit);
        return serde_json::to_string_pretty(&v).map_err(|e| (1, e.to_string()));
    }
    if tasks.is_empty() {
        return Ok(format!("Nothing to work for {}.", agent));
    }
    let lines: Vec<String> = tasks
        .iter()
        .take(args.limit.unwrap_or(usize::MAX))
        .map(|t| {
            let entry = crate::agent::queue_entry(t, notes_dir);
            let due = t.due_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "-".to_string());
            format!("{:>4}  {:<17} {:<8} {:<10}  {}", t.id, entry.pickup.as_str(), t.priority, due, t.title)
        })
        .collect();
    Ok(lines.join("\n"))
}

/// The conversation on a task, old review/question notes merged in.
pub fn thread_show(notes_dir: &Path, id: u32) -> Result<String, (i32, String)> {
    let v = crate::thread::view(notes_dir, id).ok_or_else(|| (1, format!("Task {} has no conversation yet", id)))?;
    Ok(v.body)
}

/// Post to a task's conversation as the running actor. `--question` also asks via the
/// lifecycle (agents only); `--reply` leaves the work status alone.
pub fn thread_post(
    path: &Path,
    notes_dir: &Path,
    id: u32,
    reply: Option<String>,
    question: Option<String>,
    actor: Actor,
) -> Result<String, (i32, String)> {
    use crate::thread::Kind;
    let (kind, text) = match (reply, question) {
        (Some(t), None) => (Kind::Agent, t),
        (None, Some(t)) => (Kind::Question, t),
        _ => return Err((1, "give exactly one of --reply or --question".to_string())),
    };
    if text.trim().is_empty() {
        return Err((1, "text must not be empty".to_string()));
    }
    let mut task_file = db::begin(path).map_err(|e| (1, e))?;
    let today = chrono::Local::now().date_naive();
    let t = task_file.find_task_mut(id).ok_or_else(|| (1, format!("Task {} not found", id)))?;
    if t.is_service() {
        return Err((1, format!("Task {} is a service; services have no conversation. Record runs with `task service run`", id)));
    }
    if kind == Kind::Question {
        lifecycle::apply(t, Event::Ask, actor, false, today).map_err(|e| (1, e))?;
    }
    let archived = crate::thread::post(notes_dir, t, kind, &text, today).map_err(|e| (1, e))?;
    task_file.commit().map_err(|e| (1, e))?;
    crate::thread::archive(notes_dir, &archived).map_err(|e| (1, e))?;
    Ok(match kind {
        Kind::Question => format!("Asked on task {}; it is now needs-input", id),
        _ => format!("Replied on task {}", id),
    })
}

/// Fold every task's old review/question notes into its thread. With `dry_run`, only report.
pub fn thread_migrate(path: &Path, notes_dir: &Path, dry_run: bool) -> Result<String, (i32, String)> {
    let mut task_file = db::begin(path).map_err(|e| (1, e))?;
    let mut lines = Vec::new();
    let mut archive = Vec::new();
    for t in task_file.tasks.iter_mut() {
        let pending = crate::thread::pending_legacy(notes_dir, t.id);
        if pending.is_empty() {
            continue;
        }
        if dry_run {
            lines.push(format!("task {}: would merge {} into {}", t.id, pending.join(", "), crate::thread::slug(t.id)));
            continue;
        }
        let moved = crate::thread::migrate(notes_dir, t).map_err(|e| (1, format!("task {}: {}", t.id, e)))?;
        lines.push(format!("task {}: merged {} into {}", t.id, moved.join(", "), crate::thread::slug(t.id)));
        archive.extend(moved);
    }
    if dry_run {
        if lines.is_empty() {
            lines.push("Nothing to migrate.".to_string());
        }
        return Ok(lines.join("\n"));
    }
    task_file.commit().map_err(|e| (1, e))?;
    crate::thread::archive(notes_dir, &archive).map_err(|e| (1, e))?;
    if lines.is_empty() {
        lines.push("Nothing to migrate.".to_string());
    }
    Ok(lines.join("\n"))
}

/// `task note show|append|edit` on an old `task-N-question` / `task-N-review-thread` slug:
/// kept working for callers that predate the thread, and never replaces history.
pub fn legacy_note_write(path: &Path, notes_dir: &Path, legacy: crate::thread::Legacy, body: &str) -> Result<String, (i32, String)> {
    use crate::thread::{Kind, Legacy};
    let (id, kind) = match legacy {
        Legacy::Review(id) => (id, Kind::Agent),
        Legacy::Question(id) => (id, Kind::Question),
    };
    let mut task_file = db::begin(path).map_err(|e| (1, e))?;
    let t = task_file.find_task_mut(id).ok_or_else(|| (1, format!("Task {} not found", id)))?;
    let today = chrono::Local::now().date_naive();
    let archived = crate::thread::post_raw(notes_dir, t, kind, body, today).map_err(|e| (1, e))?;
    task_file.commit().map_err(|e| (1, e))?;
    crate::thread::archive(notes_dir, &archived).map_err(|e| (1, e))?;
    Ok(notes_dir.join(format!("{}.md", crate::thread::slug(id))).display().to_string())
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
    if t.is_service() {
        out.push_str("kind: service\n");
        out.push_str(&format!("paused: {}\n", if t.paused { "yes" } else { "no" }));
        match db::last_run(path, t.id).map_err(|e| (1, e))? {
            Some(r) => out.push_str(&format!("last_run: {} {}{}\n", r.ran_at, r.result, r.summary.map(|s| format!(" — {}", s)).unwrap_or_default())),
            None => out.push_str("last_run: never\n"),
        }
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
    edit_as(path, id, args, Actor::Human, false)
}

/// The actor running the CLI: an agent when TASK_AGENT is set.
pub fn cli_actor() -> Actor {
    if crate::agent::env_agent().is_some() { Actor::Agent } else { Actor::Human }
}

/// Edit a task. State changes (`work_status`, `service`) go through the
/// lifecycle rules for `actor`; `force` (human only) skips transition rules.
pub fn edit_as(path: &Path, id: u32, args: EditArgs, actor: Actor, force: bool) -> Result<String, (i32, String)> {
    if actor == Actor::Agent && args.agent.as_deref().is_some_and(|a| a.eq_ignore_ascii_case("human")) {
        return Err((1, format!("only a human can reassign task {} to human (TASK_AGENT is set)", id)));
    }
    let mut task_file = db::begin(path).map_err(|e| (1, e))?;
    let today = chrono::Local::now().date_naive();
    let priority = args.priority.as_deref().map(Priority::from_str).transpose().map_err(|e| (1, e))?;
    let effort = args.effort.as_deref().map(Effort::from_str).transpose().map_err(|e| (1, e))?;
    let work_status = args.work_status.as_deref().map(crate::task::WorkStatus::from_str).transpose().map_err(|e| (1, e))?;
    let recur = args.recur.as_deref().map(crate::task::Recurrence::from_str).transpose().map_err(|e| (1, e))?;
    let due_date = args
        .due
        .as_deref()
        .map(|d| parser::parse_new_due_date(d, today).map_err(|e| (1, e)))
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
    if let Some(r) = recur {
        t.recurrence = Some(r);
    }
    match args.service {
        Some(true) => lifecycle::apply(t, Event::ToService, actor, force, today).map_err(|e| (1, e))?,
        Some(false) => lifecycle::apply(t, Event::ToTask, actor, force, today).map_err(|e| (1, e))?,
        None => {}
    }
    if let Some(w) = work_status {
        let event = lifecycle::event_for_work_status(t, Some(w)).map_err(|e| (1, e))?;
        lifecycle::apply(t, event, actor, force, today).map_err(|e| (1, e))?;
    }
    t.validate_kind().map_err(|e| (1, e))?;
    t.updated = Some(Utc::now());
    let title = t.title.clone();

    task_file.commit().map_err(|e| (1, e))?;
    Ok(format!("Updated task {}: {}", id, title))
}

pub fn done(path: &Path, id: u32) -> Result<String, (i32, String)> {
    done_as(path, id, Actor::Human)
}

pub fn done_as(path: &Path, id: u32, actor: Actor) -> Result<String, (i32, String)> {
    let mut task_file = db::begin(path).map_err(|e| (1, e))?;
    let today = chrono::Local::now().date_naive();
    let c = lifecycle::complete(&mut task_file, id, actor, today).map_err(|e| (1, e))?;
    if c.already_done {
        return Ok(format!("Task {} already done", id));
    }
    task_file.commit().map_err(|e| (1, e))?;
    match c.spawned {
        Some(next) => Ok(format!(
            "Completed task {}. Next occurrence: task {}, due {}",
            id,
            next.id,
            next.due_date.map(|d| d.to_string()).unwrap_or_default()
        )),
        None => Ok(format!("Completed task {}", id)),
    }
}

pub fn reopen(path: &Path, id: u32) -> Result<String, (i32, String)> {
    reopen_as(path, id, Actor::Human, false)
}

pub fn reopen_as(path: &Path, id: u32, actor: Actor, force: bool) -> Result<String, (i32, String)> {
    let mut task_file = db::begin(path).map_err(|e| (1, e))?;
    let today = chrono::Local::now().date_naive();
    let t = task_file.find_task_mut(id).ok_or_else(|| (1, format!("Task {} not found", id)))?;
    lifecycle::apply(t, Event::Reopen, actor, force, today).map_err(|e| (1, e))?;
    task_file.commit().map_err(|e| (1, e))?;
    Ok(format!("Reopened task {}", id))
}

pub fn rm(path: &Path, id: u32) -> Result<String, (i32, String)> {
    let mut task_file = db::begin(path).map_err(|e| (1, e))?;
    let removed = task_file.remove_task(id).ok_or_else(|| (1, format!("Task {} not found", id)))?;
    if removed.is_service() {
        task_file.delete_runs(id).map_err(|e| (1, e))?;
    }
    task_file.commit().map_err(|e| (1, e))?;
    Ok(format!("Deleted task {}: {}", id, removed.title))
}

// -- Services --

/// Dispatch a `task service` subcommand. `caller` is the agent identity in
/// effect (TASK_AGENT or the cwd profile), if any.
pub fn service(path: &Path, cmd: crate::cli::ServiceCommand, caller: Option<&str>) -> Result<String, (i32, String)> {
    use crate::cli::ServiceCommand as S;
    let today = chrono::Local::now().date_naive();
    match cmd {
        S::Add { title, recur, due, agent, priority, project, tags, description, instructions } => {
            service_add(path, AddArgs { title, priority, due, project, tags, agent, description, instructions }, &recur, today)
        }
        S::List => list(path, ListArgs { status: None, agent: None, project: None, tag: None, due_before: None, services: true }),
        S::Show { id } => {
            load_service(path, id)?;
            show(path, id)
        }
        S::Run { id, result, summary, json } => {
            let ok = match result.to_lowercase().as_str() {
                "ok" => true,
                "failed" | "fail" => false,
                _ => return Err((1, format!("Invalid result: '{}'. Valid values: ok, failed", result))),
            };
            service_run(path, id, ok, summary, caller, today, json)
        }
        S::Pause { id } => set_paused_as(path, id, true, today, cli_actor()),
        S::Resume { id } => set_paused_as(path, id, false, today, cli_actor()),
        S::Runs { id, limit } => {
            load_service(path, id)?;
            let runs = db::list_runs(path, id, limit, None).map_err(|e| (1, e))?;
            if runs.is_empty() {
                return Ok(format!("No runs recorded for service {}.", id));
            }
            Ok(runs
                .iter()
                .map(|r| format!("{}  {:<6}  for {}  {}", r.ran_at, r.result, r.scheduled_for, r.summary.as_deref().unwrap_or("")).trim_end().to_string())
                .collect::<Vec<_>>()
                .join("\n"))
        }
    }
}

fn load_service(path: &Path, id: u32) -> Result<Task, (i32, String)> {
    let tf = db::load(path).map_err(|e| (1, e))?;
    let t = tf.find_task(id).ok_or_else(|| (1, format!("Task {} not found", id)))?;
    if !t.is_service() {
        return Err((1, format!("Task {} is not a service", id)));
    }
    Ok(t.clone())
}

pub fn service_add(path: &Path, args: AddArgs, recur: &str, today: chrono::NaiveDate) -> Result<String, (i32, String)> {
    let recurrence = crate::task::Recurrence::from_str(recur).map_err(|e| (1, e))?;
    let due_date = match args.due.as_deref() {
        Some(d) => parser::parse_new_due_date(d, today).map_err(|e| (1, e))?,
        None => today,
    };
    let priority = Priority::from_str(&args.priority).map_err(|e| (1, e))?;
    let mut task_file = db::begin(path).map_err(|e| (1, e))?;
    let id = task_file.next_id;
    task_file.next_id += 1;
    task_file.tasks.push(Task {
        id,
        title: args.title.clone(),
        status: Status::Open,
        priority,
        tags: parse_csv(args.tags.as_deref()),
        created: Utc::now(),
        updated: None,
        description: args.description,
        instructions: args.instructions,
        due_date: Some(due_date),
        project: args.project,
        recurrence: Some(recurrence),
        notes: Vec::new(),
        agent: args.agent,
        effort: None,
        work_status: None,
        kind: crate::task::TaskKind::Service,
        paused: false,
    });
    task_file.commit().map_err(|e| (1, e))?;
    Ok(format!("Created service {}: {} (next run {})", id, args.title, due_date))
}

/// Record one run of a service and move it to its next scheduled date after
/// `today`, skipping any missed dates.
pub fn service_run(
    path: &Path,
    id: u32,
    ok: bool,
    summary: Option<String>,
    caller: Option<&str>,
    today: chrono::NaiveDate,
    json: bool,
) -> Result<String, (i32, String)> {
    let mut task_file = db::begin(path).map_err(|e| (1, e))?;
    let t = task_file.find_task_mut(id).ok_or_else(|| (1, format!("Task {} not found", id)))?;
    if !t.is_service() {
        return Err((1, format!("Task {} is not a service", id)));
    }
    if t.paused {
        return Err((1, format!("Service {} is paused; resume it first", id)));
    }
    if let (Some(c), Some(a)) = (caller, t.agent.as_deref()) {
        if !c.eq_ignore_ascii_case(a) {
            return Err((1, format!("Service {} belongs to {}, not {}", id, a, c)));
        }
    }
    let Some(due) = t.due_date else {
        return Err((1, format!("Service {} has no recurrence or due date", id)));
    };
    if due > today {
        return Err((1, format!("Service {} is not due until {}", id, due)));
    }
    let actor = if caller.is_some() { Actor::Agent } else { Actor::Human };
    lifecycle::apply(t, if ok { Event::RunOk } else { Event::RunFailed }, actor, false, today).map_err(|e| (1, e))?;
    let next = t.due_date.unwrap_or(due);
    let mut run = db::ServiceRun {
        id: 0,
        task_id: id,
        agent: caller.map(str::to_string).or_else(|| t.agent.clone()),
        scheduled_for: due.format("%Y-%m-%d").to_string(),
        ran_at: Utc::now().to_rfc3339(),
        result: if ok { "ok" } else { "failed" }.to_string(),
        summary,
    };
    run.id = task_file.insert_run(&run).map_err(|e| (1, e))?;
    task_file.commit().map_err(|e| (1, e))?;
    if json {
        let v = serde_json::json!({"run": run, "next_due": next.format("%Y-%m-%d").to_string()});
        return serde_json::to_string_pretty(&v).map_err(|e| (1, e.to_string()));
    }
    Ok(format!("Recorded {} run of service {}. Next run: {}", run.result, id, next))
}

/// Pause or resume a service. Resuming a service whose date has passed moves
/// it to its first scheduled date on or after today.
pub fn set_paused(path: &Path, id: u32, paused: bool, today: chrono::NaiveDate) -> Result<String, (i32, String)> {
    set_paused_as(path, id, paused, today, Actor::Human)
}

pub fn set_paused_as(path: &Path, id: u32, paused: bool, today: chrono::NaiveDate, actor: Actor) -> Result<String, (i32, String)> {
    let mut task_file = db::begin(path).map_err(|e| (1, e))?;
    let t = task_file.find_task_mut(id).ok_or_else(|| (1, format!("Task {} not found", id)))?;
    let event = if paused { Event::Pause } else { Event::Resume };
    lifecycle::apply(t, event, actor, false, today).map_err(|e| (1, e))?;
    let msg = if paused {
        format!("Paused service {}", id)
    } else {
        format!("Resumed service {}. Next run: {}", id, t.due_date.map(|d| d.to_string()).unwrap_or_default())
    };
    task_file.commit().map_err(|e| (1, e))?;
    Ok(msg)
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
            work_status: None, kind: Default::default(), paused: false,
        }
    }

    #[test]
    fn test_add_creates_task() {
        let (_dir, path) = setup();
        let msg = add(&path, AddArgs {
            title: "New task".to_string(),
            priority: "high".to_string(),
            due: Some("2099-12-31".to_string()),
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
    fn test_add_and_edit_reject_past_due_date() {
        let (_dir, path) = setup();
        let err = add(&path, AddArgs {
            title: "X".to_string(), priority: "medium".to_string(), due: Some("2000-01-01".to_string()),
            project: None, tags: None, agent: None, description: None, instructions: None,
        }).unwrap_err();
        assert!(err.1.contains("in the past"), "{}", err.1);
        assert!(db::load(&path).unwrap().tasks.is_empty());

        add(&path, AddArgs { title: "Y".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: None, description: None, instructions: None }).unwrap();
        let err = edit(&path, 1, EditArgs { title: None, priority: None, due: Some("2000-01-01".to_string()), project: None, tags: None, agent: None, description: None, instructions: None, effort: None, work_status: None, recur: None, service: None }).unwrap_err();
        assert!(err.1.contains("in the past"), "{}", err.1);
        assert!(db::load(&path).unwrap().tasks[0].due_date.is_none());
    }

    #[test]
    fn test_list_filters_by_status_and_agent() {
        let (_dir, path) = setup();
        add(&path, AddArgs { title: "Open task".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: Some("bot".to_string()), description: None, instructions: None }).unwrap();
        add(&path, AddArgs { title: "Other agent".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: Some("human".to_string()), description: None, instructions: None }).unwrap();
        done(&path, 2).unwrap();

        let out = list(&path, ListArgs { status: Some("open".to_string()), agent: Some("bot".to_string()), project: None, tag: None, due_before: None, services: false }).unwrap();
        assert!(out.contains("Open task"));
        assert!(!out.contains("Other agent"));
    }

    #[test]
    fn test_list_no_matches() {
        let (_dir, path) = setup();
        let out = list(&path, ListArgs { status: None, agent: Some("nobody".to_string()), project: None, tag: None, due_before: None, services: false }).unwrap();
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
        edit(&path, 1, EditArgs { title: None, priority: None, due: None, project: None, tags: None, agent: None, description: None, instructions: Some("do step 2".to_string()), effort: None, work_status: None, recur: None, service: None }).unwrap();
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
        edit(&path, 1, EditArgs { title: None, priority: Some("critical".to_string()), due: None, project: None, tags: None, agent: None, description: None, instructions: None, effort: None, work_status: None, recur: None, service: None }).unwrap();

        let tf = db::load(&path).unwrap();
        assert_eq!(tf.tasks[0].title, "Original");
        assert_eq!(tf.tasks[0].priority, Priority::Critical);
        assert!(tf.tasks[0].updated.is_some());
    }

    #[test]
    fn test_edit_missing_task_errors() {
        let (_dir, path) = setup();
        let err = edit(&path, 1, EditArgs { title: Some("x".to_string()), priority: None, due: None, project: None, tags: None, agent: None, description: None, instructions: None, effort: None, work_status: None, recur: None, service: None }).unwrap_err();
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
        add(&path, AddArgs { title: "Recurring".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: None, description: None, instructions: None }).unwrap();
        {
            let mut tf = db::load(&path).unwrap();
            tf.tasks[0].due_date = chrono::NaiveDate::from_ymd_opt(2026, 1, 5);
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
            due: None, project: None, tags: None,
            agent: Some("bot".to_string()), description: None, instructions: None,
        }).unwrap();
        {
            let mut tf = db::load(&path).unwrap();
            tf.tasks[0].due_date = chrono::NaiveDate::from_ymd_opt(2026, 1, 5);
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
            work_status: Some("in-progress".to_string()), recur: None, service: None,
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
            work_status: Some("bogus".to_string()), recur: None, service: None,
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_edit_sets_recurrence_and_show_prints_it() {
        let (_dir, path) = setup();
        add(&path, AddArgs { title: "Recur me".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: None, description: None, instructions: None }).unwrap();

        edit(&path, 1, EditArgs {
            title: None, priority: None, due: None, project: None, tags: None,
            agent: None, description: None, instructions: None, effort: None, work_status: None, service: None,
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
            agent: None, description: None, instructions: None, effort: None, work_status: None, service: None,
            recur: Some("bogus".to_string()),
        });
        assert!(result.is_err());
    }

    fn d(s: &str) -> chrono::NaiveDate {
        chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn add_service(path: &Path, due: &str) {
        let args = AddArgs {
            title: "Sync".to_string(), priority: "medium".to_string(), due: Some(due.to_string()),
            project: None, tags: None, agent: Some("Bot".to_string()), description: None, instructions: None,
        };
        service_add(path, args, "daily", d("2000-01-01")).unwrap();
    }

    #[test]
    fn test_service_run_skips_missed_dates_and_records_run() {
        let (_dir, path) = setup();
        add_service(&path, "2026-10-01");
        let msg = service_run(&path, 1, true, Some("synced".to_string()), Some("bot"), d("2026-10-05"), false).unwrap();
        assert!(msg.contains("Next run: 2026-10-06"), "{}", msg);
        let tf = db::load(&path).unwrap();
        assert_eq!(tf.tasks.len(), 1, "a run must not spawn a new row");
        assert_eq!(tf.tasks[0].due_date, Some(d("2026-10-06")));
        let runs = db::list_runs(&path, 1, 10, None).unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].scheduled_for, "2026-10-01");
        assert_eq!(runs[0].summary.as_deref(), Some("synced"));
    }

    #[test]
    fn test_service_run_failed_sets_failing_and_ok_clears_it() {
        let (_dir, path) = setup();
        add_service(&path, "2026-10-01");
        service_run(&path, 1, false, None, None, d("2026-10-01"), false).unwrap();
        let t = db::load(&path).unwrap().tasks[0].clone();
        assert_eq!(t.work_status, Some(crate::task::WorkStatus::Failing));
        assert_eq!(t.due_date, Some(d("2026-10-02")));
        service_run(&path, 1, true, None, None, d("2026-10-02"), false).unwrap();
        assert_eq!(db::load(&path).unwrap().tasks[0].work_status, None);
    }

    #[test]
    fn test_service_run_rejects_not_due_paused_wrong_agent_and_plain_tasks() {
        let (_dir, path) = setup();
        add_service(&path, "2026-10-10");
        assert!(service_run(&path, 1, true, None, None, d("2026-10-05"), false).unwrap_err().1.contains("not due until"));
        assert!(service_run(&path, 1, true, None, Some("Other"), d("2026-10-10"), false).is_err());
        set_paused(&path, 1, true, d("2026-10-10")).unwrap();
        assert!(service_run(&path, 1, true, None, None, d("2026-10-10"), false).unwrap_err().1.contains("paused"));
        add(&path, AddArgs { title: "Plain".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: None, description: None, instructions: None }).unwrap();
        assert!(service_run(&path, 2, true, None, None, d("2026-10-10"), false).is_err());
        assert!(db::list_runs(&path, 1, 10, None).unwrap().is_empty());
    }

    fn add_bot_task(path: &Path) {
        add(path, AddArgs { title: "Work".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: Some("Bot".to_string()), description: None, instructions: None }).unwrap();
    }

    #[test]
    fn test_thread_post_question_asks_and_reply_keeps_status() {
        let (dir, path) = setup();
        let notes = dir.path().join("Notes");
        add_bot_task(&path);

        // A human cannot ask; an agent can, and the question goes to the thread.
        assert!(thread_post(&path, &notes, 1, None, Some("which?".into()), Actor::Human).is_err());
        thread_post(&path, &notes, 1, None, Some("which branch?".into()), Actor::Agent).unwrap();
        let t = db::load(&path).unwrap().tasks[0].clone();
        assert_eq!(t.work_status, Some(crate::task::WorkStatus::NeedsInput));
        assert_eq!(t.notes, vec!["task-1-thread".to_string()]);
        assert!(thread_show(&notes, 1).unwrap().contains("## Question —"));

        thread_post(&path, &notes, 1, Some("on it".into()), None, Actor::Agent).unwrap();
        assert_eq!(db::load(&path).unwrap().tasks[0].work_status, Some(crate::task::WorkStatus::NeedsInput));
        assert!(thread_post(&path, &notes, 1, None, None, Actor::Agent).is_err());
        assert!(thread_post(&path, &notes, 1, Some(" ".into()), None, Actor::Agent).is_err());
    }

    #[test]
    fn test_thread_post_refuses_services() {
        let (dir, path) = setup();
        add_service(&path, "2026-10-01");
        assert!(thread_post(&path, &dir.path().join("Notes"), 1, Some("hi".into()), None, Actor::Agent).is_err());
    }

    #[test]
    fn test_legacy_note_writes_append_to_the_thread_instead_of_replacing() {
        use crate::thread::Legacy;
        let (dir, path) = setup();
        let notes = dir.path().join("Notes");
        add_bot_task(&path);
        legacy_note_write(&path, &notes, Legacy::Question(1), "first question").unwrap();
        legacy_note_write(&path, &notes, Legacy::Question(1), "second question").unwrap();
        legacy_note_write(&path, &notes, Legacy::Review(1), "## Agent response — 2026-10-30\n\ndone").unwrap();
        let body = thread_show(&notes, 1).unwrap();
        assert!(body.contains("first question") && body.contains("second question") && body.contains("done"));
        assert_eq!(body.matches("## Question —").count(), 2);
        assert!(!notes.join("task-1-question.md").exists());
    }

    #[test]
    fn test_thread_migrate_dry_run_writes_nothing_then_merges() {
        let (dir, path) = setup();
        let notes = dir.path().join("Notes");
        add_bot_task(&path);
        crate::note::write_note(&notes, &crate::note::Note { slug: "task-1-review-thread".into(), title: "R".into(), body: "## Feedback — 2026-10-01\n\nfix".into() }).unwrap();
        let mut tf = db::begin(&path).unwrap();
        tf.find_task_mut(1).unwrap().notes.push("task-1-review-thread".into());
        tf.commit().unwrap();

        let msg = thread_migrate(&path, &notes, true).unwrap();
        assert!(msg.contains("would merge task-1-review-thread"), "{}", msg);
        assert!(notes.join("task-1-review-thread.md").exists() && !notes.join("task-1-thread.md").exists());

        thread_migrate(&path, &notes, false).unwrap();
        assert!(notes.join(".migrated/task-1-review-thread.md").is_file());
        assert_eq!(db::load(&path).unwrap().tasks[0].notes, vec!["task-1-thread".to_string()]);
        assert_eq!(thread_migrate(&path, &notes, false).unwrap(), "Nothing to migrate.");
    }

    #[test]
    fn test_resume_moves_past_due_date_to_today_or_later() {
        let (_dir, path) = setup();
        add_service(&path, "2026-10-01");
        set_paused(&path, 1, true, d("2026-10-01")).unwrap();
        let msg = set_paused(&path, 1, false, d("2026-10-05")).unwrap();
        assert!(msg.contains("2026-10-05"), "{}", msg);
        assert!(!db::load(&path).unwrap().tasks[0].paused);
    }

    #[test]
    fn test_services_hidden_from_list_and_done_refused_and_rm_drops_runs() {
        let (_dir, path) = setup();
        add_service(&path, "2026-10-01");
        let plain = ListArgs { status: None, agent: None, project: None, tag: None, due_before: None, services: false };
        assert_eq!(list(&path, plain).unwrap(), "No matching tasks.");
        let svc = ListArgs { status: None, agent: None, project: None, tag: None, due_before: None, services: true };
        assert!(list(&path, svc).unwrap().contains("Sync"));
        assert!(done(&path, 1).is_err());
        assert!(show(&path, 1).unwrap().contains("kind: service"));
        service_run(&path, 1, true, None, None, d("2026-10-01"), false).unwrap();
        rm(&path, 1).unwrap();
        assert!(db::list_runs(&path, 1, 10, None).unwrap().is_empty());
    }

    #[test]
    fn test_edit_service_requires_recurrence_and_failing_only_on_services() {
        let (_dir, path) = setup();
        add(&path, AddArgs { title: "Plain".to_string(), priority: "medium".to_string(), due: None, project: None, tags: None, agent: None, description: None, instructions: None }).unwrap();
        let to_service = EditArgs { title: None, priority: None, due: None, project: None, tags: None, agent: None, description: None, instructions: None, effort: None, work_status: None, recur: None, service: Some(true) };
        assert!(edit(&path, 1, to_service).unwrap_err().1.contains("recurrence"));
        let failing = EditArgs { title: None, priority: None, due: None, project: None, tags: None, agent: None, description: None, instructions: None, effort: None, work_status: Some("failing".to_string()), recur: None, service: None };
        assert!(edit(&path, 1, failing).is_err());
        let ok = EditArgs { title: None, priority: None, due: Some("2099-01-01".to_string()), project: None, tags: None, agent: None, description: None, instructions: None, effort: None, work_status: None, recur: Some("weekly".to_string()), service: Some(true) };
        edit(&path, 1, ok).unwrap();
        assert!(db::load(&path).unwrap().tasks[0].is_service());
    }
}
