## Context

The TUI currently has views (Today, All, Weekly, etc.) that filter tasks by due date. All views render the task list as a flat sorted table. There is no way to see tasks organized by their assigned agent. The `agent` field already exists on every task; this change adds a view that makes that grouping visible.

## Goals / Non-Goals

**Goals:**
- Add `View::ByAgent` that renders tasks grouped under agent section headers
- Integrate into the existing view cycle (`v`/`V` keybindings)
- Show all open tasks (no date filtering) — the grouping IS the filter
- Within each agent group, preserve the existing due-date/priority sort order
- Groups sorted alphabetically; unassigned tasks go last under an "Unassigned" header

**Non-Goals:**
- Filtering by agent (that's the existing filter system's job)
- Modifying the `agent` field from this view (use the detail panel or agent picker)
- Showing done tasks in this view (consistent with all non-All views)

## Decisions

**Render as a flat table with injected header rows, not nested widgets**
The existing task table is a `ratatui::Table`. Inserting non-selectable section header rows inline keeps the rendering model simple and avoids a nested scroll widget. Header rows are visually styled (bold, dim background) and are skipped during selection navigation.

Alternative considered: a separate rendering path outside the table. Rejected — maintaining two render paths adds complexity and breaks consistent styling.

**`View::ByAgent` inserted between Recurring and Notes in the cycle**
Keeps Notes last (it's a different mode) and puts the new view near other "organizational" views.

**"Unassigned" group always last**
Tasks with no agent are shown at the end. This makes the named agent groups prominent and keeps unowned tasks from cluttering the top.

**Selection tracking uses logical task index, not rendered row index**
Since header rows are injected, the selection index (`app.selected`) continues to track position within the filtered task list (not the rendered row list). The draw function maps this to the correct rendered row for highlighting.

## Risks / Trade-offs

- **Injected header rows + TableState selection** → Ratatui's `TableState` tracks a row index in the rendered table. With injected headers, the rendered index ≠ task index. Mitigation: compute the rendered row index from the task index at draw time, and invert during navigation to skip header rows.
- **Long agent names** → Very long agent names could overflow the header cell. Mitigation: truncate header to available width.
