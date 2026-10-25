//! The task and service lifecycle: the one place that decides which state
//! moves are valid and who may make them. The CLI, the web API and the TUI
//! call [`apply`] (or [`complete`] for done) and never write `status`,
//! `work_status`, `kind` or `paused` themselves.

use chrono::{NaiveDate, Utc};
use serde::Serialize;

use crate::task::{Status, Task, TaskFile, TaskKind, WorkStatus};

/// Who is making a move. The CLI is an agent when `TASK_AGENT` is set; the web
/// UI and the TUI are always the human.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Actor {
    Human,
    Agent,
}

/// One derived state per task, computed from the stored fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum State {
    Todo,
    InProgress,
    NeedsInput,
    WaitingForReview,
    ChangesRequested,
    Done,
    Scheduled,
    Running,
    Failing,
}

pub fn state(t: &Task) -> State {
    if t.is_service() {
        return match t.work_status {
            Some(WorkStatus::InProgress) => State::Running,
            Some(WorkStatus::Failing) => State::Failing,
            _ => State::Scheduled,
        };
    }
    if t.status == Status::Done {
        return State::Done;
    }
    match t.work_status {
        Some(WorkStatus::InProgress) => State::InProgress,
        Some(WorkStatus::NeedsInput) => State::NeedsInput,
        Some(WorkStatus::WaitingForReview) => State::WaitingForReview,
        Some(WorkStatus::ChangesRequested) => State::ChangesRequested,
        None | Some(WorkStatus::Todo) | Some(WorkStatus::Failing) => State::Todo,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Event {
    Start,
    Ask,
    Answer,
    Submit,
    Feedback,
    Done,
    Reopen,
    Reset,
    /// A review task's due date was pushed later: the human takes it back.
    Defer,
    RunOk,
    RunFailed,
    Pause,
    Resume,
    ToService,
    ToTask,
}

pub const ALL_EVENTS: [Event; 15] = [
    Event::Start,
    Event::Ask,
    Event::Answer,
    Event::Submit,
    Event::Feedback,
    Event::Done,
    Event::Reopen,
    Event::Reset,
    Event::Defer,
    Event::RunOk,
    Event::RunFailed,
    Event::Pause,
    Event::Resume,
    Event::ToService,
    Event::ToTask,
];

impl Event {
    fn verb(self) -> &'static str {
        match self {
            Event::Start => "start",
            Event::Ask => "ask a question on",
            Event::Answer => "answer a question on",
            Event::Submit => "submit",
            Event::Feedback => "post review feedback on",
            Event::Done => "mark done",
            Event::Reopen => "reopen",
            Event::Reset => "reset",
            Event::Defer => "defer",
            Event::RunOk | Event::RunFailed => "record a run of",
            Event::Pause => "pause",
            Event::Resume => "resume",
            Event::ToService => "convert to a service",
            Event::ToTask => "convert to a task",
        }
    }
}

fn is_human_owned(t: &Task) -> bool {
    t.agent.as_deref().is_none_or(|a| a.eq_ignore_ascii_case("human"))
}

/// Actor rules. These are never skipped, not even with `force`.
fn check_actor(t: &Task, event: Event, actor: Actor, force: bool) -> Result<(), String> {
    if force && actor == Actor::Agent {
        return Err("--force is for humans only and is refused when TASK_AGENT is set".to_string());
    }
    match (event, actor) {
        (Event::Ask, Actor::Human) => {
            Err("needs-input is set by the agent when it asks a question; answer it from the task's question instead".to_string())
        }
        (Event::Submit, Actor::Human) => {
            Err("waiting-for-review is set by the agent when it submits its work".to_string())
        }
        (Event::Start, Actor::Human) if !is_human_owned(t) && !t.is_service() => Err(format!(
            "in-progress on a task assigned to {} is set by that agent",
            t.agent.as_deref().unwrap_or_default()
        )),
        (Event::Start | Event::Ask | Event::Submit, Actor::Agent) if is_human_owned(t) && !t.is_service() => {
            Err(format!("task {} is assigned to the human, not an agent", t.id))
        }
        (Event::Start, Actor::Human) if t.is_service() => {
            Err("a service is marked running by its agent".to_string())
        }
        (
            Event::Answer
            | Event::Feedback
            | Event::Done
            | Event::Reopen
            | Event::Reset
            | Event::Defer
            | Event::Pause
            | Event::Resume
            | Event::ToService
            | Event::ToTask,
            Actor::Agent,
        ) => Err(format!("only a human can {} task {} (TASK_AGENT is set)", event.verb(), t.id)),
        _ => Ok(()),
    }
}

/// Transition rules: is `event` valid from the task's current state?
fn check_transition(t: &Task, event: Event) -> Result<(), String> {
    use State as S;
    let s = state(t);
    let refuse = || Err(format!("cannot {} task {}: it is {}", event.verb(), t.id, state_name(s)));

    if t.is_service() {
        return match event {
            Event::Start | Event::RunOk | Event::RunFailed if t.paused => {
                Err(format!("Service {} is paused; resume it first", t.id))
            }
            Event::Start => if matches!(s, S::Scheduled | S::Failing | S::Running) { Ok(()) } else { refuse() },
            Event::Pause if t.paused => Err(format!("Service {} is already paused", t.id)),
            Event::Resume if !t.paused => Err(format!("Service {} is not paused", t.id)),
            Event::RunOk | Event::RunFailed | Event::Pause | Event::Resume | Event::Reset | Event::ToTask => Ok(()),
            Event::Ask => Err(format!("Service {} cannot ask a question; record a failed run instead", t.id)),
            Event::Done => Err(format!(
                "Task {} is a service and is never done: record runs with `task service run`, or pause or delete it",
                t.id
            )),
            Event::ToService => Err(format!("Task {} is already a service", t.id)),
            Event::Submit | Event::Feedback | Event::Answer | Event::Defer | Event::Reopen => {
                Err(format!("Task {} is a service; services are never reviewed", t.id))
            }
        };
    }

    match event {
        Event::Start => if matches!(s, S::Todo | S::InProgress | S::ChangesRequested) { Ok(()) } else { refuse() },
        Event::Ask => if matches!(s, S::Todo | S::InProgress | S::ChangesRequested | S::NeedsInput) { Ok(()) } else { refuse() },
        Event::Submit => {
            if !matches!(s, S::Todo | S::InProgress | S::ChangesRequested | S::WaitingForReview) {
                return refuse();
            }
            if t.notes.is_empty() {
                return Err(format!(
                    "Task {} has no linked note: add one with `task note add \"<title>\" --task {}` before submitting for review",
                    t.id, t.id
                ));
            }
            Ok(())
        }
        Event::Answer => if matches!(s, S::NeedsInput | S::ChangesRequested) { Ok(()) } else { refuse() },
        Event::Feedback => if matches!(s, S::WaitingForReview | S::ChangesRequested | S::Done) { Ok(()) } else { refuse() },
        Event::Done | Event::Reset => if s != S::Done { Ok(()) } else { refuse() },
        Event::Reopen => if s == S::Done { Ok(()) } else { refuse() },
        Event::Defer => if s == S::WaitingForReview { Ok(()) } else { refuse() },
        Event::ToService if s == S::Done => Err(format!("Task {} is done; reopen it before making it a service", t.id)),
        Event::ToService if t.recurrence.is_none() || t.due_date.is_none() => Err(format!(
            "Task {} needs a recurrence and a due date to become a service",
            t.id
        )),
        Event::ToService => Ok(()),
        Event::ToTask => Err(format!("Task {} is already a task", t.id)),
        Event::RunOk | Event::RunFailed | Event::Pause | Event::Resume => Err(format!("Task {} is not a service", t.id)),
    }
}

fn state_name(s: State) -> &'static str {
    match s {
        State::Todo => "todo",
        State::InProgress => "in-progress",
        State::NeedsInput => "needs-input",
        State::WaitingForReview => "waiting-for-review",
        State::ChangesRequested => "changes-requested",
        State::Done => "done",
        State::Scheduled => "scheduled",
        State::Running => "running",
        State::Failing => "failing",
    }
}

