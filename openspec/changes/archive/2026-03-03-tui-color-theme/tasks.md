## Tasks

- [x] Add `theme` module with color constants inside `src/tui.rs`
- [x] Replace header/footer bar colors with `theme::BAR_FG` and `theme::BAR_BG`
- [x] Replace selection highlight with `theme::HIGHLIGHT_BG`
- [x] Replace priority colors with theme constants
- [x] Replace chat panel colors with theme constants
- [x] Replace detail panel active field highlight with `theme::HIGHLIGHT_BG`
- [x] Add done-task row styling: render done tasks with `theme::DONE_TEXT` foreground
- [x] Add overdue row treatment: render entire row in `theme::OVERDUE` color and show `[!]` status marker for open tasks past due
- [x] Verify all existing TUI tests pass
