## Context

The current note editor is a custom `NoteEditor` struct backed by `Mode::EditingNote` and `Mode::ConfirmingNoteExit`. It supports basic text entry and save/discard confirmation. Notes are stored as `.md` files in `<task-dir>/Notes/`. The TUI is a Crossterm/Ratatui full-screen application that owns the terminal.

## Goals / Non-Goals

**Goals:**
- Open notes in `$EDITOR` (suspend TUI → spawn editor → resume TUI)
- Open notes via Obsidian URI (`obsidian://open?vault=...&file=...`) using `open` on macOS
- Preview notes read-only with `glow` when `note-viewer: glow` is configured
- Show a clear, actionable error when neither `$EDITOR` nor Obsidian is configured
- Remove the built-in inline editor entirely

**Non-Goals:**
- Supporting editors other than what `$EDITOR` points to
- Two-way sync between the task notes directory and an Obsidian vault (notes live in one place)
- Auto-detecting Obsidian installation
- Windows support for Obsidian URI launch (macOS `open` only)

## Decisions

### Priority order: Obsidian > `$EDITOR`
If `obsidian-vault` is configured, pressing the open key launches Obsidian regardless of `$EDITOR`. This lets Obsidian users always get their native experience. Users who want the terminal editor can leave `obsidian-vault` unset.

### Glow is a viewer, not an editor
`note-viewer: glow` controls the **view** path only. Glow is launched with `glow <path>` (which pages the output). The edit path (`$EDITOR`) is separate. If both are configured, the TUI could offer a key split (e.g. `Enter` = view with glow, `e` = edit with `$EDITOR`), but for simplicity: `g` always opens for editing (Obsidian or `$EDITOR`); a separate `G`-view key (or `v` in Notes view) launches glow when configured.

Actually: simplest and most intuitive — `g` (go-to-note) opens for editing. A separate key `V` in Notes view or detail panel launches glow. No mode split needed.

### TUI suspend/resume for `$EDITOR`
Crossterm raw mode must be disabled before spawning the editor and re-enabled after. Use `crossterm::terminal::disable_raw_mode()` + `crossterm::execute!(stdout, LeaveAlternateScreen)` before spawn, and the reverse after the child process exits. This is the standard pattern for terminal apps that launch editors (used by git, lazygit, etc.).

### Obsidian `obsidian-notes-dir` maps task notes to vault
Notes live at `<task-dir>/Notes/<slug>.md`. Obsidian expects a path relative to the vault root. Config key `obsidian-notes-dir` provides the relative path prefix (e.g. `Tasks/Notes`). Full URI: `obsidian://open?vault=<vault>&file=<obsidian-notes-dir>/<slug>`.

If `obsidian-notes-dir` is not set, use just the slug as the file path.

### New note flow
When the user creates a new note (`a` in Notes view):
1. Prompt for title (existing input flow)
2. Create the empty `.md` file with just the `# <title>` header
3. Open in the external editor/Obsidian immediately
4. On return, refresh the notes list

### Error message
If `$EDITOR` is unset AND `obsidian-vault` is unset: show status message `"No editor configured. Set $EDITOR or add obsidian-vault to config."` — no crash, no partial action.

## Risks / Trade-offs

- [Risk: Editor crashes or is not found] → Child process exit code is ignored; TUI resumes normally. The user will see the terminal briefly or get an OS error before TUI resumes.
- [Risk: Obsidian URI silently fails if app not installed] → macOS `open` will show an error in the background; TUI is unaffected.
- [Risk: Terminal state corruption if editor doesn't exit cleanly] → Crossterm's cleanup on resume handles most cases.

## Open Questions

None.