/// Is `event` allowed for `actor` right now (without force)?
pub fn check(t: &Task, event: Event, actor: Actor) -> Result<(), String> {
    check_actor(t, event, actor, false)?;
    check_transition(t, event)
}

/// The events `actor` may apply to the task now. The web UI shows controls
/// from this list instead of deciding for itself.
pub fn allowed_events(t: &Task, actor: Actor) -> Vec<Event> {
    ALL_EVENTS.into_iter().filter(|e| check(t, *e, actor).is_ok()).collect()
}

/// Apply one event. `force` (human only) skips the transition rules but never
/// the actor rules, so it can not write an agent-only state. Does not handle
/// `Done`, which may spawn a new task: use [`complete`] for that.
pub fn apply(t: &mut Task, event: Event, actor: Actor, force: bool, today: NaiveDate) -> Result<(), String> {
    check_actor(t, event, actor, force)?;
    if !force {
        check_transition(t, event)?;
    }
    match event {
        Event::Start => t.work_status = Some(WorkStatus::InProgress),
        Event::Ask => t.work_status = Some(WorkStatus::NeedsInput),
        Event::Submit => t.work_status = Some(WorkStatus::WaitingForReview),
        Event::Answer | Event::Feedback => {
            t.status = Status::Open;
            t.work_status = Some(WorkStatus::ChangesRequested);
        }
        Event::Done => {
            t.status = Status::Done;
            t.work_status = None;
        }
        Event::Reopen => {
            t.status = Status::Open;
            t.work_status = None;
        }
        Event::Reset => {
            t.status = Status::Open;
            t.work_status = if t.is_service() { None } else { Some(WorkStatus::Todo) };
        }
        Event::Defer => {
            t.agent = Some("human".to_string());
            t.work_status = None;
        }
        Event::RunOk | Event::RunFailed => {
            let (Some(recur), Some(due)) = (t.recurrence, t.due_date) else {
                return Err(format!("Service {} has no recurrence or due date", t.id));
            };
            t.due_date = Some(crate::task::next_due_after(&recur, due, today, false));
            t.work_status = if event == Event::RunOk { None } else { Some(WorkStatus::Failing) };
        }
        Event::Pause => t.paused = true,
        Event::Resume => {
            t.paused = false;
            if let (Some(recur), Some(due)) = (t.recurrence, t.due_date) {
                t.due_date = Some(crate::task::next_due_after(&recur, due, today, true));
            }
        }
        Event::ToService | Event::ToTask => {
            t.kind = if event == Event::ToService { TaskKind::Service } else { TaskKind::Task };
            t.status = Status::Open;
            t.work_status = if event == Event::ToTask { Some(WorkStatus::Todo) } else { None };
            t.paused = false;
        }
    }
    t.validate_kind()?;
    t.updated = Some(Utc::now());
    Ok(())
}

