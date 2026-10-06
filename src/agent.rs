//! Who is working, and what they should work on. Everything here is independent of
//! which agent harness is calling: identity comes from an environment variable or the
//! working directory, and the work queue is derived purely from `tasks.db` and notes.

use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use serde_json::{json, Value};

use crate::task::{Status, Task, WorkStatus};

/// Environment variable a harness sets to say which agent profile it is running as.
pub const AGENT_ENV: &str = "TASK_AGENT";

pub type Profiles = [(String, String)];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Env,
    WorkingDirectory,
}

impl Source {
    pub fn label(self) -> &'static str {
        match self {
            Source::Env => "TASK_AGENT",
            Source::WorkingDirectory => "working-directory",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Identity {
    pub name: String,
    pub source: Source,
    /// The profile's configured directory, when the name matches a profile.
    pub dir: Option<String>,
}

/// The profile whose name equals `name`, ignoring ASCII case. Returns the profile's own
/// spelling, which is what tasks and note folders are keyed on.
pub fn canonical_name(profiles: &Profiles, name: &str) -> Option<String> {
    profiles.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(n, _)| n.clone())
}

/// `name` as the matching profile spells it, or `name` unchanged if there is no such profile.
pub fn resolve_name(profiles: &Profiles, name: &str) -> String {
    canonical_name(profiles, name).unwrap_or_else(|| name.to_string())
}

/// The agent a harness declared through `TASK_AGENT`, if any.
pub fn env_agent() -> Option<String> {
    std::env::var(AGENT_ENV).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

/// Identity resolution: `TASK_AGENT` wins, then the profile whose directory contains `cwd`.
pub fn identity(env: Option<&str>, config: &Path, cwd: &Path) -> Option<Identity> {
    let profiles = crate::config::list_agent_profiles_from(config);
    if let Some(requested) = env.map(str::trim).filter(|v| !v.is_empty()) {
        let name = resolve_name(&profiles, requested);
        let dir = profiles.iter().find(|(n, _)| *n == name).map(|(_, d)| d.clone());
        return Some(Identity { name, source: Source::Env, dir });
    }
    let name = crate::config::find_agent_for_cwd_from(config, cwd)?;
    let dir = profiles.iter().find(|(n, _)| *n == name).map(|(_, d)| d.clone());
    Some(Identity { name, source: Source::WorkingDirectory, dir })
}

pub fn review_thread_slug(task_id: u32) -> String {
    format!("task-{}-review-thread", task_id)
}

pub fn question_slug(task_id: u32) -> String {
    format!("task-{}-question", task_id)
}

pub fn instructions_slug(name: &str) -> String {
    crate::config::read_config_value(&format!("agent-{}-instructions", name))
        .unwrap_or_else(|| crate::note::slugify(name))
}

pub fn agent_dir(notes_dir: &Path, slug: &str) -> PathBuf {
    notes_dir.join("Agents").join(slug)
}

/// Standing instructions: `Notes/Agents/<slug>/instructions.md`, else the legacy
/// `Notes/Instructions/<slug>.md`.
pub fn read_instructions(notes_dir: &Path, name: &str) -> Option<String> {
    let slug = instructions_slug(name);
    [agent_dir(notes_dir, &slug).join("instructions.md"), notes_dir.join("Instructions").join(format!("{}.md", slug))]
        .iter()
        .find(|p| p.exists())
        .and_then(|p| crate::note::read_note(p).ok())
        .map(|n| n.body)
}

pub fn read_memory(notes_dir: &Path, name: &str) -> Option<String> {
    let path = agent_dir(notes_dir, &crate::note::slugify(name)).join("memory.md");
    crate::note::read_note(&path).ok().map(|n| n.body)
}

fn read_body(notes_dir: &Path, slug: &str) -> Option<String> {
    crate::note::read_note(&notes_dir.join(format!("{}.md", slug))).ok().map(|n| n.body)
}

/// Why a task is in the queue, so a harness does not have to infer it from note contents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pickup {
    /// Not started yet.
    New,
    /// Already `in-progress`; resume it.
    Continue,
    /// The human left feedback on the review thread.
    ReviewFeedback,
    /// The human answered the `needs-input` question; the answer is in the question note.
    AnsweredQuestion,
}

impl Pickup {
    pub fn as_str(self) -> &'static str {
        match self {
            Pickup::New => "new",
            Pickup::Continue => "continue",
            Pickup::ReviewFeedback => "review_feedback",
            Pickup::AnsweredQuestion => "answered_question",
        }
    }
}

