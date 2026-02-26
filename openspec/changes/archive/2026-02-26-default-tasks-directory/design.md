## Context

The task manager currently resolves the tasks file via: CLI `--file` flag → `TASK_FILE` env var → hardcoded `"tasks.md"` (current directory). There is no persistent user preference. Users who always work in a fixed tasks directory must either `cd` to it, set `TASK_FILE` in their shell profile, or pass `--file` every time.

`task tui` passes whatever path `storage::resolve_file_path` returns, so adding a config layer that sits between the env var and the hardcoded fallback is the cleanest insertion point.

Config is stored at `{config_dir}/task-manager/config.md` — the same directory already used for the Todoist token (`src/auth.rs`).

## Goals / Non-Goals

**Goals:**
- Persist a default tasks directory in a human-readable config file
- `task tui` (and all other commands when no `--file`/`TASK_FILE` is given) uses `<default-dir>/tasks.md` as the fallback
- `task config set default-dir <path>` writes the setting; `task config get default-dir` reads it
- TUI exposes a keybinding (`D`) to update the default directory interactively
- Config file format is plain markdown (readable/editable by hand)

**Non-Goals:**
- Supporting multiple named profiles or multiple config keys beyond `default-dir`
- Watching the config file for live changes during a TUI session
- Migrating the `TASK_FILE` env var — it continues to take priority over config

## Decisions

### 1. Config file format: markdown key-value, not TOML/JSON

**Decision:** Store config as a simple markdown file:
```
# task-manager config

default-dir: /Users/alice/notes
```

**Rationale:** Consistent with the project's use of markdown for task files; human-readable and editable without extra tooling. Only one key is needed now so a full structured format is premature.

**Alternative considered:** TOML — better for multiple keys, but introduces a new file format into a markdown-first project.

### 2. Insertion point: between env var and hardcoded fallback

**Decision:** `storage::resolve_file_path` gains a third fallback tier: CLI flag → `TASK_FILE` env var → **config default-dir** → `"tasks.md"`.

**Rationale:** All commands go through `resolve_file_path`, so one change covers CLI and TUI uniformly. No callers need to change.

**Alternative considered:** Read config only in `Command::Tui` — would require duplicating the resolution logic and wouldn't benefit CLI commands.

### 3. TUI keybinding: `D` in Normal mode opens a directory prompt

**Decision:** Pressing `D` in Normal mode shows an input prompt at the bottom of the screen. On confirm, the new path is written to config and the TUI reloads the task file from the new location.

**Rationale:** Inline editing is consistent with how the TUI already handles task edits (bottom-bar input prompts). No modal dialogs or external processes needed.

**Alternative considered:** A separate `task config set` command only — works for CLI users but not for users who stay in the TUI.

### 4. New `src/config.rs` module

**Decision:** Config read/write lives in a dedicated `config.rs`, parallel to `auth.rs`.

**Rationale:** Keeps `storage.rs` focused on task file I/O. The config module can be tested independently with path-parameterized helpers (same pattern as `auth.rs`).

### 5. No new crate dependencies

**Decision:** Parse the markdown config with simple line scanning (`contains("default-dir:")`) rather than a markdown parser.

**Rationale:** One key, simple format. A full parser would be over-engineering. Consistent with the project's minimal-dependency stance.

## Risks / Trade-offs

- **User edits config file by hand with bad path** → `resolve_file_path` will return that path; `storage::load` will return an empty `TaskFile` (existing behaviour for missing files). No crash, but silent empty state. Mitigation: validate path exists when reading config (warn but don't error).
- **Config dir unavailable** → `dirs::config_dir()` returns `None` on some exotic platforms. Mitigation: treat as "no config" and fall through to `"tasks.md"` default; same pattern as `auth.rs`.
- **TUI reload on directory change** — reloading the task file in-place may discard unsaved in-memory edits. Mitigation: save current state before reloading.

## Open Questions

- None — scope is small and well-defined.