/// Turn a requested work status (from `task edit --work-status` or the web
/// PATCH) into the matching event, so a status edit follows the same rules as
/// every other move. `None` is an explicit clear.
pub fn event_for_work_status(t: &Task, ws: Option<WorkStatus>) -> Result<Event, String> {
    match ws {
        None | Some(WorkStatus::Todo) => Ok(Event::Reset),
        Some(WorkStatus::InProgress) => Ok(Event::Start),
        Some(WorkStatus::NeedsInput) => Ok(Event::Ask),
        Some(WorkStatus::WaitingForReview) => Ok(Event::Submit),
        Some(WorkStatus::ChangesRequested) => Err(
            "changes-requested is set by posting review feedback or answering the task's question".to_string(),
        ),
        Some(WorkStatus::Failing) if t.is_service() => Err(format!(
            "failing is set by recording a failed run: `task service run {} --result failed`",
            t.id
        )),
        Some(WorkStatus::Failing) => Err("work status 'failing' is only valid on services".to_string()),
    }
}

pub struct Completion {
    pub already_done: bool,
    pub spawned: Option<Task>,
}

/// Mark a task done and, if it recurs, add its next occurrence.
pub fn complete(tf: &mut TaskFile, id: u32, actor: Actor, today: NaiveDate) -> Result<Completion, String> {
    let idx = tf.tasks.iter().position(|t| t.id == id).ok_or_else(|| format!("Task {} not found", id))?;
    check_actor(&tf.tasks[idx], Event::Done, actor, false)?;
    if !tf.tasks[idx].is_service() && tf.tasks[idx].status == Status::Done {
        return Ok(Completion { already_done: true, spawned: None });
    }
    apply(&mut tf.tasks[idx], Event::Done, actor, false, today)?;

    let parent = &tf.tasks[idx];
    let Some(recur) = parent.recurrence else {
        return Ok(Completion { already_done: false, spawned: None });
    };
    let next = Task {
        id: tf.next_id,
        title: parent.title.clone(),
        status: Status::Open,
        priority: parent.priority,
        tags: parent.tags.clone(),
        created: Utc::now(),
        updated: None,
        description: parent.description.clone(),
        instructions: parent.instructions.clone(),
        due_date: Some(crate::task::next_due_date(&recur, parent.due_date)),
        project: parent.project.clone(),
        recurrence: Some(recur),
        notes: parent.notes.clone(),
        agent: parent.agent.clone(),
        effort: parent.effort,
        work_status: None,
        kind: TaskKind::Task,
        paused: false,
    };
    tf.next_id += 1;
    tf.tasks.push(next.clone());
    Ok(Completion { already_done: false, spawned: Some(next) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Priority, Recurrence};

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 24).unwrap()
    }

    fn task(agent: Option<&str>, status: Status, ws: Option<WorkStatus>) -> Task {
        Task {
            id: 1,
            title: "t".into(),
            status,
            priority: Priority::Medium,
            tags: vec![],
            created: Utc::now(),
            updated: None,
            description: None,
            instructions: None,
            due_date: Some(today()),
            project: None,
            recurrence: None,
            notes: vec!["result".into()],
            agent: agent.map(str::to_string),
            effort: None,
            work_status: ws,
            kind: TaskKind::Task,
            paused: false,
        }
    }

    fn service(ws: Option<WorkStatus>, paused: bool) -> Task {
        let mut t = task(Some("Bot"), Status::Open, ws);
        t.kind = TaskKind::Service;
        t.recurrence = Some("daily".parse::<Recurrence>().unwrap());
        t.paused = paused;
        t
    }

    use Actor::{Agent, Human};
    use Event as E;
    use Status::{Done as D, Open as O};
    use WorkStatus as W;

    #[test]
    fn task_moves_allowed() {
        // (agent, status, from work status, event, actor, expected status, expected work status)
        let cases: Vec<(Option<&str>, Status, Option<W>, E, Actor, Status, Option<W>)> = vec![
            (Some("Bot"), O, None, E::Start, Agent, O, Some(W::InProgress)),
            (Some("Bot"), O, Some(W::Todo), E::Start, Agent, O, Some(W::InProgress)),
            (Some("Bot"), O, Some(W::ChangesRequested), E::Start, Agent, O, Some(W::InProgress)),
            (Some("Bot"), O, Some(W::InProgress), E::Ask, Agent, O, Some(W::NeedsInput)),
            (Some("Bot"), O, Some(W::InProgress), E::Submit, Agent, O, Some(W::WaitingForReview)),
            (Some("Bot"), O, Some(W::NeedsInput), E::Answer, Human, O, Some(W::ChangesRequested)),
            (Some("Bot"), O, Some(W::WaitingForReview), E::Feedback, Human, O, Some(W::ChangesRequested)),
            (Some("Bot"), D, None, E::Feedback, Human, O, Some(W::ChangesRequested)),
            (Some("Bot"), O, Some(W::WaitingForReview), E::Done, Human, D, None),
            (Some("Bot"), D, None, E::Reopen, Human, O, None),
            (Some("Bot"), O, Some(W::NeedsInput), E::Reset, Human, O, Some(W::Todo)),
            (Some("Bot"), O, Some(W::WaitingForReview), E::Defer, Human, O, None),
            (None, O, None, E::Start, Human, O, Some(W::InProgress)),
            (Some("human"), O, None, E::Start, Human, O, Some(W::InProgress)),
        ];
        for (agent, status, ws, event, actor, want_status, want_ws) in cases {
            let mut t = task(agent, status, ws);
            apply(&mut t, event, actor, false, today()).unwrap_or_else(|e| panic!("{:?} from {:?}: {}", event, ws, e));
            assert_eq!((t.status, t.work_status), (want_status, want_ws), "{:?} from {:?}", event, ws);
        }
    }

    #[test]
    fn task_moves_refused() {
        let cases: Vec<(Option<&str>, Status, Option<W>, E, Actor)> = vec![
            // actor rules
            (Some("Bot"), O, Some(W::InProgress), E::Ask, Human),
            (Some("Bot"), O, Some(W::InProgress), E::Submit, Human),
            (Some("Bot"), O, None, E::Start, Human),
            (Some("Bot"), O, Some(W::WaitingForReview), E::Done, Agent),
            (Some("Bot"), D, None, E::Reopen, Agent),
            (Some("Bot"), O, Some(W::NeedsInput), E::Answer, Agent),
            (Some("Bot"), O, Some(W::WaitingForReview), E::Feedback, Agent),
            (Some("Bot"), O, None, E::Reset, Agent),
            (Some("Bot"), O, None, E::ToService, Agent),
            // transition rules
            (Some("Bot"), O, Some(W::WaitingForReview), E::Start, Agent),
            (Some("Bot"), O, Some(W::NeedsInput), E::Submit, Agent),
            (Some("Bot"), D, None, E::Start, Agent),
            (Some("Bot"), O, None, E::Answer, Human),
            (Some("Bot"), O, None, E::Feedback, Human),
            (Some("Bot"), O, None, E::Reopen, Human),
            (Some("Bot"), D, None, E::Reset, Human),
            (Some("Bot"), O, None, E::Defer, Human),
            (Some("Bot"), O, None, E::RunOk, Agent),
            (Some("Bot"), O, None, E::Pause, Human),
            (Some("Bot"), O, None, E::ToTask, Human),
            (Some("Bot"), D, None, E::ToService, Human),
        ];
        for (agent, status, ws, event, actor) in cases {
            let mut t = task(agent, status, ws);
            let before = t.clone();
            assert!(apply(&mut t, event, actor, false, today()).is_err(), "{:?} by {:?} from {:?}/{:?} should be refused", event, actor, status, ws);
            assert_eq!(t, before, "a refused move must not change the task");
        }
    }

    #[test]
    fn submit_needs_a_linked_note() {
        let mut t = task(Some("Bot"), O, Some(W::InProgress));
        t.notes.clear();
        assert!(apply(&mut t, E::Submit, Agent, false, today()).unwrap_err().contains("no linked note"));
    }

    #[test]
    fn service_moves() {
        let mut s = service(None, false);
        apply(&mut s, E::RunFailed, Agent, false, today()).unwrap();
        assert_eq!((state(&s), s.due_date), (State::Failing, Some(today().succ_opt().unwrap())));
        apply(&mut s, E::Start, Agent, false, today()).unwrap();
        assert_eq!(state(&s), State::Running);
        apply(&mut s, E::Pause, Human, false, today()).unwrap();
        assert!(s.paused);
        assert!(apply(&mut s.clone(), E::RunOk, Agent, false, today()).unwrap_err().contains("paused"));
        apply(&mut s, E::ToTask, Human, false, today()).unwrap();
        assert_eq!((s.kind, s.paused, state(&s)), (TaskKind::Task, false, State::Todo));
    }

    #[test]
    fn service_refuses_review_and_question_moves() {
        for event in [E::Done, E::Submit, E::Ask, E::Feedback, E::Answer, E::Defer, E::Reopen, E::ToService] {
            let actor = if matches!(event, E::Submit | E::Ask) { Agent } else { Human };
            assert!(apply(&mut service(None, false), event, actor, false, today()).is_err(), "{:?}", event);
        }
        assert!(apply(&mut service(None, false), E::Pause, Agent, false, today()).is_err());
    }

    #[test]
    fn to_service_always_starts_scheduled() {
        let mut t = task(Some("Bot"), O, Some(W::WaitingForReview));
        t.recurrence = Some("weekly".parse::<Recurrence>().unwrap());
        apply(&mut t, E::ToService, Human, false, today()).unwrap();
        assert_eq!((t.kind, t.work_status, state(&t)), (TaskKind::Service, None, State::Scheduled));
        let mut no_recur = task(Some("Bot"), O, None);
        assert!(apply(&mut no_recur, E::ToService, Human, false, today()).is_err());
    }

    #[test]
    fn force_skips_transitions_but_not_actor_rules() {
        let mut t = task(Some("Bot"), O, None);
        apply(&mut t, E::Reopen, Human, true, today()).unwrap();
        // force can not write agent-only states
        assert!(apply(&mut task(Some("Bot"), O, None), E::Submit, Human, true, today()).is_err());
        assert!(apply(&mut task(Some("Bot"), O, None), E::Ask, Human, true, today()).is_err());
        assert!(apply(&mut task(Some("Bot"), O, None), E::Start, Human, true, today()).is_err());
        assert!(apply(&mut task(Some("Bot"), D, None), E::Reopen, Agent, true, today()).is_err());
    }

    #[test]
    fn work_status_edits_map_to_events() {
        let t = task(Some("Bot"), O, None);
        assert_eq!(event_for_work_status(&t, None), Ok(E::Reset));
        assert_eq!(event_for_work_status(&t, Some(W::InProgress)), Ok(E::Start));
        assert_eq!(event_for_work_status(&t, Some(W::WaitingForReview)), Ok(E::Submit));
        assert!(event_for_work_status(&t, Some(W::ChangesRequested)).is_err());
        assert!(event_for_work_status(&service(None, false), Some(W::Failing)).unwrap_err().contains("service run"));
    }

    #[test]
    fn complete_clears_work_status_spawns_and_refuses_agents() {
        let mut tf = TaskFile::new();
        let mut t = task(Some("Bot"), O, Some(W::WaitingForReview));
        t.recurrence = Some("daily".parse::<Recurrence>().unwrap());
        tf.tasks.push(t);
        tf.next_id = 2;
        assert!(complete(&mut tf, 1, Agent, today()).is_err());
        let c = complete(&mut tf, 1, Human, today()).unwrap();
        assert_eq!((tf.tasks[0].status, tf.tasks[0].work_status), (D, None));
        assert_eq!(c.spawned.unwrap().id, 2);
        assert!(complete(&mut tf, 1, Human, today()).unwrap().already_done);
        assert_eq!(tf.tasks.len(), 2);
    }

    #[test]
    fn allowed_events_for_the_human_on_a_review_task() {
        let t = task(Some("Bot"), O, Some(W::WaitingForReview));
        assert_eq!(allowed_events(&t, Human), vec![E::Feedback, E::Done, E::Reset, E::Defer]);
    }

    // -- The full transition matrix --
    //
    // Every fixture below is run through every event, by both actors. `SPEC`
    // lists, by hand, each move that must succeed and the state it must land
    // in; every combination not listed must be refused and leave the task
    // untouched. A rule change that is not reflected here fails the test.

    /// The result of an allowed move: (state, paused).
    type Outcome = (State, bool);

    fn fixtures() -> Vec<(&'static str, Task)> {
        let weekly = || Some("weekly".parse::<Recurrence>().unwrap());
        let bot = |status, ws| {
            let mut t = task(Some("Bot"), status, ws);
            t.recurrence = weekly();
            t
        };
        let human = |agent: Option<&str>, status, ws| {
            let mut t = task(agent, status, ws);
            t.recurrence = weekly();
            t
        };
        vec![
            ("bot-todo", bot(O, None)),
            ("bot-todo-explicit", bot(O, Some(W::Todo))),
            ("bot-in-progress", bot(O, Some(W::InProgress))),
            ("bot-needs-input", bot(O, Some(W::NeedsInput))),
            ("bot-review", bot(O, Some(W::WaitingForReview))),
            ("bot-changes", bot(O, Some(W::ChangesRequested))),
            ("bot-done", bot(D, None)),
            ("human-todo", human(Some("human"), O, None)),
            ("human-in-progress", human(Some("Human"), O, Some(W::InProgress))),
            ("human-done", human(Some("human"), D, None)),
            ("unassigned-todo", human(None, O, None)),
            ("svc-scheduled", service(None, false)),
            ("svc-running", service(Some(W::InProgress), false)),
            ("svc-failing", service(Some(W::Failing), false)),
            ("svc-paused", service(None, true)),
            ("svc-paused-failing", service(Some(W::Failing), true)),
        ]
    }

    /// Every allowed (fixture, actor, event) and where it lands.
    fn spec() -> Vec<(&'static str, Actor, Event, Outcome)> {
        use State as S;
        let mut v = Vec::new();
        let mut allow = |fixtures: &[&'static str], actor, event, to: Outcome| {
            for f in fixtures {
                v.push((*f, actor, event, to));
            }
        };
        let todo = ["bot-todo", "bot-todo-explicit"];
        let open_bot = ["bot-todo", "bot-todo-explicit", "bot-in-progress", "bot-needs-input", "bot-review", "bot-changes"];
        let open_human = ["human-todo", "human-in-progress", "unassigned-todo"];

        // The agent's moves on its own tasks.
        allow(&[&todo[..], &["bot-in-progress", "bot-changes"]].concat(), Agent, E::Start, (S::InProgress, false));
        allow(&[&todo[..], &["bot-in-progress", "bot-changes", "bot-needs-input"]].concat(), Agent, E::Ask, (S::NeedsInput, false));
        allow(&[&todo[..], &["bot-in-progress", "bot-changes", "bot-review"]].concat(), Agent, E::Submit, (S::WaitingForReview, false));

        // The human's moves on agent tasks.
        allow(&["bot-needs-input", "bot-changes"], Human, E::Answer, (S::ChangesRequested, false));
        allow(&["bot-review", "bot-changes", "bot-done", "human-done"], Human, E::Feedback, (S::ChangesRequested, false));
        allow(&[&open_bot[..], &open_human[..]].concat(), Human, E::Done, (S::Done, false));
        allow(&[&open_bot[..], &open_human[..]].concat(), Human, E::Reset, (S::Todo, false));
        allow(&["bot-done", "human-done"], Human, E::Reopen, (S::Todo, false));
        allow(&["bot-review"], Human, E::Defer, (S::Todo, false));
        allow(&[&open_bot[..], &open_human[..]].concat(), Human, E::ToService, (S::Scheduled, false));

        // The human works their own tasks.
        allow(&open_human, Human, E::Start, (S::InProgress, false));

        // Services.
        let active = ["svc-scheduled", "svc-running", "svc-failing"];
        let paused = ["svc-paused", "svc-paused-failing"];
        allow(&active, Agent, E::Start, (S::Running, false));
        for actor in [Agent, Human] {
            allow(&active, actor, E::RunOk, (S::Scheduled, false));
            allow(&active, actor, E::RunFailed, (S::Failing, false));
        }
        allow(&["svc-scheduled"], Human, E::Pause, (S::Scheduled, true));
        allow(&["svc-running"], Human, E::Pause, (S::Running, true));
        allow(&["svc-failing"], Human, E::Pause, (S::Failing, true));
        allow(&["svc-paused"], Human, E::Resume, (S::Scheduled, false));
        allow(&["svc-paused-failing"], Human, E::Resume, (S::Failing, false));
        allow(&active, Human, E::Reset, (S::Scheduled, false));
        allow(&paused, Human, E::Reset, (S::Scheduled, true));
        allow(&[&active[..], &paused[..]].concat(), Human, E::ToTask, (S::Todo, false));
        v
    }

    #[test]
    fn every_move_matches_the_spec() {
        let spec = spec();
        let mut checked = 0;
        for (name, fixture) in fixtures() {
            for actor in [Human, Agent] {
                let allowed = allowed_events(&fixture, actor);
                for event in ALL_EVENTS {
                    let want = spec.iter().find(|(f, a, e, _)| *f == name && *a == actor && *e == event).map(|s| s.3);
                    let mut t = fixture.clone();
                    let got = apply(&mut t, event, actor, false, today());
                    match (want, got) {
                        (Some(to), Ok(())) => assert_eq!((state(&t), t.paused), to, "{name}: {event:?} by {actor:?}"),
                        (Some(_), Err(e)) => panic!("{name}: {event:?} by {actor:?} should be allowed, got: {e}"),
                        (None, Ok(())) => panic!("{name}: {event:?} by {actor:?} should be refused"),
                        (None, Err(e)) => {
                            assert!(!e.is_empty());
                            assert_eq!(t, fixture, "{name}: a refused {event:?} must not change the task");
                        }
                    }
                    // The web UI shows controls from allowed_events, so it must
                    // list exactly the moves that apply would accept.
                    assert_eq!(allowed.contains(&event), want.is_some(), "{name}: allowed_events vs {event:?} by {actor:?}");
                    checked += 1;
                }
            }
        }
        assert_eq!(checked, fixtures().len() * 2 * ALL_EVENTS.len());
        // Every spec row names a real fixture, so a typo can not hide a move.
        let names: Vec<_> = fixtures().into_iter().map(|(n, _)| n).collect();
        for (f, ..) in &spec {
            assert!(names.contains(f), "spec names unknown fixture {f}");
        }
    }

    #[test]
    fn moves_set_the_side_fields() {
        let today = today();
        // Defer hands the task back to the human.
        let mut t = task(Some("Bot"), O, Some(W::WaitingForReview));
        apply(&mut t, E::Defer, Human, false, today).unwrap();
        assert_eq!((t.agent.as_deref(), t.work_status), (Some("human"), None));
        // Reopen and Done clear the work status; Reset on a task sets todo.
        let mut t = task(Some("Bot"), D, None);
        apply(&mut t, E::Reopen, Human, false, today).unwrap();
        assert_eq!((t.status, t.work_status), (O, None));
        // Every applied move stamps `updated`.
        assert!(t.updated.is_some());
        // A run moves the due date to the next scheduled date after today.
        let mut s = service(None, false);
        s.due_date = today.pred_opt();
        apply(&mut s, E::RunOk, Agent, false, today).unwrap();
        assert!(s.due_date.unwrap() > today);
        // Resume moves a passed due date to on or after today.
        let mut s = service(None, true);
        s.due_date = Some(today - chrono::Days::new(10));
        apply(&mut s, E::Resume, Human, false, today).unwrap();
        assert!(s.due_date.unwrap() >= today);
        // Converting a service back to a task clears failing and paused.
        let mut s = service(Some(W::Failing), true);
        apply(&mut s, E::ToTask, Human, false, today).unwrap();
        assert_eq!((s.kind, s.work_status, s.paused), (TaskKind::Task, Some(W::Todo), false));
    }

    #[test]
    fn force_never_lets_anyone_past_the_actor_rules() {
        for (name, fixture) in fixtures() {
            for event in ALL_EVENTS {
                // An agent with --force is always refused.
                assert!(apply(&mut fixture.clone(), event, Agent, true, today()).is_err(), "{name}: forced {event:?} by agent");
                // A forced human move never writes an agent-only state.
                let mut t = fixture.clone();
                if apply(&mut t, event, Human, true, today()).is_ok() {
                    let agent_only = matches!(event, E::Ask | E::Submit)
                        || (event == E::Start && (fixture.is_service() || !is_human_owned(&fixture)));
                    assert!(!agent_only, "{name}: forced {event:?} by human should be refused");
                }
            }
        }
    }

    #[test]
    fn every_work_status_edit_maps_to_an_event_or_a_reason() {
        let t = task(Some("Bot"), O, None);
        let s = service(None, false);
        let all = [None, Some(W::Todo), Some(W::InProgress), Some(W::NeedsInput), Some(W::WaitingForReview), Some(W::ChangesRequested), Some(W::Failing)];
        let want_task = [Ok(E::Reset), Ok(E::Reset), Ok(E::Start), Ok(E::Ask), Ok(E::Submit), Err(()), Err(())];
        for (ws, want) in all.iter().zip(want_task) {
            assert_eq!(event_for_work_status(&t, *ws).map_err(|_| ()), want, "{ws:?} on a task");
        }
        for ws in all {
            let got = event_for_work_status(&s, ws);
            assert_eq!(got.is_err(), matches!(ws, Some(W::ChangesRequested | W::Failing)), "{ws:?} on a service");
        }
    }
}