/// True when the last `## ` section of a question note is an answer.
fn ends_with_answer(question: &str) -> bool {
    question.lines().rev().find(|l| l.starts_with("## ")).is_some_and(|l| l.starts_with("## Answer"))
}

pub fn pickup_kind(task: &Task, question: Option<&str>) -> Pickup {
    match task.work_status {
        Some(WorkStatus::ChangesRequested) if question.is_some_and(ends_with_answer) => Pickup::AnsweredQuestion,
        Some(WorkStatus::ChangesRequested) => Pickup::ReviewFeedback,
        Some(WorkStatus::InProgress) => Pickup::Continue,
        _ => Pickup::New,
    }
}

/// Open tasks assigned to `agent` (case-insensitive) that are ready to be worked now, in
/// the order they should be worked: earliest due date first (none last), then priority.
///
/// Excluded: tasks already handed back (`waiting-for-review`, `complete`), tasks blocked
/// on the human (`needs-input` — the web UI moves them to `changes-requested` once
/// answered), and recurring tasks that are not due yet.
pub fn eligible<'a>(tasks: &'a [Task], agent: &str, today: NaiveDate) -> Vec<&'a Task> {
    let ready: Vec<&Task> = tasks
        .iter()
        .filter(|t| t.status == Status::Open)
        .filter(|t| t.agent.as_deref().is_some_and(|a| a.eq_ignore_ascii_case(agent)))
        .filter(|t| !matches!(t.work_status, Some(WorkStatus::WaitingForReview | WorkStatus::Complete | WorkStatus::NeedsInput)))
        .filter(|t| t.recurrence.is_none() || t.due_date.is_none_or(|d| d <= today))
        .collect();
    crate::commands::sort_by_due_then_priority(ready)
}

pub struct QueueEntry<'a> {
    pub task: &'a Task,
    pub pickup: Pickup,
    pub review_thread: Option<String>,
    pub question: Option<String>,
    /// Linked notes other than the review thread and question.
    pub notes: Vec<crate::note::Note>,
}

pub fn queue_entry<'a>(task: &'a Task, notes_dir: &Path) -> QueueEntry<'a> {
    let review_slug = review_thread_slug(task.id);
    let question_slug = question_slug(task.id);
    let review_thread = read_body(notes_dir, &review_slug);
    let question = read_body(notes_dir, &question_slug);
    let notes = task
        .notes
        .iter()
        .filter(|s| **s != review_slug && **s != question_slug)
        .filter_map(|s| crate::note::read_note(&notes_dir.join(format!("{}.md", s))).ok())
        .collect();
    QueueEntry { pickup: pickup_kind(task, question.as_deref()), task, review_thread, question, notes }
}

pub fn entry_json(e: &QueueEntry) -> Value {
    json!({
        "task": e.task,
        "pickup": e.pickup.as_str(),
        "review_thread": e.review_thread,
        "question": e.question,
        "notes": e.notes.iter().map(|n| json!({"slug": n.slug, "title": n.title, "body": n.body})).collect::<Vec<_>>(),
    })
}

