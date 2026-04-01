## 1. Add DueWindow enum and App state

- [x] 1.1 Add `DueWindow { Day, Week, Month, Year, All }` enum with `next()`, `prev()`, and `label()` methods
- [x] 1.2 Add `due_window: DueWindow` field to `App` (default: `Day`)
- [x] 1.3 Add `group_by: GroupBy` field to `App` where `GroupBy { None, Agent, Project, Priority }` (default: `None`)
- [x] 1.4 Initialize both fields in `App::new` — read `group-by:` from config for `group_by`

## 2. Refactor the View enum

- [x] 2.1 Remove `View::Today`, `View::All`, `View::Weekly`, `View::Monthly`, `View::Yearly`, `View::ByAgent`
- [x] 2.2 Add `View::Due`; update `View::next()` and `View::prev()` cycle to: Due → NoDueDate → Recurring → Notes → Due
- [x] 2.3 Update `View::display_name()` — Due returns dynamic label via `app.due_window.label()`; update other names
- [x] 2.4 Update `View::from_config()` — map `"due"`, `"today"`, `"all"`, `"weekly"`, `"monthly"`, `"yearly"` all to `View::Due`; remove `"by-agent"`
- [x] 2.5 Update `View::matches()` — add `View::Due` branch that delegates to `app.due_window`; remove removed variants

## 3. Wire `[` and `]` keybindings

- [x] 3.1 In `handle_normal`, add `KeyCode::Char(']')` to expand `app.due_window` to next level (only when `View::Due` is active)
- [x] 3.2 Add `KeyCode::Char('[')` to shrink `app.due_window` to previous level (only when `View::Due` is active)
- [x] 3.3 Update footer hints to include `[/]:window` when `View::Due` is active

## 4. Update draw_header for Due view

- [x] 4.1 In `draw_header`, when `app.view == View::Due`, show `Due [<window label>]` instead of a static name

## 5. Remove ByAgent rendering, integrate grouping into draw_table

- [x] 5.1 Delete `draw_agent_grouped_table` function
- [x] 5.2 Remove the `View::ByAgent` layout branch from `draw()`
- [x] 5.3 Integrate grouped rendering into `draw_table`: when `app.group_by != GroupBy::None`, build group buckets and inject section header rows before rendering
- [x] 5.4 Compute TableState render index offset from injected headers (reuse the logic from the deleted function)

## 6. Wire `G` keybinding and `:group` command

- [x] 6.1 Add `KeyCode::Char('G')` in `handle_normal` to cycle `app.group_by` (None → Project → Agent → Priority → None) and save to config
- [x] 6.2 In the `:` command dispatcher, handle `group <field>` — parse the field, set `app.group_by`, save `group-by: <field>` to config
- [x] 6.3 Show an error status message for unknown field names in `:group`
- [x] 6.4 Update footer hints to include `G:group`

## 7. Configurable columns

- [x] 7.1 Add `ColumnId` enum with variants for each supported column: Id, Status, Priority, Title, Desc, Due, Project, Agent, Recur, Note, Tags
- [x] 7.2 In `App::new`, read `columns:` from config and parse into `Vec<ColumnId>`; store as `app.columns`
- [x] 7.3 Refactor `draw_table` to iterate `app.columns` (when non-empty) instead of the `show_*` boolean logic
- [x] 7.4 When `app.columns` is empty (config key absent), fall back to existing auto-show logic

## 8. Update all tests

- [x] 8.1 Update view cycle tests (`next_cycles_through_all_views`, `prev_cycles_through_all_views`) to use new 4-view cycle
- [x] 8.2 Update any tests that construct `App` structs with removed view fields (`pending_nlp_update` etc. — check for compile errors)
- [x] 8.3 Update or remove tests that reference `View::Today`, `View::All`, `View::Weekly`, `View::Monthly`, `View::Yearly`, `View::ByAgent`

## 9. Verify and clean up

- [x] 9.1 `cargo build` — clean
- [x] 9.2 `cargo test` — no regressions
- [x] 9.3 Remove dead imports and constants left over from removed view variants
