//! The per-task conversation between the human and an agent.
//!
//! One append-only markdown note, `task-<id>-thread`. Every section is
//! `## <Kind> — <YYYY-MM-DD>`; the kind is read from the heading. Older tasks
//! kept two notes (`task-<id>-review-thread`, `task-<id>-question`); those are
//! merged into the thread losslessly by [`migrate`].

use std::path::Path;

use chrono::NaiveDate;
use serde::Serialize;

use crate::lifecycle::{self, Actor, Event, State};
use crate::task::Task;

pub fn slug(task_id: u32) -> String {
    format!("task-{}-thread", task_id)
}

pub fn legacy_review_slug(task_id: u32) -> String {
    format!("task-{}-review-thread", task_id)
}

pub fn legacy_question_slug(task_id: u32) -> String {
    format!("task-{}-question", task_id)
}

/// Which old note a slug names, and for which task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Legacy {
    Review(u32),
    Question(u32),
}

pub fn legacy_of(slug: &str) -> Option<Legacy> {
    let rest = slug.strip_prefix("task-")?;
    let (id, tail) = rest.split_once('-')?;
    let id: u32 = id.parse().ok()?;
    match tail {
        "review-thread" => Some(Legacy::Review(id)),
        "question" => Some(Legacy::Question(id)),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Agent,
    Question,
    Answer,
    Feedback,
    Message,
    Approval,
    Snapshot,
}

impl Kind {
    pub fn heading(self) -> &'static str {
        match self {
            Kind::Agent => "Agent",
            Kind::Question => "Question",
            Kind::Answer => "Answer",
            Kind::Feedback => "Feedback",
            Kind::Message => "Message",
            Kind::Approval => "Approval",
            Kind::Snapshot => "Snapshot",
        }
    }

    /// Written by the human (or the system on their behalf), as opposed to the agent.
    pub fn is_human(self) -> bool {
        matches!(self, Kind::Answer | Kind::Feedback | Kind::Message | Kind::Approval)
    }

    fn from_heading(heading: &str) -> Kind {
        let lower = heading.trim().to_lowercase();
        let word: String = lower.chars().take_while(|c| c.is_alphabetic()).collect();
        match word.as_str() {
            "agent" | "response" => Kind::Agent,
            "question" => Kind::Question,
            "answer" => Kind::Answer,
            "feedback" => Kind::Feedback,
            "message" => Kind::Message,
            "approval" => Kind::Approval,
            "snapshot" => Kind::Snapshot,
            // Unknown headings read as the human, as the web UI has always shown them.
            _ => Kind::Feedback,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub kind: Kind,
    pub date: Option<String>,
    /// The section as written, heading included, trimmed.
    pub raw: String,
    /// Text under the heading, trimmed.
    pub body: String,
}

fn heading_date(heading: &str) -> Option<String> {
    let tail = heading.split_once(['—', '–']).or_else(|| heading.split_once(" - ")).map(|(_, t)| t.trim())?;
    (!tail.is_empty()).then(|| tail.to_string())
}

fn render_section(kind: Kind, date: &str, text: &str) -> String {
    format!("## {} — {}\n\n{}", kind.heading(), date, text.trim())
}

/// Splits a thread body into leading text (before the first `## `) and sections.
pub fn parse(body: &str) -> (String, Vec<Section>) {
    let mut preamble = String::new();
    let mut chunks: Vec<Vec<&str>> = Vec::new();
    let mut in_fence = false;
    for line in body.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
        }
        if !in_fence && line.starts_with("## ") {
            chunks.push(vec![line]);
        } else if let Some(cur) = chunks.last_mut() {
            cur.push(line);
        } else {
            preamble.push_str(line);
            preamble.push('\n');
        }
    }
    let sections = chunks
        .into_iter()
        .map(|lines| {
            let heading = lines[0].trim_start_matches("## ").trim();
            let text = lines[1..].join("\n").trim().to_string();
            Section {
                kind: Kind::from_heading(heading),
                date: heading_date(heading),
                raw: lines.join("\n").trim().to_string(),
                body: text,
            }
        })
        .collect();
    (preamble.trim().to_string(), sections)
}

/// A thread is unread when its last section (ignoring snapshots) is the human's.
pub fn unread(sections: &[Section]) -> bool {
    sections.iter().rev().find(|s| s.kind != Kind::Snapshot).is_some_and(|s| s.kind.is_human())
}

pub fn last_human_kind(sections: &[Section]) -> Option<Kind> {
    sections.iter().rev().find(|s| s.kind != Kind::Snapshot).map(|s| s.kind).filter(|k| k.is_human())
}

