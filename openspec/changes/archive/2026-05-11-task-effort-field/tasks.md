## 1. Data Model

- [x] 1.1 Add `Effort` enum (`High`, `Medium`, `Low`) to `src/task.rs`
- [x] 1.2 Add `effort: Option<Effort>` field to `Task` struct in `src/task.rs`
- [x] 1.3 Update `Task::new()` / default construction to set `effort: None`

## 2. Parser — Serialization & Deserialization

- [x] 2.1 Parse `effort` key in metadata comment in `src/parser.rs` (`effort:high` → `Some(Effort::High)`, unknown values → `None`)
- [x] 2.2 Serialize `effort` key in `src/parser.rs` (omit when `None`)
- [x] 2.3 Add round-trip tests for effort field (parse → serialize → parse)

## 3. TUI — GroupBy

- [x] 3.1 Add `GroupBy::Effort` variant to the `GroupBy` enum in `src/tui.rs`
- [x] 3.2 Insert `Effort` into the `next()` cycle after `Priority`: `Priority → Effort → DueDate`
- [x] 3.3 Add `"effort"` to `GroupBy::to_config_str()` and `GroupBy::from_config()`
- [x] 3.4 Add grouping logic in the task list renderer for `GroupBy::Effort` (group header labels: `High`, `Medium`, `Low`, `(none)`)

## 4. TUI — Display

- [x] 4.1 Add effort column to task list display (short label: `H` / `M` / `L` / blank)
- [x] 4.2 Add `Effort:` field to task detail panel (full label or `—` when None)
- [x] 4.3 Include effort in the `show_effort` flag / column visibility logic (consistent with other optional columns)

## 5. TUI — Editing

- [x] 5.1 Add effort picker state fields to `App` struct (`effort_picker_open: bool`, `effort_picker_selected: usize`)
- [x] 5.2 Implement `handle_effort_picker()` function (mirror `handle_agent_picker` pattern)
- [x] 5.3 Bind `E` key in task edit/detail mode to open the effort picker
- [x] 5.4 Implement `draw_effort_picker()` render function
- [x] 5.5 Ensure `Esc` closes picker without change; selecting an option sets `task.effort` and closes picker; `(clear)` sets `effort: None`

## 6. Documentation

- [x] 6.1 Update `AGENTS.md` — add `effort` to the metadata fields table with valid values and description
- [x] 6.2 Update the help/keybinding hint line in the TUI to include `E:effort`
