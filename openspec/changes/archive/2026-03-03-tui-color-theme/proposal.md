## Why

The TUI currently uses hardcoded colors scattered throughout draw functions. The color choices are functional but basic — cyan header/footer bars, DarkGray selection highlight, and priority-colored cells. There's no cohesive visual identity, and the hardcoded approach makes it difficult to adjust colors consistently.

This change introduces a centralized color theme module that defines all TUI colors in one place, applies a more polished and cohesive palette, and makes future theme adjustments trivial.

## Capabilities

### New Capabilities

- **tui-theme**: A centralized color theme definition for the TUI, providing named color constants used by all draw functions.

### Modified Capabilities

- **tui**: Draw functions will reference the theme module instead of hardcoding `Color::*` values inline.

## Impact

- `src/tui.rs`: All `Color::*` and `Style::*` references in draw functions will be replaced with theme constants
- No new dependencies required (ratatui `Color` already supports RGB and indexed colors)
- No behavioral changes — purely visual refinement