// ---------------------------------------------------------------------------
// What the human may do
// ---------------------------------------------------------------------------

/// What the composer offers right now. Derived from the lifecycle, never guessed by the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Allowed {
    /// `answer`, `feedback` or `message`; `None` when sending is not possible.
    pub send: Option<&'static str>,
    pub approve: bool,
}

pub fn allowed(t: &Task) -> Allowed {
    let none = Allowed { send: None, approve: false };
    if t.is_service() {
        return none;
    }
    let ok = |e: Event| lifecycle::check(t, e, Actor::Human).is_ok();
    let agent_owned = t.agent.as_deref().is_some_and(|a| !a.eq_ignore_ascii_case("human"));
    match lifecycle::state(t) {
        State::NeedsInput if ok(Event::Answer) => Allowed { send: Some("answer"), approve: false },
        State::WaitingForReview => Allowed {
            send: ok(Event::Feedback).then_some("feedback"),
            approve: ok(Event::Done),
        },
        State::Done if ok(Event::Feedback) => Allowed { send: Some("feedback"), approve: false },
        // The agent is already on it, so a note to it needs no owner check.
        State::ChangesRequested => Allowed { send: Some("message"), approve: false },
        State::Todo | State::InProgress if agent_owned => Allowed { send: Some("message"), approve: false },
        _ => none,
    }
}

// ---------------------------------------------------------------------------
// Reading, merging and migrating
// ---------------------------------------------------------------------------

fn note_path(dir: &Path, slug: &str) -> std::path::PathBuf {
    dir.join(format!("{}.md", slug))
}

fn file_date(path: &Path) -> String {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|t| chrono::DateTime::<chrono::Local>::from(t).format("%Y-%m-%d").to_string())
        .unwrap_or_else(|_| chrono::Local::now().format("%Y-%m-%d").to_string())
}

/// Sections of a legacy note. Text before the first heading (the question an agent
/// wrote as a bare body) becomes one section of `default_kind`, dated from the file.
fn legacy_sections(dir: &Path, slug: &str, default_kind: Kind) -> Option<Vec<Section>> {
    let path = note_path(dir, slug);
    let note = crate::note::read_note(&path).ok()?;
    let (preamble, mut sections) = parse(&note.body);
    if !preamble.is_empty() {
        let raw = render_section(default_kind, &file_date(&path), &preamble);
        sections.insert(0, Section { kind: default_kind, date: Some(file_date(&path)), raw, body: preamble });
    }
    Some(sections)
}

fn date_key(s: &Section, carried: &mut String) -> String {
    if let Some(d) = s.date.as_deref().filter(|d| {
        let b = d.as_bytes();
        b.len() >= 10 && b[4] == b'-' && b[7] == b'-' && b[..4].iter().all(u8::is_ascii_digit)
    }) {
        *carried = d[..10].to_string();
    }
    carried.clone()
}

/// The thread body combining the thread note (if any) with both legacy notes.
/// Returns `None` when there is nothing at all.
struct Merged {
    body: String,
    legacy: Vec<(String, Vec<Section>)>,
    has_legacy: bool,
}

fn merge(dir: &Path, task_id: u32) -> Option<Merged> {
    let thread = crate::note::read_note(&note_path(dir, &slug(task_id))).ok();
    let legacy_defs = [(legacy_review_slug(task_id), Kind::Agent), (legacy_question_slug(task_id), Kind::Question)];
    let legacy: Vec<(String, Vec<Section>)> = legacy_defs
        .iter()
        .filter_map(|(s, k)| legacy_sections(dir, s, *k).map(|secs| (s.clone(), secs)))
        .collect();
    if thread.is_none() && legacy.is_empty() {
        return None;
    }

    // Existing thread text first; a headingless thread body is kept as an agent message.
    let mut items: Vec<Section> = Vec::new();
    if let Some(n) = &thread {
        let (pre, secs) = parse(&n.body);
        if !pre.is_empty() {
            items.push(Section {
                kind: Kind::Agent,
                date: None,
                raw: format!("## Agent\n\n{}", pre),
                body: pre,
            });
        }
        items.extend(secs);
    }
    // Legacy sections already in the thread (same kind and body) are not added twice.
    let mut present: Vec<(Kind, String)> = items.iter().map(|s| (s.kind, s.body.clone())).collect();
    for (_, secs) in &legacy {
        for s in secs {
            if let Some(i) = present.iter().position(|(k, b)| *k == s.kind && *b == s.body) {
                present.remove(i);
            } else {
                items.push(s.clone());
            }
        }
    }

    // Chronological, stable: ties keep thread-then-review-then-question order.
    let mut carried = String::new();
    let mut keyed: Vec<(String, usize, Section)> =
        items.into_iter().enumerate().map(|(i, s)| (date_key(&s, &mut carried), i, s)).collect();
    keyed.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let body = keyed.iter().map(|(_, _, s)| s.raw.as_str()).collect::<Vec<_>>().join("\n\n");
    let has_legacy = !legacy.is_empty();
    Some(Merged { body, legacy, has_legacy })
}

