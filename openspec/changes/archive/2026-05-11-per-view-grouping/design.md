## Context

Currently `App` holds a single `group_by: GroupBy` field. The `G` key cycles it and writes `group-by` to config. On launch, the single key is read and applied to whatever view is active.

There are 4 views: `Due`, `NoDueDate`, `Recurring`, `Notes`. Each needs its own grouping stored and restored independently.

## Goals / Non-Goals

**Goals:**
- Each view remembers its own `GroupBy`, saved to config and restored on launch
- `GroupBy::DueDate` added — groups by `task.due_date` formatted as `YYYY-MM-DD` (or `"No Due Date"`)
- `G` key cycle extended: `none → project → agent → priority → due-date → none`
- Switching views immediately applies that view's saved grouping
- Backward-compatible config migration: if old `group-by` key exists, ignore it (just defaults to `None`)

**Non-Goals:**
- Per-view-window grouping (Due view sub-windows don't have separate groupings)
- Configurable cycle order

## Decisions

**`HashMap<View, GroupBy>` in App**
Replace `group_by: GroupBy` with `view_groupings: HashMap<View, GroupBy>`. Active grouping is always `view_groupings[app.view]`. This is the minimal change that supports per-view storage.

**Config key scheme: `group-by.<view-name>`**
Keys: `group-by.due`, `group-by.no-due-date`, `group-by.recurring`, `group-by.notes`. Derived from `View::to_config_str()`. Simple, inspectable, human-editable.

**Old `group-by` key silently ignored**
No migration needed — unrecognised config keys are already ignored by the parser.

**`GroupBy::DueDate` renders groups as date strings**
In `draw_table` grouped rendering: `GroupBy::DueDate` key = `task.due_date.map(|d| d.format("%Y-%m-%d").to_string())`. `None` date = key `None` → displayed as `"No Due Date"`. Sorts chronologically (ascending), `None` last.

**View-switch updates active grouping**
When `v`/`V` changes the view, `app.group_by` (used by drawing code) is updated to `view_groupings[new_view]`. This requires a helper: `app.active_group_by() -> GroupBy` OR simply keep a derived field updated on every view change.

Simplest approach: keep `app.group_by` as the active field, but on every view switch AND on G-press, also write to `view_groupings[current_view]` and read from `view_groupings[new_view]`.

## Risks / Trade-offs

- [Risk] `View` doesn't implement `Hash` yet — needs `#[derive(Hash)]`. Low risk, trivial to add.
- [Risk] DueDate grouping sort: currently group keys sort alphabetically. Date strings `YYYY-MM-DD` sort correctly alphabetically, so no special sort logic is needed. `None` sorts last via existing `(None, _) => Greater` logic.

## Migration Plan

No migration needed. Old `group-by` key is ignored. All per-view keys default to `None` on first launch.
