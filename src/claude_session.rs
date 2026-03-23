use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::thread;

use serde::{Deserialize, Serialize};
use serde_json::Value;

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
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClaudeSessionStatus {
    Running,
    WaitingForInput,
    Failed,
    Done,
}

/// A single structured output event stored in the session ring buffer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionOutputEvent {
    Text(String),
    ToolCall {
        name: String,
        input_preview: String,
        result_lines: Vec<String>,
        collapsed: bool,
    },
    PermissionRequest {
        tool: String,
        input_preview: String,
    },
    TurnSeparator,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClaudeSession {
    pub id: usize,
    pub label: String,
    pub working_dir: PathBuf,
    pub status: ClaudeSessionStatus,
    pub output: Vec<SessionOutputEvent>,
    pub session_id: Option<String>,
    #[serde(skip)]
    pub child: Option<Child>,
    #[serde(skip)]
    pub stdin: Option<ChildStdin>,
    #[serde(skip)]
    pub rx: Option<mpsc::Receiver<SessionEvent>>,
}

/// Events sent from the reader thread to the TUI polling loop.
#[derive(Debug)]
pub enum SessionEvent {
    OutputEvent(SessionOutputEvent),
    AppendToolResult { lines: Vec<String> },
    PermissionRequest { tool: String, input_preview: String },
    SessionIdCaptured(String),
    Done,
    Error(String),
}

// ---------------------------------------------------------------------------
// Context builder
// ---------------------------------------------------------------------------

pub fn build_session_context(title: &str, body: &str) -> String {
    if body.is_empty() {
        format!("Task: {}", title)
    } else {
        format!("Task: {}\n\n{}", title, body)
    }
}

// ---------------------------------------------------------------------------
// Session launch
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
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose")
        .arg("-p")
        .arg(&context)
        .current_dir(&working_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to spawn claude: {}", e))?;

    let stdin = child.stdin.take();
    let stdout = child.stdout.take().expect("stdout was piped");

    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    for event in parse_stream_json_line(&l) {
                        let _ = tx.send(event);
                    }
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
        session_id: None,
        child: Some(child),
        stdin,
        rx: Some(rx),
    })
}

