use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// 1.1 Config helper
// ---------------------------------------------------------------------------

pub fn claude_code_dir() -> PathBuf {
    if let Some(val) = crate::config::read_config_value("claude-code-dir") {
        if val.starts_with("~/") {
            if let Some(home) = dirs::home_dir() {
                return home.join(&val[2..]);
            }
        }
        return PathBuf::from(val);
    }
    dirs::home_dir()
        .map(|h| h.join("code"))
        .unwrap_or_else(|| PathBuf::from("code"))
}

// ---------------------------------------------------------------------------
// 2.1–2.3 Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClaudeSessionStatus {
    Running,
    WaitingForInput,
    Failed,
    Done,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClaudeSession {
    pub id: usize,
    pub label: String,
    pub working_dir: PathBuf,
    pub status: ClaudeSessionStatus,
    /// Ring buffer — max MAX_OUTPUT_LINES entries
    pub output: Vec<String>,
    #[serde(skip)]
    pub child: Option<Child>,
    #[serde(skip)]
    pub rx: Option<mpsc::Receiver<SessionEvent>>,
}

#[derive(Debug)]
pub enum SessionEvent {
    Line(String),
    Done,
    Error(String),
}

// ---------------------------------------------------------------------------
// 4.1 Context builder
// ---------------------------------------------------------------------------

pub fn build_session_context(title: &str, body: &str) -> String {
    if body.is_empty() {
        format!("Task: {}", title)
    } else {
        format!("Task: {}\n\n{}", title, body)
    }
}

// ---------------------------------------------------------------------------
// 4.2–4.3 Session launch
// ---------------------------------------------------------------------------

pub fn claude_available() -> bool {
    Command::new("claude")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

pub fn launch_claude_session(
    id: usize,
    label: String,
    working_dir: PathBuf,
    context: String,
) -> Result<ClaudeSession, String> {
    let (tx, rx) = mpsc::channel::<SessionEvent>();

    let mut child = Command::new("claude")
        .arg("--print")
        .arg("-p")
        .arg(&context)
        .current_dir(&working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to spawn claude: {}", e))?;

    let stdout = child.stdout.take().expect("stdout was piped");
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    let _ = tx.send(SessionEvent::Line(l));
                }
                Err(e) => {
                    let _ = tx.send(SessionEvent::Error(e.to_string()));
                    return;
                }
            }
        }
        let _ = tx.send(SessionEvent::Done);
    });

    Ok(ClaudeSession {
        id,
        label,
        working_dir,
        status: ClaudeSessionStatus::Running,
        output: Vec::new(),
        child: Some(child),
        rx: Some(rx),
    })
}

pub fn continue_claude_session(session: &mut ClaudeSession, message: String) -> Result<(), String> {
    let (tx, rx) = mpsc::channel::<SessionEvent>();

    let mut child = Command::new("claude")
        .arg("--print")
        .arg("--continue")
        .arg("-p")
        .arg(&message)
        .current_dir(&session.working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to spawn claude: {}", e))?;

    let stdout = child.stdout.take().expect("stdout was piped");
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    let _ = tx.send(SessionEvent::Line(l));
                }
                Err(e) => {
                    let _ = tx.send(SessionEvent::Error(e.to_string()));
                    return;
                }
            }
        }
        let _ = tx.send(SessionEvent::Done);
    });

    session.child = Some(child);
    session.rx = Some(rx);
    session.status = ClaudeSessionStatus::Running;
    Ok(())
}

// ---------------------------------------------------------------------------
// Ring buffer helper
// ---------------------------------------------------------------------------

const MAX_OUTPUT_LINES: usize = 500;

pub fn push_output_line(output: &mut Vec<String>, line: String) {
    if output.len() >= MAX_OUTPUT_LINES {
        output.remove(0);
    }
    output.push(line);
}

// ---------------------------------------------------------------------------
// 10.1–10.6 Persistence
// ---------------------------------------------------------------------------

pub fn session_dir(task_dir: &Path) -> PathBuf {
    task_dir.join("claude-sessions")
}