/// Every section of every legacy note must appear, kind and body unchanged, in `body`.
fn verify(body: &str, legacy: &[(String, Vec<Section>)]) -> Result<(), String> {
    let (_, sections) = parse(body);
    let mut pool: Vec<(Kind, &str)> = sections.iter().map(|s| (s.kind, s.body.as_str())).collect();
    for (slug, secs) in legacy {
        for s in secs {
            match pool.iter().position(|(k, b)| *k == s.kind && *b == s.body) {
                Some(i) => {
                    pool.remove(i);
                }
                None => return Err(format!("thread migration check failed: a section of {} is missing from the merge", slug)),
            }
        }
    }
    Ok(())
}

/// The thread as the reader should see it, without touching any file.
pub struct View {
    pub body: String,
    pub sections: Vec<Section>,
}

impl View {
    pub fn unread(&self) -> bool {
        unread(&self.sections)
    }
}

pub fn view(dir: &Path, task_id: u32) -> Option<View> {
    let m = merge(dir, task_id)?;
    let (_, sections) = parse(&m.body);
    Some(View { body: m.body, sections })
}

/// Slugs of old notes still on disk for this task.
pub fn pending_legacy(dir: &Path, task_id: u32) -> Vec<String> {
    [legacy_review_slug(task_id), legacy_question_slug(task_id)]
        .into_iter()
        .filter(|s| note_path(dir, s).is_file())
        .collect()
}

/// Merge the old notes into the thread note and relink the task. Returns the old
/// slugs to hand to [`archive`] once the caller has committed the task.
///
/// Nothing is written unless the merged thread contains every old section verbatim.
pub fn migrate(dir: &Path, task: &mut Task) -> Result<Vec<String>, String> {
    let Some(m) = merge(dir, task.id) else { return Ok(vec![]) };
    if !m.has_legacy {
        return Ok(vec![]);
    }
    verify(&m.body, &m.legacy)?;
    let note = crate::note::Note { slug: slug(task.id), title: format!("Conversation — {}", task.title), body: m.body };
    crate::note::write_note(dir, &note)?;
    // The written file must read back to the same sections before anything is retired.
    let written = crate::note::read_note(&note_path(dir, &note.slug))?;
    verify(&written.body, &m.legacy)?;

    let archived: Vec<String> = m.legacy.iter().map(|(s, _)| s.clone()).collect();
    task.notes.retain(|s| !archived.contains(s));
    if !task.notes.contains(&note.slug) {
        task.notes.push(note.slug);
    }
    Ok(archived)
}

/// Move retired notes to `<notes>/.migrated/` (never deleted), after the task is committed.
pub fn archive(dir: &Path, slugs: &[String]) -> Result<(), String> {
    if slugs.is_empty() {
        return Ok(());
    }
    let dest_dir = dir.join(".migrated");
    std::fs::create_dir_all(&dest_dir).map_err(|e| format!("Failed to create {}: {}", dest_dir.display(), e))?;
    for s in slugs {
        let from = note_path(dir, s);
        if !from.is_file() {
            continue;
        }
        let mut to = dest_dir.join(format!("{}.md", s));
        let mut n = 1;
        while to.exists() {
            to = dest_dir.join(format!("{}-{}.md", s, n));
            n += 1;
        }
        std::fs::rename(&from, &to).map_err(|e| format!("Failed to move {}: {}", from.display(), e))?;
    }
    Ok(())
}

/// Append a section to the task's thread, creating and linking the note, and
/// migrating old notes first. Returns old slugs to [`archive`] after the commit.
pub fn post(dir: &Path, task: &mut Task, kind: Kind, text: &str, date: NaiveDate) -> Result<Vec<String>, String> {
    let archived = migrate(dir, task)?;
    let thread = slug(task.id);
    if !note_path(dir, &thread).is_file() {
        crate::note::write_note(
            dir,
            &crate::note::Note { slug: thread.clone(), title: format!("Conversation — {}", task.title), body: String::new() },
        )?;
    }
    let section = render_section(kind, &date.format("%Y-%m-%d").to_string(), text);
    crate::note::append_to_note(dir, &thread, &section)?;
    if !task.notes.contains(&thread) {
        task.notes.push(thread);
    }
    // The note is a file, so touching the task is what tells live viewers something changed.
    task.updated = Some(chrono::Utc::now());
    Ok(archived)
}

