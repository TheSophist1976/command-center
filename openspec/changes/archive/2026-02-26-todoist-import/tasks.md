## 1. Dependencies

- [x] 1.1 Add `reqwest` (features: blocking, json), `serde_json`, `tiny_http`, `open`, `dirs` to `Cargo.toml`
- [x] 1.2 Declare `mod auth` and `mod todoist` in `src/main.rs`

## 2. Data Model (`src/task.rs`)

- [x] 2.1 Add `Critical` variant to `Priority` enum above `High`; update `Display` and `FromStr` to handle `"critical"`
- [x] 2.2 Add `due_date: Option<NaiveDate>` and `project: Option<String>` fields to `Task` struct with `#[serde(skip_serializing_if = "Option::is_none")]`

## 3. Parser & Serializer (`src/parser.rs`)

- [x] 3.1 Update format version check to accept both `1` and `2` (currently hard-rejects anything other than `1`)
- [x] 3.2 Parse `due` and `project` keys in `parse_metadata_comment`; map `due` to `NaiveDate::parse_from_str` (format `%Y-%m-%d`), `project` to `String`
- [x] 3.3 Serialize `due_date` and `project` into the metadata comment in `serialize` (omit when `None`)
- [x] 3.4 Bump `format_version` to `2` in `serialize` (write `<!-- format:2 -->` header regardless of input version)

## 4. CLI (`src/cli.rs`)

- [x] 4.1 Add `--due <date>` (`Option<String>`) and `--project <name>` (`Option<String>`) flags to `Add` and `Edit` variants
- [x] 4.2 Add `--project <name>` (`Option<String>`) filter flag to `List` variant
- [x] 4.3 Add `Auth` subcommand group with sub-variants: `Todoist`, `Status`, `Revoke`
- [x] 4.4 Add `Import` subcommand group with sub-variant `Todoist { #[arg(long)] test: bool }`
- [x] 4.5 Add `Migrate` subcommand

## 5. Auth Module (`src/auth.rs`)

- [x] 5.1 Implement `token_path() -> PathBuf` using `dirs::config_dir()` to resolve `~/.config/task-manager/todoist_token`
- [x] 5.2 Implement `read_token() -> Option<String>` and `write_token(token: &str)` (set 0600 permissions after write on Unix)
- [x] 5.3 Implement `delete_token()` for the revoke flow
- [x] 5.4 Implement `run_oauth_flow(client_id: &str, client_secret: &str) -> Result<String, String>`: build Todoist authorization URL, open browser via `open::that()`, spin up `tiny_http` listener on `127.0.0.1:7777`, capture `?code=` from redirect
- [x] 5.5 Implement `exchange_code(code: &str, client_id: &str, client_secret: &str) -> Result<String, String>`: POST to Todoist token endpoint, extract `access_token`

## 6. Todoist API Client (`src/todoist.rs`)

- [x] 6.1 Define `TodoistTask` struct matching REST API v2 response fields: `id`, `content`, `description`, `priority`, `labels`, `due` (nested struct with `date` string), `project_id`
- [x] 6.2 Implement `fetch_open_tasks(token: &str) -> Result<Vec<TodoistTask>, String>` via `GET https://api.todoist.com/rest/v2/tasks`
- [x] 6.3 Implement `fetch_projects(token: &str) -> Result<HashMap<String, String>, String>` via `GET https://api.todoist.com/rest/v2/projects`, returning `id → name` map
- [x] 6.4 Implement `map_priority(p: u8) -> Priority`: `1→Critical`, `2→High`, `3→Medium`, `4→Low`
- [x] 6.5 Implement `map_task(t: &TodoistTask, project_map: &HashMap<String, String>) -> Task`: map all fields, add `"imported"` tag, assign id=0 (caller assigns real id)
- [x] 6.6 Implement `label_exported(token: &str, task_id: &str, existing_labels: &[String]) -> Result<(), String>`: POST updated labels list including `"exported"` to `https://api.todoist.com/rest/v2/tasks/{id}`
- [x] 6.7 Implement `run_import(token: &str, task_file: &mut TaskFile, test_mode: bool) -> Result<(usize, usize), String>`: orchestrate fetch → filter already-exported → optionally limit to 3 in test mode → map → append → label in Todoist (skip labeling in test mode); return `(imported, skipped)` counts

## 7. Main Dispatch (`src/main.rs`)

- [x] 7.1 Update `Add` handler: parse `--due` to `NaiveDate`, pass `due_date` and `project` when constructing `Task`
- [x] 7.2 Update `Edit` handler: apply `--due` and `--project` changes when provided
- [x] 7.3 Update `List` handler: apply `--project` filter
- [x] 7.4 Add `Migrate` handler: load file, set `task_file.format_version = 2`, save, print confirmation
- [x] 7.5 Add `Auth::Todoist` handler: read `TODOIST_CLIENT_ID`/`TODOIST_CLIENT_SECRET` env vars, run OAuth flow, write token, print confirmation
- [x] 7.6 Add `Auth::Status` handler: check token path exists, print status message
- [x] 7.7 Add `Auth::Revoke` handler: delete token, print confirmation
- [x] 7.8 Add `Import::Todoist` handler: read token, load task file, call `todoist::run_import`, save file, print summary

## 8. Output (`src/output.rs`)

- [x] 8.1 Add `due_date` and `project` columns to `print_task_table` (only render columns when at least one task has the field set)
- [x] 8.2 Add `due_date` and `project` lines to `print_task_detail`
- [x] 8.3 Include `due_date` and `project` in `task_to_json`

## 9. TUI (`src/tui.rs`)

- [x] 9.1 Add `Critical` arm to priority color mapping (bright magenta) in the table renderer
- [x] 9.2 Add `'c'` key to `handle_priority` to set `Priority::Critical`; update footer picker hint to show `c/h/m/l`
- [x] 9.3 Add `due_date` and `project` columns to the task table (shown when at least one visible task has the field set)
- [x] 9.4 Add `project:<name>` filter parsing in `apply_filter`; add `priority:critical` support if not already handled

## 10. Verification

- [x] 10.1 Run `cargo build` with zero errors and zero warnings
- [x] 10.2 Smoke test auth flow: `task auth status` (no token), `task auth revoke` (no token)
- [x] 10.3 Smoke test new fields: `task add "Test" --due 2025-12-01 --project Work && task show <id>`
- [x] 10.4 Smoke test critical priority: `task add "Urgent" --priority critical && task list`
- [x] 10.5 Smoke test migrate: `task migrate` on existing tasks.md; verify `format:2` header written
- [ ] 10.6 Smoke test test-mode import: `task import todoist --test` (after auth)