fn session_slug(label: &str) -> String {
    label
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

pub fn save_session(task_dir: &Path, session: &ClaudeSession) -> Result<(), String> {
    let dir = session_dir(task_dir);
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Failed to create session dir: {}", e))?;

    let slug = session_slug(&session.label);
    let filename = format!("{:05}-{}.json", session.id, slug);
    let path = dir.join(&filename);

    let json = serde_json::to_string_pretty(session)
        .map_err(|e| format!("Failed to serialize session: {}", e))?;
    std::fs::write(&path, json)
        .map_err(|e| format!("Failed to write session file: {}", e))?;

    enforce_retention(&dir);
    Ok(())
}

fn enforce_retention(dir: &Path) {
    let mut entries: Vec<(std::time::SystemTime, PathBuf)> =
        match std::fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(_) => return,
        }
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |x| x == "json"))
        .filter_map(|e| {
            let mtime = e.metadata().ok()?.modified().ok()?;
            Some((mtime, e.path()))
        })
        .collect();

    if entries.len() <= 30 {
        return;
    }
    entries.sort_by_key(|(t, _)| *t);
    for (_, path) in entries.iter().take(entries.len() - 30) {
        let _ = std::fs::remove_file(path);
    }
}

pub fn load_sessions(task_dir: &Path) -> Vec<ClaudeSession> {
    let dir = session_dir(task_dir);
    if !dir.exists() {
        return Vec::new();
    }
    let read_dir = match std::fs::read_dir(&dir) {
        Ok(rd) => rd,
        Err(_) => return Vec::new(),
    };
    let mut sessions: Vec<ClaudeSession> = read_dir
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |x| x == "json"))
        .filter_map(|e| {
            let data = std::fs::read_to_string(e.path()).ok()?;
            let mut session: ClaudeSession = serde_json::from_str(&data).ok()?;
            // Loaded sessions are never Running
            if session.status == ClaudeSessionStatus::Running {
                session.status = ClaudeSessionStatus::Failed;
            }
            Some(session)
        })
        .collect();
    sessions.sort_by_key(|s| s.id);
    sessions
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_build_session_context_with_body() {
        let result = build_session_context("Fix auth bug", "JWT tokens expire too early");
        assert!(result.contains("Fix auth bug"));
        assert!(result.contains("JWT tokens expire too early"));
    }

    #[test]
    fn test_build_session_context_empty_body() {
        let result = build_session_context("My Task", "");
        assert_eq!(result, "Task: My Task");
    }

    #[test]
    fn test_claude_code_dir_default() {
        let dir = claude_code_dir();
        assert!(dir.to_string_lossy().ends_with("code"));
    }

    #[test]
    fn test_push_output_line_ring_buffer() {
        let mut output = Vec::new();
        for i in 0..MAX_OUTPUT_LINES + 10 {
            push_output_line(&mut output, format!("line {}", i));
        }
        assert_eq!(output.len(), MAX_OUTPUT_LINES);
        assert_eq!(output[0], "line 10");
    }

    #[test]
    fn test_status_never_running_after_load() {
        let dir = tempfile::tempdir().unwrap();
        let session = ClaudeSession {
            id: 1,
            label: "test".to_string(),
            working_dir: PathBuf::from("/tmp"),
            status: ClaudeSessionStatus::Running,
            output: vec!["hello".to_string()],
            child: None,
            rx: None,
        };
        save_session(dir.path(), &session).unwrap();
        let loaded = load_sessions(dir.path());
        assert_eq!(loaded.len(), 1);
        assert_ne!(loaded[0].status, ClaudeSessionStatus::Running);
    }

    #[test]
    fn test_save_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let session = ClaudeSession {
            id: 42,
            label: "my-feature".to_string(),
            working_dir: PathBuf::from("/projects/foo"),
            status: ClaudeSessionStatus::WaitingForInput,
            output: vec!["line 1".to_string(), "line 2".to_string()],
            child: None,
            rx: None,
        };
        save_session(dir.path(), &session).unwrap();
        let loaded = load_sessions(dir.path());
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, 42);
        assert_eq!(loaded[0].label, "my-feature");
        assert_eq!(loaded[0].output.len(), 2);
        assert_eq!(loaded[0].status, ClaudeSessionStatus::WaitingForInput);
    }

    #[test]
    fn test_retention_deletes_oldest() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..35usize {
            let session = ClaudeSession {
                id: i,
                label: format!("session-{}", i),
                working_dir: PathBuf::from("/tmp"),
                status: ClaudeSessionStatus::Done,
                output: Vec::new(),
                child: None,
                rx: None,
            };
            save_session(dir.path(), &session).unwrap();
        }
        let loaded = load_sessions(dir.path());
        assert_eq!(loaded.len(), 30);
    }
}
