## 1. Config module

- [x] 1.1 Create `src/config.rs` with `config_path() -> Option<PathBuf>` using `dirs::config_dir()`
- [x] 1.2 Implement `read_config_value(key: &str) -> Option<String>` — reads config file, returns value for key
- [x] 1.3 Implement `write_config_value(key: &str, value: &str) -> Result<(), String>` — writes/updates key in config file, creates directory if absent
- [x] 1.4 Add path-parameterised helpers `read_config_value_from(path, key)` and `write_config_value_to(path, key, value)` for testability
- [x] 1.5 Register `mod config;` in `src/main.rs`

## 2. File path resolution

- [x] 2.1 Update `storage::resolve_file_path` to read `config::read_config_value("default-dir")` as the third fallback tier (after `TASK_FILE` env var, before `"tasks.md"`)
- [x] 2.2 When `default-dir` is set, return `PathBuf::from(default_dir).join("tasks.md")`

## 3. CLI subcommand

- [x] 3.1 Add `ConfigCommand` enum to `src/cli.rs` with `Set { key: String, value: String }` and `Get { key: String }` variants
- [x] 3.2 Add `Config { subcommand: ConfigCommand }` variant to the `Command` enum in `src/cli.rs`
- [x] 3.3 Handle `Command::Config` in `src/main.rs`: `Set` calls `config::write_config_value` and prints confirmation; `Get` calls `config::read_config_value` and prints the value or "not set"
- [x] 3.4 Update the `use cli::{...}` import in `src/main.rs` to include `ConfigCommand`

## 4. TUI: load from config on startup

- [x] 4.1 Confirm that `Command::Tui` in `src/main.rs` already passes the resolved path from `storage::resolve_file_path` to `tui::run` — no change needed if so, or update if not

## 5. TUI: set default directory keybinding

- [x] 5.1 Add `EditingDefaultDir` variant to the `Mode` enum in `src/tui.rs`
- [x] 5.2 In Normal mode key handler, bind `D` to enter `EditingDefaultDir` mode with the current config value pre-populated in the input buffer
- [x] 5.3 In `EditingDefaultDir` mode: `Enter` saves current task state, calls `config::write_config_value("default-dir", input)`, reloads the task file from the new path, returns to Normal mode; `Esc` cancels without change
- [x] 5.4 Update the footer hint string to include `D:set-dir` in Normal mode

## 6. Tests

- [x] 6.1 Unit tests for `config::read_config_value_from` — key present, key absent, malformed lines ignored, None path returns None
- [x] 6.2 Unit tests for `config::write_config_value_to` — creates file and dir, overwrites existing key, appends new key
- [x] 6.3 Unit test for `storage::resolve_file_path` with config value set (mock via env or path-param helper)
- [x] 6.4 Integration tests for `task config set default-dir <path>` and `task config get default-dir`