pub fn continue_claude_session(session: &mut ClaudeSession, message: String) -> Result<(), String> {
    push_output_event(&mut session.output, SessionOutputEvent::TurnSeparator);

    let (tx, rx) = mpsc::channel::<SessionEvent>();

    // Use --resume <id> if we captured a session_id, otherwise fall back to --continue
    let resume_arg: Box<dyn AsRef<std::ffi::OsStr>> = if let Some(ref sid) = session.session_id {
        Box::new(sid.clone())
    } else {
        Box::new("--continue")
    };

    let mut cmd = Command::new("claude");
    cmd.arg("--print")
        .arg("--output-format")
        .arg("stream-json")
        .arg("--verbose");

    if session.session_id.is_some() {
        cmd.arg("--resume").arg(resume_arg.as_ref());
    } else {
        cmd.arg("--continue");
    }

    cmd.arg("-p")
        .arg(&message)
        .current_dir(&session.working_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn claude: {}", e))?;

    let stdin = child.stdin.take();
    let stdout = child.stdout.take().expect("stdout was piped");

    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    for event in parse_stream_json_line(&l) {
                        let _ = tx.send(event);
                    }
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
    session.stdin = stdin;
    session.rx = Some(rx);
    session.status = ClaudeSessionStatus::Running;
    Ok(())
}

/// Write a permission response to the session subprocess stdin.
/// `allowed`: true → "y\n", false → "n\n"
pub fn respond_to_permission(session: &mut ClaudeSession, allowed: bool) {
    if let Some(ref mut stdin) = session.stdin {
        let response = if allowed { "y\n" } else { "n\n" };
        let _ = stdin.write_all(response.as_bytes());
        let _ = stdin.flush();
    }
}

// ---------------------------------------------------------------------------
// Stream-JSON parser
// ---------------------------------------------------------------------------

fn extract_tool_input_preview(input: &Value) -> String {
    input["command"]
        .as_str()
        .or_else(|| input["description"].as_str())
        .or_else(|| input["prompt"].as_str())
        .or_else(|| input["path"].as_str())
        .or_else(|| input["pattern"].as_str())
        .unwrap_or("")
        .lines()
        .next()
        .unwrap_or("")
        .to_string()
}

/// Parse a single newline-delimited stream-json event and return zero or more `SessionEvent`s.
pub fn parse_stream_json_line(raw: &str) -> Vec<SessionEvent> {
    let v: Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(_) => {
            // Not JSON — emit as plain text if non-empty
            let s = raw.trim();
            if s.is_empty() {
                return vec![];
            }
            return vec![SessionEvent::OutputEvent(SessionOutputEvent::Text(s.to_string()))];
        }
    };

    let event_type = v["type"].as_str().unwrap_or("");

    match event_type {
        "system" => {
            // Check for permission_request subtype
            let subtype = v["subtype"].as_str().unwrap_or("");
            if subtype.contains("permission") {
                let tool = v["tool_name"]
                    .as_str()
                    .or_else(|| v["tool"].as_str())
                    .unwrap_or("tool")
                    .to_string();
                let input_preview = extract_tool_input_preview(&v["input"]);
                return vec![SessionEvent::PermissionRequest { tool, input_preview }];
            }
            vec![]
        }

        "result" => {
            // Capture session_id
            let mut events = vec![];
            if let Some(sid) = v["session_id"].as_str() {
                events.push(SessionEvent::SessionIdCaptured(sid.to_string()));
            }
            events
        }

        "assistant" => {
            let mut events = Vec::new();
            if let Some(content) = v["message"]["content"].as_array() {
                for item in content {
                    match item["type"].as_str().unwrap_or("") {
                        "thinking" => {
                            events.push(SessionEvent::OutputEvent(SessionOutputEvent::Text(
                                "💭 thinking...".to_string(),
                            )));
                        }
                        "tool_use" => {
                            let name = item["name"].as_str().unwrap_or("tool").to_string();
                            let input_preview = extract_tool_input_preview(&item["input"]);
                            events.push(SessionEvent::OutputEvent(SessionOutputEvent::ToolCall {
                                name,
                                input_preview,
                                result_lines: Vec::new(),
                                collapsed: true,
                            }));
                        }
                        "text" => {
                            let text = item["text"].as_str().unwrap_or("");
                            for line in text.lines() {
                                events.push(SessionEvent::OutputEvent(SessionOutputEvent::Text(
                                    line.to_string(),
                                )));
                            }
                            if text.is_empty() {
                                events.push(SessionEvent::OutputEvent(SessionOutputEvent::Text(
                                    String::new(),
                                )));
                            }
                        }
                        _ => {}
                    }
                }
            }
            events
        }

        "user" => {
            let mut events = Vec::new();
            if let Some(content) = v["message"]["content"].as_array() {
                for item in content {
                    if item["type"].as_str() == Some("tool_result") {
                        let output = v["tool_use_result"]["stdout"]
                            .as_str()
                            .or_else(|| item["content"].as_str())
                            .unwrap_or("");
                        let mut lines: Vec<String> =
                            output.lines().take(50).map(|l| strip_ansi(l)).collect();
                        let total = output.lines().count();
                        if total > 50 {
                            lines.push(format!("… ({} more lines)", total - 50));
                        }
                        events.push(SessionEvent::AppendToolResult { lines });
                    }
                }
            }
            events
        }

        _ => vec![],
    }
}

// ---------------------------------------------------------------------------
// Ring buffer
// ---------------------------------------------------------------------------

const MAX_OUTPUT_EVENTS: usize = 500;

pub const TURN_SEPARATOR_LABEL: &str = "──── reply ────";

/// Push a `SessionOutputEvent` into the ring buffer, evicting oldest if full.
pub fn push_output_event(output: &mut Vec<SessionOutputEvent>, event: SessionOutputEvent) {
    if output.len() >= MAX_OUTPUT_EVENTS {
        output.remove(0);
    }
    output.push(event);
}

