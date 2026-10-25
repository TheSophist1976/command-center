//! Integration tests for the task and service lifecycle (`src/lifecycle.rs`),
//! driven through the real `task` binary: the CLI as the human and as an agent
//! (`TASK_AGENT`), and the web API served by `task serve` over HTTP.
//!
//! Every test asserts the stored state afterwards, and that a refused move
//! leaves the task exactly as it was.

use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use task::lifecycle::{self, State};
use task::task::Task;

struct Env {
    _dir: tempfile::TempDir,
    root: PathBuf,
    db: PathBuf,
}

impl Env {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        Env { db: root.join("tasks.db"), root, _dir: dir }
    }

    /// A `task` command isolated from the real config, home and agent identity.
    fn cmd(&self, agent: Option<&str>, args: &[&str]) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_task"));
        c.arg("--file").arg(&self.db).args(args);
        c.current_dir(&self.root)
            .env("HOME", &self.root)
            .env("TASK_CONFIG_FILE", self.root.join("config"))
            .env_remove("TASK_AGENT");
        if let Some(a) = agent {
            c.env("TASK_AGENT", a);
        }
        c
    }

    fn run(&self, agent: Option<&str>, args: &[&str]) -> Output {
        self.cmd(agent, args).output().unwrap()
    }

    /// Run as the human and require success.
    fn human(&self, args: &[&str]) -> String {
        ok(self.run(None, args), args)
    }

    /// Run as `agent` and require success.
    fn agent(&self, agent: &str, args: &[&str]) -> String {
        ok(self.run(Some(agent), args), args)
    }

    /// Run and require a refusal whose message contains `reason`, with the
    /// task unchanged.
    fn refused(&self, agent: Option<&str>, id: u32, args: &[&str], reason: &str) {
        let before = self.task(id);
        let out = self.run(agent, args);
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(!out.status.success(), "{:?} as {:?} should be refused", args, agent);
        assert!(err.contains(reason), "{:?}: expected {:?} in: {}", args, reason, err);
        assert_eq!(self.task(id), before, "{:?}: a refused move must not change the task", args);
    }

    fn task(&self, id: u32) -> Task {
        task::db::load(&self.db).unwrap().find_task(id).cloned().unwrap_or_else(|| panic!("task {} missing", id))
    }

    fn state(&self, id: u32) -> State {
        lifecycle::state(&self.task(id))
    }
}

