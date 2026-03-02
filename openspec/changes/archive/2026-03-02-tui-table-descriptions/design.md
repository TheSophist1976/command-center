## Context

The TUI task table conditionally shows Due and Project columns when at least one visible task has those fields set. Descriptions are currently hidden — only accessible via `r` (edit) or the CLI. This change adds both a truncated description column in the table and a toggleable detail panel for full task inspection.

## Goals / Non-Goals

**Goals:**
- Show task descriptions in the table when any visible task has one
- Truncate long descriptions to keep the table readable
- Provide a detail panel to view all fields of the selected task
- Panel toggles with a keybinding, updates as user navigates

**Non-Goals:**
- Word-wrap or multi-line row display in the table
- Editing from the detail panel (existing `e`/`r`/`t`/`p` keys handle editing)

## Decisions

### 1. Description column: after Title, before Due/Project/Tags

Column order: ID, Status, Priority, Title, **Desc**, [Due], [Project], Tags. Placing Desc next to Title groups textual content.

### 2. Conditional column display

Same pattern as `show_due` and `show_project`: check if any filtered task has a non-None, non-empty description. Only show the column when at least one task has content.

### 3. Truncation with fixed width

Use `Constraint::Length(30)` for the Desc column. Descriptions longer than 30 characters: show first 29 characters + `…`. Truncate the string before creating the `Cell` for control over the suffix.

### 4. Detail panel: toggleable bottom panel

Add a `show_detail_panel: bool` field to `App`. When true, the Normal mode layout splits into: table (top ~70%) and detail panel (bottom ~30%). The panel shows all fields of the currently selected task:

- ID, Title, Status, Priority
- Description (full text, wrapped)
- Tags, Due Date, Project
- Created, Updated timestamps

### 5. Toggle keybinding: `Tab`

`Tab` is unused in Normal mode and is a natural "toggle panel" key. Pressing `Tab` flips `show_detail_panel`. The panel content updates automatically as the user navigates with `j`/`k`.

**Alternatives considered:**
- `?` for "info" — less discoverable
- Always-on panel — wastes screen space when not needed

### 6. Panel rendering

Use a `Paragraph` widget inside a bordered `Block` titled "Task Details". The content is formatted as key-value lines. The panel respects the current selection — if no task is selected (empty/filtered list), show "No task selected."

## Risks / Trade-offs

- **Table width pressure**: Adding the Desc column on narrow terminals may squeeze Title. Mitigated by conditional display and fixed 30-char width.
- **Panel reduces table rows**: When the detail panel is visible, ~30% of vertical space is used. Users can toggle it off when they need more table rows.
- **Tab key in other modes**: `Tab` is only handled in Normal mode. Other modes (Adding, Filtering, etc.) ignore it, so no conflicts.
