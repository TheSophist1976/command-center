## 1. Data Model

- [x] 1.1 Replace `note: Option<String>` with `notes: Vec<String>` on `Task` struct in `src/task.rs`
- [x] 1.2 Update `Task` default/construction sites: `note: None` → `notes: vec![]` everywhere

## 2. Parser — Serialization & Deserialization

- [x] 2.1 Parse `notes:slug1,slug2` key in `src/parser.rs` → `Vec<String>` (split on comma, filter empty)
- [x] 2.2 Parse legacy `note:slug` key → `vec!["slug"]` (single-element backward compat)
- [x] 2.3 Serialize `notes` key in `src/parser.rs` — comma-join; omit key when empty
- [x] 2.4 Add round-trip tests: single note, multiple notes, legacy `note:` parse, empty list

## 3. CLI — `task note link` / `task note unlink`

- [x] 3.1 Update `task note link <slug> <id>` in `src/bin/task.rs`: append slug to `notes` if not present (was: replace single `note`)
- [x] 3.2 Update `task note unlink <id>` in `src/bin/task.rs`: if one note → clear it; if multiple → print slugs and require slug argument; add `--slug <slug>` flag to `NoteCommand::Unlink`

## 4. TUI — Display

- [x] 4.1 Update note indicator in task list: show `[N]` count badge when `notes.len() > 1`; show existing indicator when `notes.len() == 1`; show nothing when empty
- [x] 4.2 Update detail panel to show all note slugs (comma-separated or one per line)
- [x] 4.3 Update `show_note` auto-column logic to use `notes.len() > 0`

## 5. TUI — `n` key multi-note picker

- [x] 5.1 Update the `n`-key handler: if `notes` is empty or action is "Add note", show existing note-picker for linking; if notes present, show multi-note picker with attached slugs + "Add note" + "Remove note" options
- [x] 5.2 Selecting an attached slug → open that note in the inline editor
- [x] 5.3 Selecting "Add note" → show note-picker, append chosen slug to `notes`
- [x] 5.4 Selecting "Remove note" → show picker of attached slugs, remove chosen slug
- [x] 5.5 Add `note_multi_picker_mode` state to `App` to distinguish add/remove/navigate actions

## 6. TUI — `g` key multi-note navigation

- [x] 6.1 Update `g`-key handler: if `notes.len() == 1` open directly (existing behavior); if `notes.len() > 1` show slug picker first; if empty → no-op

## 7. Skill & Documentation Updates

- [x] 7.1 Update `~/.claude/skills/work-agent-tasks/SKILL.md`: in standing subagent instructions, add step to read all attached notes before Phase 0; flag instruction notes (slug/title contains `instructions`, `how-to`, `steps`) as highest priority reads
- [x] 7.2 Update each agent's `instructions.md` (research, follow-up, writer, reviewer, automator): add guidance that task notes should be read before starting work, and instruction notes must be read first
- [x] 7.3 Update `AGENTS.md`: document `notes` field (replaces `note`), comma-separated slugs, instruction note naming convention
