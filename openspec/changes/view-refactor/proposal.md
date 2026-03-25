## Why

The TUI has 9 views cycling on a single `v` keybinding. Four of them (Today, Weekly, Monthly, Yearly) are variations of the same idea — "show tasks due in a time window" — making the cycle feel redundant and slow to navigate. Meanwhile, grouping is stuck to one dimension (agent), and the visible columns are hardcoded. This change consolidates the time views, makes columns user-configurable, and introduces a flexible grouping system.

## What Changes

- **BREAKING**: Remove `View::Today`, `View::Weekly`, `View::Monthly`, `View::Yearly`, and `View::All` as separate views
- Add `View::Due` — a single view with a sub-window that cycles through: Day → Week → Month → Year → All
  - `[` shrinks the window, `]` expands it
  - Day mode retains Today's behavior (due today + overdue + no-due-date tasks)
  - Header shows the active window: `Due [Today]`, `Due [This Week]`, etc.
- Add `:group <field>` command to group the task table by any field (agent, project, priority, none)
  - Grouping is saved to config and restored on startup
  - `G` quick-cycles through configured group options
- Add `columns:` config key to control which columns are shown and in what order
  - Falls back to current auto-show logic if not set

## Capabilities

### New Capabilities
- `tui-due-view`: The consolidated Due view with sub-window toggle (`[`/`]`) replacing the four time-based views

### Modified Capabilities
- `tui-views`: Remove Today/Weekly/Monthly/Yearly/All variants; add ByAgent was previous change — update the view enum and cycle
- `tui-agent-view`: Grouping is now a cross-cutting feature (`:group` + `G`), not tied to a dedicated view — `View::ByAgent` is removed and replaced by `:group agent` on any view
- `app-config`: Add `columns:` and `group-by:` config keys

## Impact

- `src/tui.rs`: View enum changes; new Due sub-window state; `:group` command handler; column config reading; remove `draw_agent_grouped_table`; adapt `filtered_indices` and `draw_table`
- `src/config.rs`: No changes needed (key-value reads already work generically)
- `config.md` (user config): New optional keys `columns:` and `group-by:`