fn ok(out: Output, args: &[&str]) -> String {
    assert!(out.status.success(), "{:?} failed: {}", args, String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn today() -> String {
    chrono::Local::now().date_naive().to_string()
}

fn days_from_today(n: u64) -> String {
    (chrono::Local::now().date_naive() + chrono::Days::new(n)).to_string()
}

// -- CLI: the agent's half of the lifecycle --

#[test]
fn cli_agent_review_loop() {
    let env = Env::new();
    env.human(&["add", "Write the report", "--agent", "Bot"]);
    assert_eq!(env.state(1), State::Todo);

    env.agent("Bot", &["edit", "1", "--work-status", "in-progress"]);
    assert_eq!(env.state(1), State::InProgress);

    // Submitting needs a linked note.
    env.refused(Some("Bot"), 1, &["edit", "1", "--work-status", "waiting-for-review"], "no linked note");
    env.agent("Bot", &["note", "add", "Report result", "--task", "1"]);
    env.agent("Bot", &["edit", "1", "--work-status", "waiting-for-review"]);
    assert_eq!(env.state(1), State::WaitingForReview);

    // The retired `complete` alias is a submit too.
    env.agent("Bot", &["edit", "1", "--work-status", "complete"]);
    assert_eq!(env.state(1), State::WaitingForReview);

    // From review the agent may not start again: the human has it.
    env.refused(Some("Bot"), 1, &["edit", "1", "--work-status", "in-progress"], "cannot start");
}

#[test]
fn cli_agent_question() {
    let env = Env::new();
    env.human(&["add", "Pick a colour", "--agent", "Bot"]);
    env.agent("Bot", &["note", "add", "Task 1 Question", "--task", "1"]);
    env.agent("Bot", &["edit", "1", "--work-status", "needs-input"]);
    assert_eq!(env.state(1), State::NeedsInput);
    // Asking again is fine; submitting or starting while blocked is not.
    env.agent("Bot", &["edit", "1", "--work-status", "needs-input"]);
    env.refused(Some("Bot"), 1, &["edit", "1", "--work-status", "waiting-for-review"], "cannot submit");
    env.refused(Some("Bot"), 1, &["edit", "1", "--work-status", "in-progress"], "cannot start");
}

#[test]
fn cli_agent_is_refused_human_moves() {
    let env = Env::new();
    env.human(&["add", "Ship it", "--agent", "Bot"]);
    env.agent("Bot", &["note", "add", "Ship result", "--task", "1"]);
    env.agent("Bot", &["edit", "1", "--work-status", "waiting-for-review"]);

    env.refused(Some("Bot"), 1, &["done", "1"], "only a human");
    env.refused(Some("Bot"), 1, &["edit", "1", "--work-status", "todo"], "only a human");
    env.refused(Some("Bot"), 1, &["edit", "1", "--work-status", "changes-requested"], "changes-requested is set by");
    env.refused(Some("Bot"), 1, &["edit", "1", "--agent", "human"], "only a human can reassign");
    env.refused(Some("Bot"), 1, &["edit", "1", "--work-status", "in-progress", "--force"], "--force is for humans only");
    env.human(&["done", "1"]);
    env.refused(Some("Bot"), 1, &["reopen", "1"], "only a human");
    env.refused(Some("Bot"), 1, &["reopen", "1", "--force"], "--force is for humans only");
}

#[test]
fn cli_agent_may_not_work_a_human_task() {
    let env = Env::new();
    env.human(&["add", "Call the plumber", "--agent", "human"]);
    env.human(&["add", "Unassigned errand"]);
    for id in ["1", "2"] {
        let n: u32 = id.parse().unwrap();
        env.refused(Some("Bot"), n, &["edit", id, "--work-status", "in-progress"], "assigned to the human");
        env.refused(Some("Bot"), n, &["edit", id, "--work-status", "needs-input"], "assigned to the human");
    }
}

// -- CLI: the human's half --

#[test]
fn cli_human_moves() {
    let env = Env::new();
    env.human(&["add", "Mine"]);
    env.human(&["add", "Theirs", "--agent", "Bot"]);

    // The human works their own task.
    env.human(&["edit", "1", "--work-status", "in-progress"]);
    assert_eq!(env.state(1), State::InProgress);
    env.human(&["edit", "1", "--work-status", "todo"]);
    assert_eq!(env.state(1), State::Todo);

    // Agent-only states are refused to the human, with or without --force.
    env.refused(None, 2, &["edit", "2", "--work-status", "in-progress"], "is set by that agent");
    env.refused(None, 2, &["edit", "2", "--work-status", "in-progress", "--force"], "is set by that agent");
    env.refused(None, 2, &["edit", "2", "--work-status", "needs-input"], "set by the agent");
    env.refused(None, 2, &["edit", "2", "--work-status", "waiting-for-review", "--force"], "set by the agent");
    env.refused(None, 2, &["edit", "2", "--work-status", "failing"], "only valid on services");

    // done -> reopen, and the transition rules around them.
    assert!(env.human(&["done", "2"]).contains("Completed task 2"));
    assert_eq!(env.state(2), State::Done);
    assert!(env.human(&["done", "2"]).contains("already done"));
    env.refused(None, 2, &["edit", "2", "--work-status", "todo"], "cannot reset");
    env.human(&["reopen", "2"]);
    assert_eq!((env.state(2), env.task(2).work_status), (State::Todo, None));
    env.refused(None, 2, &["reopen", "2"], "cannot reopen");
    // --force skips the transition rule.
    env.human(&["reopen", "2", "--force"]);
    assert_eq!(env.state(2), State::Todo);
}

#[test]
fn cli_done_on_a_recurring_task_spawns_the_next() {
    let env = Env::new();
    env.human(&["add", "Water plants", "--due", &today(), "--agent", "Bot"]);
    env.human(&["edit", "1", "--recur", "weekly"]);
    let out = env.human(&["done", "1"]);
    assert!(out.contains("Next occurrence: task 2"), "{}", out);
    assert_eq!(env.state(1), State::Done);
    let next = env.task(2);
    assert_eq!((lifecycle::state(&next), next.agent.as_deref()), (State::Todo, Some("Bot")));
    assert_eq!(next.due_date.unwrap().to_string(), days_from_today(7));
}

// -- CLI: services --

#[test]
fn cli_service_lifecycle() {
    let env = Env::new();
    env.human(&["service", "add", "Nightly sync", "--recur", "daily", "--agent", "Bot"]);
    assert_eq!(env.state(1), State::Scheduled);

    env.agent("Bot", &["edit", "1", "--work-status", "in-progress"]);
    assert_eq!(env.state(1), State::Running);
    env.agent("Bot", &["service", "run", "1", "--result", "failed", "--summary", "timeout"]);
    assert_eq!(env.state(1), State::Failing);
    assert_eq!(env.task(1).due_date.unwrap().to_string(), days_from_today(1));

    // Not due yet now; another agent's service is not yours to run.
    env.refused(Some("Bot"), 1, &["service", "run", "1", "--result", "ok"], "not due until");
    env.refused(Some("Other"), 1, &["service", "run", "1", "--result", "ok"], "belongs to Bot");

    // Pause and resume are the human's, and each only from the other side.
    env.refused(Some("Bot"), 1, &["service", "pause", "1"], "only a human");
    env.human(&["service", "pause", "1"]);
    assert!(env.task(1).paused);
    env.refused(None, 1, &["service", "pause", "1"], "already paused");
    env.refused(Some("Bot"), 1, &["edit", "1", "--work-status", "in-progress"], "paused");
    env.human(&["service", "resume", "1"]);
    assert!(!env.task(1).paused);
    env.refused(None, 1, &["service", "resume", "1"], "not paused");

    // A service is never done or reviewed.
    env.refused(None, 1, &["done", "1"], "never done");
    env.agent("Bot", &["note", "add", "Sync result", "--task", "1"]);
    env.refused(Some("Bot"), 1, &["edit", "1", "--work-status", "waiting-for-review"], "never reviewed");
    env.refused(Some("Bot"), 1, &["edit", "1", "--work-status", "needs-input"], "cannot ask");

    // Reset clears failing; converting is the human's.
    env.human(&["edit", "1", "--work-status", "todo"]);
    assert_eq!(env.state(1), State::Scheduled);
    env.refused(Some("Bot"), 1, &["edit", "1", "--no-service"], "only a human");
    env.human(&["edit", "1", "--no-service"]);
    assert_eq!(env.state(1), State::Todo);
    env.human(&["edit", "1", "--service"]);
    assert_eq!(env.state(1), State::Scheduled);
}

#[test]
fn cli_converting_needs_a_schedule_and_an_open_task() {
    let env = Env::new();
    env.human(&["add", "One-off"]);
    env.refused(None, 1, &["edit", "1", "--service"], "needs a recurrence and a due date");
    env.human(&["edit", "1", "--service", "--recur", "daily", "--due", &today()]);
    assert_eq!(env.state(1), State::Scheduled);

    env.human(&["add", "Finished", "--due", &today()]);
    env.human(&["edit", "2", "--recur", "daily"]);
    env.human(&["done", "2"]);
    env.refused(None, 2, &["edit", "2", "--service"], "reopen it before");
}

// -- Web API, through `task serve` --

struct Server {
    child: Child,
    base: String,
    http: reqwest::blocking::Client,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Server {
    fn start(env: &Env) -> Self {
        let port = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let child = env
            .cmd(None, &["serve"])
            .env("TASK_SERVER_PORT", port.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let server = Server {
            child,
            base: format!("http://127.0.0.1:{}", port),
            http: reqwest::blocking::Client::builder().timeout(Duration::from_secs(10)).build().unwrap(),
        };
        let deadline = Instant::now() + Duration::from_secs(20);
        while server.http.get(server.url("/api/tasks")).send().is_err() {
            assert!(Instant::now() < deadline, "task serve did not start on port {}", port);
            std::thread::sleep(Duration::from_millis(50));
        }
        server
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base, path)
    }

    fn get(&self, path: &str) -> Value {
        let r = self.http.get(self.url(path)).send().unwrap();
        assert!(r.status().is_success(), "GET {}: {}", path, r.status());
        r.json().unwrap()
    }

    /// Send a request and return (status, body).
    fn send(&self, method: &str, path: &str, body: Option<Value>) -> (u16, Value) {
        let mut req = self.http.request(method.parse().unwrap(), self.url(path));
        if let Some(b) = body {
            req = req.json(&b);
        }
        let r = req.send().unwrap();
        let status = r.status().as_u16();
        (status, r.json().unwrap_or(Value::Null))
    }

    fn ok(&self, method: &str, path: &str, body: Option<Value>) -> Value {
        let (status, json) = self.send(method, path, body);
        assert!((200..300).contains(&status), "{} {}: {} {}", method, path, status, json);
        json
    }

    /// Require a refusal with `status`, an error containing `reason`, and the
    /// task unchanged on disk.
    fn refused(&self, env: &Env, id: u32, method: &str, path: &str, body: Option<Value>, status: u16, reason: &str) {
        let before = env.task(id);
        let (got, json) = self.send(method, path, body);
        assert_eq!(got, status, "{} {}: {}", method, path, json);
        let err = json["error"].as_str().unwrap_or_default();
        assert!(err.contains(reason), "{} {}: expected {:?} in {:?}", method, path, reason, err);
        assert_eq!(env.task(id), before, "{} {}: a refused move must not change the task", method, path);
    }
}

fn actions(v: &Value) -> Vec<String> {
    v["allowed_actions"].as_array().unwrap().iter().map(|a| a.as_str().unwrap().to_string()).collect()
}

/// A task assigned to Bot, submitted for review with a linked note. `edit`
/// args (such as `--recur`) are applied before the agent picks it up.
fn submitted(env: &Env, title: &str, edit: &[&str]) -> u32 {
    let out = env.human(&["add", title, "--agent", "Bot"]);
    let id: u32 = out.split(|c: char| !c.is_ascii_digit()).find(|s| !s.is_empty()).unwrap().parse().unwrap();
    let ids = id.to_string();
    if !edit.is_empty() {
        env.human(&[&["edit", &ids][..], edit].concat());
    }
    env.agent("Bot", &["note", "add", &format!("{} result", title), "--task", &ids]);
    env.agent("Bot", &["edit", &ids, "--work-status", "waiting-for-review"]);
    id
}

#[test]
fn api_reports_state_and_only_moves_that_work() {
    let env = Env::new();
    let id = submitted(&env, "Review me", &[]);
    let server = Server::start(&env);

    let t = server.get(&format!("/api/tasks/{}", id));
    assert_eq!(t["state"], "waiting-for-review");
    assert_eq!(actions(&t), ["feedback", "done", "reset", "defer"]);
    // The same view comes back from the list.
    let list = server.get("/api/tasks");
    assert_eq!(list[0]["allowed_actions"], t["allowed_actions"]);

    // A recurring task may also become a service.
    let rid = submitted(&env, "Weekly review", &["--recur", "weekly", "--due", &today()]);
    assert!(actions(&server.get(&format!("/api/tasks/{}", rid))).contains(&"to-service".to_string()));
}

#[test]
fn api_review_and_question_loop() {
    let env = Env::new();
    let id = submitted(&env, "Draft", &[]);
    let server = Server::start(&env);
    let p = |s: &str| format!("/api/tasks/{}{}", id, s);

    // Feedback sends it back to the agent and writes the review thread.
    server.ok("POST", &p("/review"), Some(json!({ "text": "Tighten the intro" })));
    assert_eq!(env.state(id), State::ChangesRequested);
    let review = server.get(&p("/review"));
    assert!(review["body"].as_str().unwrap().contains("Tighten the intro"));

    // The agent restarts, asks a question, and the human answers.
    env.agent("Bot", &["edit", &id.to_string(), "--work-status", "in-progress"]);
    server.refused(&env, id, "POST", &p("/question"), Some(json!({ "text": "early" })), 409, "cannot answer");
    server.refused(&env, id, "POST", &p("/review"), Some(json!({ "text": "early" })), 409, "cannot post review feedback");
    env.agent("Bot", &["note", "add", &format!("Task {} Question", id), "--task", &id.to_string()]);
    env.agent("Bot", &["edit", &id.to_string(), "--work-status", "needs-input"]);
    server.ok("POST", &p("/question"), Some(json!({ "text": "Blue" })));
    assert_eq!(env.state(id), State::ChangesRequested);
    assert!(server.get(&p("/question"))["body"].as_str().unwrap().contains("Blue"));

    // The human can not set agent states from the API. (Resending the current
    // changes-requested is not a move, so it is left out.)
    for ws in ["in-progress", "needs-input", "waiting-for-review"] {
        let (status, _) = server.send("PATCH", &p(""), Some(json!({ "work_status": ws })));
        assert_eq!(status, 400, "PATCH work_status {}", ws);
    }
    assert_eq!(env.state(id), State::ChangesRequested);

    // Reset, done, reopen. Every response carries the new state and moves.
    let reset = server.ok("PATCH", &p(""), Some(json!({ "work_status": "todo" })));
    assert_eq!((&reset["work_status"], &reset["state"]), (&json!("todo"), &json!("todo")));
    assert!(actions(&reset).contains(&"done".to_string()));
    let done = server.ok("POST", &p("/done"), None);
    assert_eq!(done["completed"]["status"], "done");
    assert_eq!(actions(&done["completed"]), ["feedback", "reopen"]);
    assert_eq!(server.ok("POST", &p("/done"), None)["completed"]["status"], "done");
    // Feedback on a done task reopens it for another round.
    server.ok("POST", &p("/review"), Some(json!({ "text": "One more thing" })));
    assert_eq!(env.state(id), State::ChangesRequested);
    server.ok("POST", &p("/done"), None);
    let reopened = server.ok("POST", &p("/reopen"), None);
    assert_eq!(reopened["state"], "todo");
    assert_eq!(env.state(id), State::Todo);
    server.refused(&env, id, "POST", &p("/reopen"), None, 409, "cannot reopen");
}

#[test]
fn api_review_handoffs() {
    let env = Env::new();
    let deferred = submitted(&env, "Defer me", &["--due", &days_from_today(1)]);
    let reassigned = submitted(&env, "Reassign me", &[]);
    let kept = submitted(&env, "Keep me", &[]);
    let server = Server::start(&env);

    // Pushing a review task's due date later hands it back to the human.
    server.ok("PATCH", &format!("/api/tasks/{}", deferred), Some(json!({ "due": days_from_today(5) })));
    let t = env.task(deferred);
    assert_eq!((t.agent.as_deref(), lifecycle::state(&t)), (Some("human"), State::Todo));

    // Reassigning a review task to another agent resets it for them.
    server.ok("PATCH", &format!("/api/tasks/{}", reassigned), Some(json!({ "agent": "Other" })));
    let t = env.task(reassigned);
    assert_eq!((t.agent.as_deref(), lifecycle::state(&t)), (Some("Other"), State::Todo));

    // Resending the current status (a full edit form) is not a move.
    server.ok("PATCH", &format!("/api/tasks/{}", kept), Some(json!({ "work_status": "waiting-for-review", "title": "Kept" })));
    let t = env.task(kept);
    assert_eq!((t.title.as_str(), lifecycle::state(&t)), ("Kept", State::WaitingForReview));
}

#[test]
fn api_service_lifecycle() {
    let env = Env::new();
    env.human(&["add", "Backups", "--agent", "Bot"]);
    let server = Server::start(&env);

    // Converting needs a schedule; the web UI sends it with the conversion.
    server.refused(&env, 1, "PATCH", "/api/tasks/1", Some(json!({ "kind": "service" })), 400, "needs a recurrence");
    server.ok("PATCH", "/api/tasks/1", Some(json!({ "kind": "service", "recurrence": "daily", "due": today(), "work_status": "todo" })));
    assert_eq!(env.state(1), State::Scheduled);
    let services = server.get("/api/services");
    assert_eq!(services[0]["state"], "scheduled");
    assert_eq!(actions(&services[0]), ["reset", "run-ok", "run-failed", "pause", "to-task"]);
    // Services leave the task list.
    assert_eq!(server.get("/api/tasks").as_array().unwrap().len(), 0);

    server.refused(&env, 1, "POST", "/api/tasks/1/done", None, 400, "never done");
    server.ok("POST", "/api/services/1/pause", None);
    assert!(env.task(1).paused);
    assert_eq!(actions(&server.get("/api/services")[0]), ["reset", "resume", "to-task"]);
    server.refused(&env, 1, "POST", "/api/services/1/pause", None, 400, "already paused");
    server.ok("POST", "/api/services/1/resume", None);
    server.refused(&env, 1, "POST", "/api/services/1/resume", None, 400, "not paused");

    env.agent("Bot", &["service", "run", "1", "--result", "failed"]);
    assert_eq!(server.get("/api/services")[0]["state"], "failing");

    server.ok("PATCH", "/api/tasks/1", Some(json!({ "kind": "task" })));
    let t = env.task(1);
    assert_eq!((lifecycle::state(&t), t.paused), (State::Todo, false));

    // A new service from the API starts scheduled.
    let created = server.ok("POST", "/api/services", Some(json!({ "title": "Digest", "recurrence": "weekly", "agent": "Bot" })));
    let id = created["id"].as_u64().map(|n| n as u32).unwrap_or(2);
    assert_eq!(env.state(id), State::Scheduled);
}

#[test]
fn every_move_is_refused_with_the_state_left_alone() {
    // A sweep across stored states: for each, every agent-only work status
    // PATCHed by the human is refused and nothing on disk changes.
    let env = Env::new();
    env.human(&["add", "Todo", "--agent", "Bot"]);
    env.human(&["add", "Progress", "--agent", "Bot"]);
    env.agent("Bot", &["edit", "2", "--work-status", "in-progress"]);
    let review = submitted(&env, "Review", &[]);
    env.human(&["add", "Done", "--agent", "Bot"]);
    env.human(&["done", "4"]);
    let server = Server::start(&env);
    for id in [1, 2, review, 4] {
        for ws in ["needs-input", "waiting-for-review", "changes-requested", "failing"] {
            let current = env.task(id).work_status.map(|w| w.to_string());
            if current.as_deref() == Some(ws) {
                continue;
            }
            let (status, _) = server.send("PATCH", &format!("/api/tasks/{}", id), Some(json!({ "work_status": ws })));
            assert_eq!(status, 400, "task {} -> {}", id, ws);
        }
    }
    assert_eq!(
        [1, 2, review, 4].map(|id| env.state(id)),
        [State::Todo, State::InProgress, State::WaitingForReview, State::Done]
    );
}

#[test]
fn api_done_on_a_recurring_task_returns_the_next() {
    let env = Env::new();
    env.human(&["add", "Standup notes", "--agent", "Bot", "--due", &today()]);
    env.human(&["edit", "1", "--recur", "daily"]);
    let server = Server::start(&env);
    let done = server.ok("POST", "/api/tasks/1/done", None);
    assert_eq!(done["completed"]["status"], "done");
    assert_eq!(done["spawned"]["id"], 2);
    assert_eq!(done["spawned"]["due_date"], days_from_today(1));
    assert_eq!(env.state(2), State::Todo);
    // Completing it again does not spawn another.
    assert!(server.ok("POST", "/api/tasks/1/done", None)["spawned"].is_null());
    assert_eq!(task::db::load(&env.db).unwrap().tasks.len(), 2);
}

#[test]
fn api_patch_is_all_or_nothing() {
    // The conversion would succeed but the status move after it is refused:
    // nothing in the request may be saved.
    let env = Env::new();
    env.human(&["add", "Half a move", "--agent", "Bot"]);
    let server = Server::start(&env);
    server.refused(
        &env,
        1,
        "PATCH",
        "/api/tasks/1",
        Some(json!({ "kind": "service", "recurrence": "daily", "due": today(), "title": "Renamed", "work_status": "needs-input" })),
        400,
        "set by the agent",
    );
}

#[test]
fn retired_complete_status_is_migrated_to_waiting_for_review() {
    let env = Env::new();
    env.human(&["add", "Old row", "--agent", "Bot"]);
    // Write the retired value the way an old build stored it.
    let conn = rusqlite::Connection::open(&env.db).unwrap();
    conn.execute("UPDATE tasks SET work_status = 'complete' WHERE id = 1", []).unwrap();
    drop(conn);
    assert_eq!(env.state(1), State::WaitingForReview);
    let conn = rusqlite::Connection::open(&env.db).unwrap();
    let stored: String = conn.query_row("SELECT work_status FROM tasks WHERE id = 1", [], |r| r.get(0)).unwrap();
    assert_eq!(stored, "waiting-for-review");
}
