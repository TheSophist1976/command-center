## Approach

Add a `theme` submodule within `src/tui.rs` that defines all colors as constants. Replace every inline `Color::*` reference in draw functions with theme constants.

## Color Palette

A dark-background-friendly palette using ratatui `Color::Rgb`:

| Element | Current | New |
|---------|---------|-----|
| Header/footer bg | Cyan | Deep blue `Rgb(30, 60, 114)` |
| Header/footer fg | Black | White |
| Selection highlight bg | DarkGray | Muted blue `Rgb(40, 50, 80)` |
| Priority Critical | Magenta+Bold | `Rgb(255, 85, 85)` bold |
| Priority High | Red | `Rgb(255, 150, 50)` |
| Priority Medium | Yellow | `Rgb(255, 215, 0)` |
| Priority Low | Green | `Rgb(100, 200, 100)` |
| Chat user message | Cyan | `Rgb(100, 180, 255)` |
| Chat task list | Yellow | `Rgb(255, 215, 0)` |
| Chat error | Red | `Rgb(255, 85, 85)` |
| Detail active field bg | DarkGray | Muted blue (same as selection) |
| Completed task text | (none) | `Rgb(100, 100, 100)` dim |
| Overdue row fg | (none) | `Rgb(255, 85, 85)` — entire row text turns red |
| Overdue status marker | (none) | Status column shows `[!]` instead of `[ ]` |

## Structure

```rust
mod theme {
    use ratatui::style::{Color, Modifier, Style};

    // Bar (header/footer)
    pub const BAR_FG: Color = Color::White;
    pub const BAR_BG: Color = Color::Rgb(30, 60, 114);

    // Selection
    pub const HIGHLIGHT_BG: Color = Color::Rgb(40, 50, 80);

    // Priority colors
    pub const PRIORITY_CRITICAL: Color = Color::Rgb(255, 85, 85);
    pub const PRIORITY_HIGH: Color = Color::Rgb(255, 150, 50);
    pub const PRIORITY_MEDIUM: Color = Color::Rgb(255, 215, 0);
    pub const PRIORITY_LOW: Color = Color::Rgb(100, 200, 100);

    // Chat
    pub const CHAT_USER: Color = Color::Rgb(100, 180, 255);
    pub const CHAT_TASK_LIST: Color = Color::Rgb(255, 215, 0);
    pub const CHAT_ERROR: Color = Color::Rgb(255, 85, 85);

    // Task states
    pub const DONE_TEXT: Color = Color::Rgb(100, 100, 100);
    pub const OVERDUE: Color = Color::Rgb(255, 85, 85);
}
```

## Decisions

- **No config file or runtime theme switching** — keep it simple, compile-time constants only
- **No new dependencies** — use existing ratatui `Color::Rgb`
- **Dim completed tasks** — strike-through done tasks with muted color for visual distinction
- **Overdue row treatment** — open tasks past due get the entire row in red text plus a `[!]` status marker, making them impossible to miss
