## Context

The TUI currently has 9 views cycling on `v`/`V`. Five are date-window variants (Today, All, Weekly, Monthly, Yearly) of the same underlying idea. Grouping is locked to a single `View::ByAgent` that can't be used with other views. Columns are auto-shown based on data, with no user control. This change consolidates views, generalises grouping, and adds column config.

## Goals / Non-Goals

**Goals:**
- Replace Today/All/Weekly/Monthly/Yearly with one `View::Due` + sub-window toggle
- Remove `View::ByAgent`; add generic grouping on any view via `:group <field>` and `G` cycle
- Read `columns:` from config to control table column visibility and order
- Read `group-by:` from config on startup; write it when grouping changes
- Maintain all existing sort, filter, and navigation behavior

**Non-Goals:**
- Column reordering from within the TUI (config-file-only)
- Grouping by date fields (agent/project/priority only)
- Multi-level grouping (one active group at a time)

## Decisions

**`View::Due` with an enum sub-window, not a separate state variable**
A `DueWindow { Day, Week, Month, Year, All }` enum stored in `App` is cleaner than a free integer or string. It makes the `matches()` logic straightforward and the window name is derived from the enum variant. The sub-window persists across view switches so users can leave Due view and return to the same window.

**Remove `View::ByAgent`, don't keep it alongside grouping**
Having both a dedicated ByAgent view AND a generic `:group agent` command would confuse users. The generic approach is strictly more capable (works on any view), so `View::ByAgent` is removed. This shrinks the view cycle from 5 views to 4.

**Final view cycle: Due → NoDueDate → Recurring → Notes**
Four views — clean and fast to cycle. `NoDueDate` stays separate because it serves a distinct workflow (finding tasks that need scheduling), not just a different date window.

**Grouping renders inline with the existing `draw_table` path**
Rather than a separate `draw_agent_grouped_table` function (which existed for ByAgent), grouping is integrated into `draw_table` via an `app.group_by` field. When `group_by` is not `None`, the table build loop injects section header rows before each group. This removes the duplicated rendering code and the TableState selection offset logic applies uniformly.

**`:group` reuses the existing `:` command-line mode**
The `:` key already enters a command input mode (used for NLP previously, now free). The command dispatcher handles `group <field>` as a new command. This avoids adding a new mode.

**`columns:` is parsed once at startup and stored in `App`**
Parsing the config on every draw would be wasteful. On startup (and when the default-dir is changed), `App` reads the `columns:` key and builds an ordered `Vec<ColumnId>` enum. `draw_table` iterates this vec instead of the current `show_*` booleans. If the config key is absent, the vec is empty and the auto-show fallback activates.

## Risks / Trade-offs

- **Tests referencing removed views** → Any test using `View::Today`, `View::All`, etc. must be updated. Mitigation: grepping for view variants will find them all.
- **`from_config("today")` → was returning `View::Today`** → After this change it returns `View::Due` with Day window. Need to keep the mapping working gracefully.
- **Removing `View::ByAgent` is breaking** → The `ByAgent` view was added in the previous change. Any user with `default-view: by-agent` in their config will silently fall back to `View::Due`. Acceptable.

## Migration Plan

1. Add `DueWindow` enum; add `App.due_window` and `App.group_by` fields
2. Replace 5 view variants with `View::Due`; update `matches()`, `next()`, `prev()`, `display_name()`, `from_config()`
3. Remove `View::ByAgent` and `draw_agent_grouped_table`
4. Wire `[`/`]` keybindings to cycle `App.due_window`
5. Integrate grouping into `draw_table`; wire `G` and `:group` command
6. Add `ColumnId` enum; parse `columns:` config in `App::new`; refactor `draw_table` column loop
7. Read/write `group-by:` config on startup and on grouping change
8. Update all tests referencing removed view variants
