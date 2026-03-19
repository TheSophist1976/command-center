## 1. Config

- [x] 1.1 Add `read_config_value("claude-code-dir")` usage in a new `claude_code_dir()` helper that returns the configured path or `~/code` as default (tilde-expanded via `dirs::home_dir()`)

## 2. Data structures

- [x] 2.1 Add `ClaudeSessionStatus` enum: `Running`, `WaitingForInput`, `Failed`, `Done`
- [x] 2.2 Add `ClaudeSession` struct: `id: usize`, `label: String`, `working_dir: PathBuf`, `status: ClaudeSessionStatus`, `output: Vec<String>` (ring buffer), `child: Option<Child>`, `rx: Option<mpsc::Receiver<SessionEvent>>`
- [x] 2.3 Add `SessionEvent` enum: `Line(String)`, `Done`, `Error(String)`
- [x] 2.4 Add to `App`: `claude_sessions: Vec<ClaudeSession>`, `session_selected: usize`, `session_dir_picker: Vec<PathBuf>`, `session_dir_picker_selected: usize`, `session_pending_context: Option<String>`, `session_reply_input: String`
- [x] 2.5 Add `Mode::SessionDirectoryPicker`, `Mode::Sessions`, `Mode::SessionReply` variants to the `Mode` enum

## 3. Directory picker

- [x] 3.1 Add `fn populate_session_dir_picker(app: &mut App)` that reads `claude_code_dir()`, lists immediate subdirectories via `fs::read_dir`, sorts alphabetically, and stores them in `app.session_dir_picker`
- [x] 3.2 Add `fn draw_session_dir_picker(frame, app, area)` rendering a modal overlay with the directory list and a highlighted selection row
- [x] 3.3 Add `fn handle_session_dir_picker(app, key)` handling `j`/`k`/`Up`/`Down` for navigation, `Enter` to confirm (call `launch_claude_session`), `Esc` to cancel back to Normal mode

## 4. Session launch

- [x] 4.1 Add `fn build_session_context(title: &str, body: &str) -> String` formatting task/note content as the initial prompt string
- [x] 4.2 Add `fn launch_claude_session(app: &mut App, working_dir: PathBuf, context: String)` that: checks for `claude` binary, spawns `claude --print -p "<context>"` with `stdout: Stdio::piped()`, spawns a reader thread sending `SessionEvent` lines via mpsc, and pushes a new `ClaudeSession` to `app.claude_sessions`
- [x] 4.3 Wire graceful degradation: if `Command::new("claude").arg("--version").output()` fails, set a status message and return without entering Sessions mode

## 5. Session event loop polling

- [x] 5.1 In the main TUI event loop, iterate `app.claude_sessions` and `try_recv` on each session's `rx`; append `Line` events to `session.output` (ring buffer: drop front when `len > 500`), handle `Done` → `WaitingForInput`, handle `Error` → `Failed`, clear `child` and `rx` on terminal events

## 6. Sessions panel

- [x] 6.1 Add `fn draw_sessions_panel(frame, app, area)` rendering a list of sessions with label, directory name, status indicator (spinner for Running), and last output line
- [x] 6.2 Add `fn handle_sessions(app, key)` handling `j`/`k` navigation, `Enter` to view full output (scrollable inner panel), `r` on a WaitingForInput session to enter `SessionReply` mode, `Esc`/`q` to return to Normal mode
- [x] 6.3 Add scrollable full-output detail view within the sessions panel (activated by `Enter` on a session, `Esc` to return to list)

## 7. Inline reply

- [x] 7.1 Add `fn draw_session_reply(frame, app, area)` rendering the reply input box below the sessions panel
- [x] 7.2 Add `fn handle_session_reply(app, key)` handling character input into `app.session_reply_input`, `Backspace`, `Enter` to call `send_session_reply`, `Esc` to cancel
- [x] 7.3 Add `fn send_session_reply(app: &mut App)` that spawns `claude --print --continue -p "<message>"` in the selected session's working directory, wires a new mpsc reader thread, updates session status to Running, and clears `session_reply_input`

## 8. Keybinding wiring

- [x] 8.1 Wire `KeyCode::Char('C')` in Normal mode: call `populate_session_dir_picker`, store selected task context in `app.session_pending_context`, enter `Mode::SessionDirectoryPicker`
- [x] 8.2 Wire `KeyCode::Char('C')` in Notes view (`Mode::Normal` with `View::Notes`): store selected note context, enter `Mode::SessionDirectoryPicker`
- [x] 8.3 Update footer hint strings: add `C:claude` to Normal mode and Notes view footers
- [x] 8.4 Route `Mode::SessionDirectoryPicker`, `Mode::Sessions`, `Mode::SessionReply` through the main key-dispatch match in the event loop

## 9. Cleanup on exit

- [x] 9.1 On TUI exit (before `disable_raw_mode` / `LeaveAlternateScreen`), iterate `app.claude_sessions` and call `child.kill()` on any session with `status == Running` and a live `child` handle

## 10. Session persistence

- [x] 10.1 Add `#[derive(Serialize, Deserialize)]` to `ClaudeSession` and `ClaudeSessionStatus`; annotate `child` and `rx` fields with `#[serde(skip)]`
- [x] 10.2 Add `fn session_dir(task_dir: &Path) -> PathBuf` returning `<task-dir>/claude-sessions/`
- [x] 10.3 Add `fn save_session(task_dir: &Path, session: &ClaudeSession) -> Result<(), String>` that creates the directory if absent, writes the session to `<id>-<slug>.json`, then deletes oldest files if count > 30
- [x] 10.4 Add `fn load_sessions(task_dir: &Path) -> Vec<ClaudeSession>` that reads all `*.json` files, deserializes them, and forces status to never be `Running`
- [x] 10.5 Call `save_session` in the event loop after each session transitions to `WaitingForInput`, `Done`, or `Failed`
- [x] 10.6 Call `load_sessions` during `App` initialization and populate `app.claude_sessions` with the results

## 11. Tests

- [x] 11.1 Unit test `build_session_context`: task title + description produces expected string
- [x] 11.2 Unit test `claude_code_dir()`: returns default `~/code` when config key absent; returns configured path when set
- [x] 11.3 Unit test session output ring buffer: pushing >500 lines drops oldest entries
- [x] 11.4 Unit test `ClaudeSessionStatus` transitions: `Running → WaitingForInput` on `SessionEvent::Done`; `Running → Failed` on `SessionEvent::Error`
- [x] 11.5 Unit test `save_session` / `load_sessions` round-trip: saved session deserializes with correct fields; loaded session status is never `Running`
- [x] 11.6 Unit test retention: saving a 31st session deletes the oldest file
