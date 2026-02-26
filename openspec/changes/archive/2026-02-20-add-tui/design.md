## Context

The task manager is a single-binary Rust CLI (~210 lines of core logic) that stores tasks in a Markdown file (`tasks.md`). The architecture is straightforward: `cli.rs` defines clap subcommands, `main.rs` dispatches them, `storage.rs` handles file I/O with advisory locking, `parser.rs` round-trips the Markdown format, `task.rs` holds data types, and `output.rs` formats human/JSON output. All mutation follows a load-modify-save cycle through `storage::load` / `storage::save`.

The TUI needs to layer an interactive, full-screen interface on top of this existing infrastructure without altering any CLI behavior.

## Goals / Non-Goals

**Goals:**
- Provide a keyboard-driven dashboard for browsing, filtering, and mutating tasks
- Reuse the existing `storage` and `parser` layers so the TUI and CLI always agree on file format
- Keep the TUI self-contained — a single new module that existing code never depends on
- Ship a usable first version (list view, navigation, toggle done, filter, add/delete) without over-engineering

**Non-Goals:**
- Multi-pane or split-view layouts (keep it single-list for v1)
- Mouse support (keyboard-only is sufficient for a developer-focused tool)
- Real-time file-watching or external change detection (reload is manual)
- Undo/redo within the TUI session
- Inline description editing (descriptions can still be edited via `task edit` or direct file edits)

## Decisions

### 1. Terminal framework: `ratatui` + `crossterm`

**Choice**: Use `ratatui` (the maintained fork of `tui-rs`) with `crossterm` as the backend.

**Why over alternatives**:
- `ratatui` is the de-facto standard for Rust terminal UIs — active maintenance, large ecosystem of examples, and widget library (Table, Paragraph, Block, etc.)
- `crossterm` is pure-Rust and cross-platform (macOS, Linux, Windows) with no C dependencies, keeping the build simple
- Alternative `termion` is Linux/macOS only; `ncurses` requires C linkage
- The existing project has zero C dependencies — `crossterm` preserves that property

### 2. Module structure: single `src/tui.rs` file

**Choice**: Start with a single `src/tui.rs` module rather than a `src/tui/` directory tree.

**Why**: The TUI's responsibilities (event loop, state, rendering) are tightly coupled and modest in size for v1. A single file avoids premature decomposition. If the module grows past ~500 lines, it can be refactored into a directory with `mod.rs`, `state.rs`, `ui.rs`, `event.rs` submodules — but that's a future concern.

### 3. State management: in-memory `TaskFile` with explicit save

**Choice**: Load the `TaskFile` into memory at startup. Mutations update the in-memory state and immediately write back to disk via `storage::save`.

**Why**:
- Matches the existing CLI pattern (load → mutate → save) so locking and atomicity guarantees carry over
- Avoids introducing a separate "dirty state" tracker or deferred flush
- The file is small (task lists are typically <1000 entries) so full re-serialization on every change is negligible

**Alternative considered**: Batch saves on a timer or at exit — rejected because a crash would lose mutations, and the current atomic-rename save is fast enough.

### 4. App state struct

**Choice**: A single `App` struct holds all TUI state:
- `task_file: TaskFile` — the loaded data
- `file_path: PathBuf` — for saving back
- `selected: usize` — cursor index into the filtered list
- `filter: Filter` — current status/priority/tag filter
- `mode: Mode` — enum: `Normal`, `Adding`, `Filtering`, `Confirming(action)`
- `input_buffer: String` — text input for add/filter modes

**Why**: A flat struct keeps state management obvious. Modes are an enum so the event handler can pattern-match cleanly without boolean soup.

### 5. Event loop architecture

**Choice**: Single-threaded blocking event loop using `crossterm::event::poll` with a 250ms timeout.

**Why**:
- The TUI doesn't need async I/O or background work — it only reacts to keyboard input
- A poll timeout lets us redraw on terminal resize events without busy-waiting
- No need for `tokio` or a channel-based event system — that's complexity for features we don't need (no network, no file-watching)

### 6. Keybindings

**Choice**: Vim-inspired navigation as the primary scheme, with arrow keys as aliases:

| Key | Action |
|-----|--------|
| `j` / `↓` | Move cursor down |
| `k` / `↑` | Move cursor up |
| `Enter` / `Space` | Toggle done/open |
| `a` | Add new task (enters input mode) |
| `d` | Delete task (with confirmation) |
| `f` | Open filter prompt |
| `/` | Alias for filter |
| `Esc` | Cancel current mode / clear filter |
| `q` | Quit |

**Why**: Developer-focused tool — vim keys are expected. Arrow keys as fallback keeps it accessible. Single-letter bindings for actions keep the interface fast.

### 7. Rendering layout

**Choice**: Three-region layout:
1. **Header bar** (1 line): Title + active filter summary
2. **Task table** (fills remaining space): Scrollable list with columns — ID, status checkbox, priority, title, tags
3. **Footer bar** (1 line): Context-sensitive help hints (e.g., "j/k:nav  Enter:toggle  a:add  d:delete  f:filter  q:quit")

**Why**: Matches the information density of `task list` output while adding interactivity. The footer doubles as a mode indicator (shows input prompts during add/filter modes).

### 8. Filter behavior

**Choice**: Filtering works as a live narrowing of the displayed list. The filter prompt accepts expressions like `status:open`, `priority:high`, `tag:frontend`, or combinations. Clearing filter (Esc) shows all tasks.

**Why**: Matches the existing CLI `--status`, `--priority`, `--tag` filter semantics. Keeping the same filter vocabulary means users don't need to learn a new syntax.

### 9. CLI integration: `Tui` variant in `Command` enum

**Choice**: Add `Tui` to the `Command` enum in `cli.rs`. In `main.rs`, the `Command::Tui` arm calls `tui::run(path)`, which takes ownership of the terminal and runs the event loop. On exit, terminal state is restored.

**Why**: Minimal surface area — one new enum variant, one new match arm, one function call. The `--file` and `TASK_FILE` resolution happens before dispatch, so the TUI gets the resolved path like every other command.

### 10. Terminal cleanup on panic

**Choice**: Install a panic hook that restores the terminal before printing the panic message.

**Why**: If the TUI panics without restoring the terminal, the user's shell is left in raw mode (no echo, no line editing). A custom panic hook calls `crossterm::terminal::disable_raw_mode()` and `execute!(stdout, LeaveAlternateScreen)` before delegating to the default handler.

## Risks / Trade-offs

**Binary size increase** → `ratatui` + `crossterm` add ~200-400KB to the release binary. Acceptable for the functionality gained. Mitigation: these are the only new dependencies; no transitive bloat.

**File contention with concurrent CLI usage** → If a user runs `task done 3` in another terminal while the TUI is open, the TUI won't see the change until it reloads. The TUI's save could also overwrite the CLI's change. → Mitigation: The existing advisory lock in `storage::save` prevents simultaneous writes from corrupting the file. For v1, this is acceptable — users can press a reload key to refresh. File-watching can be added later.

**Raw mode shell corruption on unexpected exit** → SIGKILL or power loss could leave the terminal in raw mode. → Mitigation: Panic hook handles most cases. For SIGKILL, `reset` command restores the terminal — this is standard for any TUI application.

**Feature creep** → The TUI could grow unboundedly (description editing, drag-and-drop reordering, color themes). → Mitigation: The non-goals above explicitly scope v1. The `Mode` enum makes it easy to add new modes incrementally without restructuring.
