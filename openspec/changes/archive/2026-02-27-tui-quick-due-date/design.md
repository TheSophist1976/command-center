## Context

The TUI has an established pattern for single-keypress editing in Normal mode: `Enter` toggles status, `p` enters priority sub-mode. Due dates (`Option<NaiveDate>`) are currently only settable via the CLI. The user wants due dates assignable with a single keypress — no sub-mode, no extra step.

## Goals / Non-Goals

**Goals:**
- Set due date on the selected task with a single keypress in Normal mode
- Support relative date options: today, +1 week, +1 month, +3 months, clear
- Persist immediately and show a status message confirming the date

**Non-Goals:**
- Custom/arbitrary date entry (e.g., a date picker or typed date)
- Bulk due date assignment on multiple tasks
- A separate editing mode — all keys work directly in Normal mode

## Decisions

### 1. Shift-letter keybindings in Normal mode

No new `Mode` variant needed. The Shift-letter keys are added directly to `handle_normal`:

- `T` → today (`Local::now().date_naive()`)
- `W` → next week (`today + 7 days`)
- `M` → next month (`today + 1 month` via `checked_add_months`)
- `Q` → next quarter (`today + 3 months` via `checked_add_months`)
- `X` → clear due date (`None`)

**Why Shift letters:** Lowercase `t`, `m`, `q` are taken. Shift variants are unused, mnemonic (Today, Week, Month, Quarter, X=clear), and require no mode switch.

### 2. Date arithmetic via `chrono`

The project already depends on `chrono`. Use `chrono::Local::now().date_naive()` for "today" and `NaiveDate::checked_add_months` / `checked_add_signed` for offsets. No new dependencies needed.

### 3. Status message format

After setting: `"Due: YYYY-MM-DD"`. After clearing: `"Due date cleared"`. Same `status_message` pattern used elsewhere.

### 4. Footer hints

Add `T:today W:+wk M:+mo Q:+qtr X:clr-due` to the Normal mode footer. This is compact enough to fit alongside existing hints.

## Risks / Trade-offs

- **`Q` shadows quit in muscle memory**: Users accustomed to `q` for quit might accidentally hit `Q` (Shift+Q) and set a due date. Mitigated by the fact that `Q` is clearly distinct from `q` (Shift required), and the action is easily reversible with `X` to clear.
- **Month arithmetic edge cases**: Adding 1 month to Jan 31 → Feb 28/29. `checked_add_months` clamps to the last day of the target month. Acceptable behavior.