/// The whole work queue for `agent`: its standing instructions and memory plus the
/// tasks to work, ready to hand to any harness.
pub fn queue_json(agent: &str, notes_dir: &Path, tasks: &[&Task], limit: Option<usize>) -> Value {
    let entries: Vec<Value> =
        tasks.iter().take(limit.unwrap_or(usize::MAX)).map(|t| entry_json(&queue_entry(t, notes_dir))).collect();
    json!({
        "agent": agent,
        "instructions": read_instructions(notes_dir, agent),
        "memory": read_memory(notes_dir, agent),
        "tasks": entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Priority, Recurrence};
    use chrono::Utc;
    use std::fs;
    use tempfile::tempdir;

    fn task(id: u32, agent: &str) -> Task {
        Task {
            id,
            title: format!("task {}", id),
            status: Status::Open,
            priority: Priority::Medium,
            tags: vec![],
            created: Utc::now(),
            updated: None,
            description: None,
            instructions: None,
            due_date: None,
            project: None,
            recurrence: None,
            notes: vec![],
            agent: Some(agent.to_string()),
            effort: None,
            work_status: None,
        }
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 6).unwrap()
    }

    #[test]
    fn canonical_name_ignores_case_and_returns_profile_spelling() {
        let profiles = vec![("Follow-up".to_string(), "/code/f".to_string())];
        assert_eq!(canonical_name(&profiles, "follow-up"), Some("Follow-up".to_string()));
        assert_eq!(canonical_name(&profiles, "FOLLOW-UP"), Some("Follow-up".to_string()));
        assert_eq!(canonical_name(&profiles, "other"), None);
        assert_eq!(resolve_name(&profiles, "other"), "other");
    }

    #[test]
    fn identity_prefers_env_over_working_directory() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.md");
        fs::write(&config, "agent-Writer: /code/writer\nagent-Reviewer: /code/reviewer\n").unwrap();

        let id = identity(Some("reviewer"), &config, Path::new("/code/writer/src")).unwrap();
        assert_eq!((id.name.as_str(), id.source), ("Reviewer", Source::Env));
        assert_eq!(id.dir.as_deref(), Some("/code/reviewer"));
    }

    #[test]
    fn identity_falls_back_to_working_directory_and_then_none() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.md");
        fs::write(&config, "agent-Writer: /code/writer\n").unwrap();

        let id = identity(None, &config, Path::new("/code/writer/src")).unwrap();
        assert_eq!((id.name.as_str(), id.source), ("Writer", Source::WorkingDirectory));
        assert!(identity(None, &config, Path::new("/elsewhere")).is_none());
        assert!(identity(Some("  "), &config, Path::new("/elsewhere")).is_none());
    }

    #[test]
    fn identity_accepts_an_env_name_with_no_profile() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.md");
        fs::write(&config, "").unwrap();
        let id = identity(Some("adhoc"), &config, Path::new("/")).unwrap();
        assert_eq!((id.name.as_str(), id.dir), ("adhoc", None));
    }

    #[test]
    fn eligible_applies_the_work_agent_tasks_skip_rules() {
        let mut waiting = task(1, "Bot");
        waiting.work_status = Some(WorkStatus::WaitingForReview);
        let mut complete = task(2, "Bot");
        complete.work_status = Some(WorkStatus::Complete);
        let mut blocked = task(3, "Bot");
        blocked.work_status = Some(WorkStatus::NeedsInput);
        let mut future_recurring = task(4, "Bot");
        future_recurring.recurrence = Some("daily".parse::<Recurrence>().unwrap());
        future_recurring.due_date = Some(today() + chrono::Duration::days(2));
        let mut due_recurring = task(5, "Bot");
        due_recurring.recurrence = Some("daily".parse::<Recurrence>().unwrap());
        due_recurring.due_date = Some(today());
        let mut changes = task(6, "Bot");
        changes.work_status = Some(WorkStatus::ChangesRequested);
        let mut done = task(7, "Bot");
        done.status = Status::Done;
        let other = task(8, "Someone");
        let plain = task(9, "bot"); // different case, same agent

        let tasks = vec![waiting, complete, blocked, future_recurring, due_recurring, changes, done, other, plain];
        let ids: Vec<u32> = eligible(&tasks, "Bot", today()).iter().map(|t| t.id).collect();
        assert_eq!(ids, vec![5, 6, 9]);
    }

    #[test]
    fn eligible_orders_by_due_date_then_priority_with_undated_last() {
        let mut a = task(1, "Bot");
        a.priority = Priority::Low;
        a.due_date = Some(today());
        let mut b = task(2, "Bot");
        b.priority = Priority::Critical;
        b.due_date = Some(today());
        let mut c = task(3, "Bot");
        c.priority = Priority::Critical;
        let mut d = task(4, "Bot");
        d.priority = Priority::Low;
        d.due_date = Some(today() - chrono::Duration::days(3));
        let tasks = vec![a, b, c, d];
        let ids: Vec<u32> = eligible(&tasks, "Bot", today()).iter().map(|t| t.id).collect();
        assert_eq!(ids, vec![4, 2, 1, 3]);
    }

    #[test]
    fn pickup_distinguishes_feedback_from_an_answered_question() {
        let mut t = task(1, "Bot");
        assert_eq!(pickup_kind(&t, None), Pickup::New);
        t.work_status = Some(WorkStatus::InProgress);
        assert_eq!(pickup_kind(&t, None), Pickup::Continue);
        t.work_status = Some(WorkStatus::ChangesRequested);
        assert_eq!(pickup_kind(&t, None), Pickup::ReviewFeedback);
        assert_eq!(pickup_kind(&t, Some("body\n\n## Answer — 2026-10-06\n\nyes")), Pickup::AnsweredQuestion);
        // A question that is still the newest section has not been answered.
        assert_eq!(pickup_kind(&t, Some("## Answer — 2026-10-05\n\nold\n\n## Question\n\nagain?")), Pickup::ReviewFeedback);
    }

    #[test]
    fn queue_entry_bundles_review_thread_question_and_other_notes() {
        let dir = tempdir().unwrap();
        let write = |slug: &str, title: &str, body: &str| {
            crate::note::write_note(dir.path(), &crate::note::Note { slug: slug.into(), title: title.into(), body: body.into() }).unwrap();
        };
        write("task-1-review-thread", "Review", "## Feedback — 2026-10-06\n\nfix it");
        write("task-1-question", "Question", "which one?\n\n## Answer — 2026-10-06\n\nthe first");
        write("how-to", "How to", "steps");

        let mut t = task(1, "Bot");
        t.work_status = Some(WorkStatus::ChangesRequested);
        t.notes = vec!["task-1-review-thread".into(), "task-1-question".into(), "how-to".into(), "missing".into()];

        let e = queue_entry(&t, dir.path());
        assert_eq!(e.pickup, Pickup::AnsweredQuestion);
        assert!(e.review_thread.unwrap().contains("fix it"));
        assert!(e.question.unwrap().contains("the first"));
        assert_eq!(e.notes.iter().map(|n| n.slug.as_str()).collect::<Vec<_>>(), vec!["how-to"]);
    }

    #[test]
    fn queue_json_has_a_stable_shape_and_honours_limit() {
        let dir = tempdir().unwrap();
        let tasks = vec![task(1, "Bot"), task(2, "Bot")];
        let refs: Vec<&Task> = tasks.iter().collect();
        let v = queue_json("Bot", dir.path(), &refs, Some(1));
        assert_eq!(v["agent"], "Bot");
        assert!(v["instructions"].is_null() && v["memory"].is_null());
        assert_eq!(v["tasks"].as_array().unwrap().len(), 1);
        let first = &v["tasks"][0];
        assert_eq!(first["task"]["id"], 1);
        assert_eq!(first["pickup"], "new");
        assert!(first["review_thread"].is_null() && first["question"].is_null());
        assert_eq!(first["notes"], json!([]));
    }
}