/// Append text an old-style caller wrote for a legacy note. Text that already opens with a
/// `## ` heading is kept as written; anything else becomes a section of `default_kind`.
pub fn post_raw(dir: &Path, task: &mut Task, default_kind: Kind, text: &str, date: NaiveDate) -> Result<Vec<String>, String> {
    if text.trim_start().starts_with("## ") {
        let archived = migrate(dir, task)?;
        let thread = slug(task.id);
        if !note_path(dir, &thread).is_file() {
            crate::note::write_note(
                dir,
                &crate::note::Note { slug: thread.clone(), title: format!("Conversation — {}", task.title), body: String::new() },
            )?;
        }
        crate::note::append_to_note(dir, &thread, text.trim())?;
        if !task.notes.contains(&thread) {
            task.notes.push(thread);
        }
        Ok(archived)
    } else {
        post(dir, task, default_kind, text, date)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Priority, Status, TaskKind, WorkStatus};
    use tempfile::tempdir;

    fn task(agent: Option<&str>, ws: Option<WorkStatus>) -> Task {
        Task {
            id: 1,
            title: "T".into(),
            status: Status::Open,
            priority: Priority::Medium,
            tags: vec![],
            created: chrono::Utc::now(),
            updated: None,
            description: None,
            instructions: None,
            due_date: None,
            project: None,
            recurrence: None,
            notes: vec![],
            agent: agent.map(String::from),
            effort: None,
            work_status: ws,
            kind: TaskKind::Task,
            paused: false,
        }
    }

    fn put(dir: &Path, slug: &str, body: &str) {
        crate::note::write_note(dir, &crate::note::Note { slug: slug.into(), title: slug.into(), body: body.into() }).unwrap();
    }

    #[test]
    fn parse_reads_kinds_dates_and_ignores_headings_in_fences() {
        let (pre, s) = parse("lead\n\n## Agent response — 2026-10-01\n\ndone\n\n## Feedback — 2026-10-02\n\n```\n## not a heading\n```\nmore");
        assert_eq!(pre, "lead");
        assert_eq!(s.len(), 2);
        assert_eq!((s[0].kind, s[0].date.as_deref()), (Kind::Agent, Some("2026-10-01")));
        assert_eq!(s[1].kind, Kind::Feedback);
        assert!(s[1].body.contains("## not a heading"));
    }

    #[test]
    fn unread_is_true_only_when_the_human_spoke_last() {
        let (_, s) = parse("## Agent — 2026-10-01\n\na\n\n## Message — 2026-10-02\n\nm\n\n## Snapshot — 2026-10-02\n\nx");
        assert!(unread(&s));
        let (_, s) = parse("## Message — 2026-10-02\n\nm\n\n## Agent — 2026-10-03\n\nok");
        assert!(!unread(&s));
        let (_, s) = parse("## Agent — 2026-10-03\n\nQ\n\n## Question — 2026-10-03\n\nwhich?");
        assert!(!unread(&s));
        assert!(!unread(&[]));
    }

    #[test]
    fn allowed_follows_the_lifecycle() {
        let a = |t: &Task| allowed(t);
        assert_eq!(a(&task(Some("Bot"), Some(WorkStatus::NeedsInput))), Allowed { send: Some("answer"), approve: false });
        assert_eq!(a(&task(Some("Bot"), Some(WorkStatus::WaitingForReview))), Allowed { send: Some("feedback"), approve: true });
        assert_eq!(a(&task(Some("Bot"), Some(WorkStatus::InProgress))).send, Some("message"));
        assert_eq!(a(&task(Some("Bot"), Some(WorkStatus::ChangesRequested))).send, Some("message"));
        assert_eq!(a(&task(None, Some(WorkStatus::ChangesRequested))).send, Some("message"));
        assert_eq!(a(&task(Some("Bot"), None)).send, Some("message"));
        // Human-owned or unassigned tasks have nobody to message.
        assert_eq!(a(&task(Some("human"), None)).send, None);
        assert_eq!(a(&task(None, Some(WorkStatus::InProgress))).send, None);
        let mut done = task(Some("Bot"), None);
        done.status = Status::Done;
        assert_eq!(a(&done), Allowed { send: Some("feedback"), approve: false });
        let mut svc = task(Some("Bot"), None);
        svc.kind = TaskKind::Service;
        assert_eq!(a(&svc), Allowed { send: None, approve: false });
    }

    #[test]
    fn legacy_slugs_are_recognised() {
        assert_eq!(legacy_of("task-12-question"), Some(Legacy::Question(12)));
        assert_eq!(legacy_of("task-3-review-thread"), Some(Legacy::Review(3)));
        assert_eq!(legacy_of("task-3-thread"), None);
        assert_eq!(legacy_of("task-x-question"), None);
    }

    #[test]
    fn migrate_merges_both_notes_by_date_and_keeps_every_section() {
        let dir = tempdir().unwrap();
        put(dir.path(), "task-1-review-thread", "## Snapshot — 2026-10-01\n\npinned\n\n## Feedback — 2026-10-04\n\nfix it\n\n## Agent response — 2026-10-05\n\nfixed");
        put(dir.path(), "task-1-question", "which one?\n\n## Answer — 2026-10-03\n\nthe first");
        let mut t = task(Some("Bot"), Some(WorkStatus::ChangesRequested));
        t.notes = vec!["task-1-review-thread".into(), "task-1-question".into(), "other".into()];

        let archived = migrate(dir.path(), &mut t).unwrap();
        assert_eq!(archived, vec!["task-1-review-thread".to_string(), "task-1-question".to_string()]);
        assert_eq!(t.notes, vec!["other".to_string(), "task-1-thread".to_string()]);

        let body = crate::note::read_note(&note_path(dir.path(), "task-1-thread")).unwrap().body;
        let (_, s) = parse(&body);
        let kinds: Vec<Kind> = s.iter().map(|s| s.kind).collect();
        // The bare question body is dated from the file (today), so it sorts after the 2026 dates.
        assert_eq!(kinds, vec![Kind::Snapshot, Kind::Answer, Kind::Feedback, Kind::Agent, Kind::Question]);
        assert!(body.contains("which one?") && body.contains("the first") && body.contains("fix it") && body.contains("pinned"));
    }

    #[test]
    fn archive_moves_instead_of_deleting_and_migrate_is_idempotent() {
        let dir = tempdir().unwrap();
        put(dir.path(), "task-1-question", "## Question — 2026-10-01\n\nq?\n\n## Answer — 2026-10-02\n\na");
        let mut t = task(Some("Bot"), None);
        let archived = migrate(dir.path(), &mut t).unwrap();
        archive(dir.path(), &archived).unwrap();
        assert!(!note_path(dir.path(), "task-1-question").exists());
        assert!(dir.path().join(".migrated/task-1-question.md").is_file());
        assert!(migrate(dir.path(), &mut t).unwrap().is_empty());

        // A crash between commit and archive leaves old notes behind: re-running adds nothing twice.
        put(dir.path(), "task-1-question", "## Question — 2026-10-01\n\nq?\n\n## Answer — 2026-10-02\n\na");
        let before = crate::note::read_note(&note_path(dir.path(), "task-1-thread")).unwrap().body;
        let again = migrate(dir.path(), &mut t).unwrap();
        archive(dir.path(), &again).unwrap();
        let after = crate::note::read_note(&note_path(dir.path(), "task-1-thread")).unwrap().body;
        assert_eq!(before, after);
        assert!(dir.path().join(".migrated/task-1-question-1.md").is_file());
    }

    #[test]
    fn view_merges_without_writing() {
        let dir = tempdir().unwrap();
        put(dir.path(), "task-1-review-thread", "## Feedback — 2026-10-04\n\nfix it");
        let v = view(dir.path(), 1).unwrap();
        assert!(v.unread());
        assert!(note_path(dir.path(), "task-1-review-thread").is_file());
        assert!(!note_path(dir.path(), "task-1-thread").exists());
        assert!(view(dir.path(), 2).is_none());
    }

    #[test]
    fn post_creates_links_and_appends() {
        let dir = tempdir().unwrap();
        let mut t = task(Some("Bot"), Some(WorkStatus::InProgress));
        let d = NaiveDate::from_ymd_opt(2026, 10, 30).unwrap();
        post(dir.path(), &mut t, Kind::Message, "hello", d).unwrap();
        post(dir.path(), &mut t, Kind::Agent, "hi", d).unwrap();
        assert_eq!(t.notes, vec!["task-1-thread".to_string()]);
        let v = view(dir.path(), 1).unwrap();
        assert_eq!(v.sections.len(), 2);
        assert!(!v.unread());
        assert!(v.body.contains("## Message — 2026-10-30\n\nhello"));
    }
}
