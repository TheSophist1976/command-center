## Why

Currently there is a single global `group-by` setting that applies to all views. Switching from the Due view (where you might want grouping by agent) to the Recurring view (where grouping doesn't make sense) resets your context. Each view has different data and different grouping needs — they should remember their own grouping independently, persisted across sessions.

Additionally, "group by due date" is missing as a grouping option. Grouping by due date (day) lets you see how tasks cluster across time, which is useful in the All or NoDueDate-adjacent views.

## What Changes

- `GroupBy` gains a new variant: `DueDate` (groups tasks by their due date string, or "No Due Date")
- `G` key cycles through: `none → project → agent → priority → due-date → none`
- Per-view grouping: each `View` variant has its own `GroupBy`, stored independently in config
  - Config keys: `group-by.due`, `group-by.no-due-date`, `group-by.recurring`, `group-by.notes`
  - Default for all views: `GroupBy::None`
- Global `group-by` config key is removed; replaced by per-view keys
- When switching views, the active grouping switches to that view's saved grouping
- Saving a grouping change writes only the current view's key

## Capabilities

### New Capabilities
- `tui-group-by-due-date`: Group tasks by due date value (YYYY-MM-DD or "No Due Date")

### Modified Capabilities
- `tui-views`: View switching now also restores that view's saved grouping
- `app-config`: `group-by` global key replaced by `group-by.<view>` per-view keys

## Impact

- `src/tui.rs`: `GroupBy` enum gains `DueDate`; `App` stores `HashMap<View, GroupBy>` instead of single `GroupBy`; view switch handler restores per-view grouping; `G` key cycle adds `DueDate`; config read/write updated
- `openspec/specs/app-config/spec.md`: delta for new config keys
- `openspec/specs/tui-views/spec.md`: delta for view-switch grouping restore