/// Strip ANSI escape sequences (ESC [ ... cmd_byte) from a string.
pub fn strip_ansi(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\x1b' && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            i += 2;
            while i < bytes.len() && !(0x40..=0x7e).contains(&bytes[i]) {
                i += 1;
            }
            i += 1;
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Persistence
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
    fn test_strip_ansi_color_codes() {
        assert_eq!(strip_ansi("\x1b[32mHello\x1b[0m"), "Hello");
    }

    #[test]
    fn test_strip_ansi_plain_text_unchanged() {
        assert_eq!(strip_ansi("plain text"), "plain text");
    }

    #[test]
    fn test_strip_ansi_partial_sequence_at_end() {
        let s = "ok\x1b[";
        let result = strip_ansi(s);
        assert!(!result.contains('\x1b'));
    }

    #[test]
    fn test_strip_ansi_multiple_sequences() {
        assert_eq!(strip_ansi("\x1b[1m\x1b[33mWarn\x1b[0m: msg"), "Warn: msg");
    }

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
    fn test_push_output_event_ring_buffer() {
        let mut output = Vec::new();
        for i in 0..MAX_OUTPUT_EVENTS + 10 {
            push_output_event(&mut output, SessionOutputEvent::Text(format!("line {}", i)));
        }
        assert_eq!(output.len(), MAX_OUTPUT_EVENTS);
        // Oldest 10 evicted — first remaining is "line 10"
        if let SessionOutputEvent::Text(s) = &output[0] {
            assert_eq!(s, "line 10");
        } else {
            panic!("expected Text event");
        }
    }

    #[test]
    fn test_parse_stream_json_text_event() {
        let json = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hello\nWorld"}]}}"#;
        let events = parse_stream_json_line(json);
        assert_eq!(events.len(), 2);
        if let SessionEvent::OutputEvent(SessionOutputEvent::Text(s)) = &events[0] {
            assert_eq!(s, "Hello");
        } else {
            panic!("expected Text");
        }
    }

    #[test]
    fn test_parse_stream_json_tool_use() {
        let json = r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Bash","input":{"command":"ls -la"}}]}}"#;
        let events = parse_stream_json_line(json);
        assert_eq!(events.len(), 1);
        if let SessionEvent::OutputEvent(SessionOutputEvent::ToolCall { name, input_preview, collapsed, .. }) = &events[0] {
            assert_eq!(name, "Bash");
            assert_eq!(input_preview, "ls -la");
            assert!(*collapsed);
        } else {
            panic!("expected ToolCall");
        }
    }

    #[test]
    fn test_parse_stream_json_tool_result() {
        let json = r#"{"type":"user","message":{"content":[{"type":"tool_result","content":"output here"}]},"tool_use_result":{"stdout":"output here","stderr":"","interrupted":false,"isImage":false,"noOutputExpected":false}}"#;
        let events = parse_stream_json_line(json);
        assert_eq!(events.len(), 1);
        if let SessionEvent::AppendToolResult { lines } = &events[0] {
            assert_eq!(lines[0], "output here");
        } else {
            panic!("expected AppendToolResult");
        }
    }

    #[test]
    fn test_parse_stream_json_session_id() {
        let json = r#"{"type":"result","subtype":"success","session_id":"abc-123","result":"done"}"#;
        let events = parse_stream_json_line(json);
        assert!(events.iter().any(|e| matches!(e, SessionEvent::SessionIdCaptured(s) if s == "abc-123")));
    }

    #[test]
    fn test_parse_stream_json_system_skipped() {
        let json = r#"{"type":"system","subtype":"hook_started","hook_name":"test"}"#;
        let events = parse_stream_json_line(json);
        assert!(events.is_empty());
    }

    fn make_session(id: usize) -> ClaudeSession {
        ClaudeSession {
            id,
            label: format!("session-{}", id),
            working_dir: PathBuf::from("/tmp"),
            status: ClaudeSessionStatus::Done,
            output: Vec::new(),
            session_id: None,
            child: None,
            stdin: None,
            rx: None,
        }
    }

    #[test]
    fn test_status_never_running_after_load() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = make_session(1);
        session.status = ClaudeSessionStatus::Running;
        session.output.push(SessionOutputEvent::Text("hello".to_string()));
        save_session(dir.path(), &session).unwrap();
        let loaded = load_sessions(dir.path());
        assert_eq!(loaded.len(), 1);
        assert_ne!(loaded[0].status, ClaudeSessionStatus::Running);
    }

    #[test]
    fn test_save_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = make_session(42);
        session.label = "my-feature".to_string();
        session.working_dir = PathBuf::from("/projects/foo");
        session.status = ClaudeSessionStatus::WaitingForInput;
        session.output.push(SessionOutputEvent::Text("line 1".to_string()));
        session.output.push(SessionOutputEvent::Text("line 2".to_string()));
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
            save_session(dir.path(), &make_session(i)).unwrap();
        }
        let loaded = load_sessions(dir.path());
        assert_eq!(loaded.len(), 30);
    }
}
