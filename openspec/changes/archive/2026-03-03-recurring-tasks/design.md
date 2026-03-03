## Context

The task manager currently supports one-shot tasks with optional due dates. Users who have recurring obligations (daily standups, weekly reviews, monthly reports) must manually re-create tasks each time. The system already has an NLP chat interface that interprets natural language commands and returns structured actions (Filter, Update, Message, ShowTasks). The parser stores task metadata as key-value pairs in HTML comments (e.g., `id:1 priority:high due:2025-12-31`).

## Goals / Non-Goals

**Goals:**
- Add a recurrence field to tasks that supports simple intervals (daily, weekly, monthly, yearly) and nth-weekday patterns (e.g., 3rd Thursday of each month)
- Auto-create the next occurrence when a recurring task is completed
- Provide a quick TUI keybinding (`R`) to set recurrence via an inline NLP prompt
- Extend the NLP system to understand recurrence commands (set, change, remove)
- Display recurrence info in the task table and detail panel

**Non-Goals:**
- Complex cron-like schedules (e.g., "every other Tuesday except holidays")
- Recurrence end dates or occurrence limits
- Bulk recurrence operations
- Calendar integration or external sync

## Decisions

### 1. Recurrence data model

The `Recurrence` enum will have two variants:
- `Interval(IntervalUnit)` — for daily, weekly, monthly, yearly
- `NthWeekday { n: u8, weekday: Weekday }` — for "nth weekday of the month" (e.g., 3rd Thursday)

`IntervalUnit` is a simple enum: `Daily`, `Weekly`, `Monthly`, `Yearly`.

The `Task` struct gains `recurrence: Option<Recurrence>`.

**Rationale:** Two variants cover 95% of real-world recurrence needs without complex scheduling libraries. The nth-weekday pattern handles cases like "every third Thursday" which was explicitly requested. Keeping it as an enum (not a string) gives type safety and makes next-date calculation straightforward.

**Alternative considered:** A single `String` field parsed on demand — rejected because it pushes validation to runtime and makes next-date calculation error-prone.

### 2. Serialization format

In the metadata comment, recurrence is stored as `recur:<value>`:
- Simple intervals: `recur:daily`, `recur:weekly`, `recur:monthly`, `recur:yearly`
- Nth weekday: `recur:monthly:N:DAY` (e.g., `recur:monthly:3:thu` for 3rd Thursday)

**Rationale:** Consistent with existing metadata format (key:value pairs, no spaces). The `monthly:N:DAY` format is unambiguous and parseable without complex logic. The three-letter lowercase weekday abbreviation (mon, tue, wed, thu, fri, sat, sun) matches chrono's conventions.

### 3. Next-occurrence calculation

When a recurring task is marked done:
1. The original task is marked done (as usual)
2. A new task is created with the same title, tags, project, priority, and recurrence
3. The new task's due date is calculated from the **original task's due date** (not today):
   - Interval: add 1 day/week/month/year to the original due date
   - NthWeekday: find the next month's nth weekday after the original due date
4. If the original task had no due date, the new task's due date is calculated from today
5. The new task gets a new ID from `next_id` and `created` set to now

**Rationale:** Calculating from the original due date preserves the intended schedule rhythm. If a weekly task due Monday is completed on Wednesday, the next occurrence is still the following Monday, not the following Wednesday. Falling back to "from today" when there's no due date is a pragmatic default.

**Alternative considered:** Always calculate from today — rejected because it causes schedule drift for tasks completed late.

### 4. NLP recurrence action

A new `SetRecurrence` variant is added to `NlpAction`:
```
SetRecurrence { task_id: u32, recurrence: Option<String>, description: String }
```

The `recurrence` field is `None` to remove recurrence, or a string like `"weekly"` or `"monthly:3:thu"` that maps to the `Recurrence` enum. The NLP system prompt is extended with a new action format and the recurrence field is included in task context.

**Rationale:** Keeping the NLP response as a string that maps to the existing serialization format avoids a separate parsing path. The TUI handles the conversion from string to `Recurrence` enum using the same parser logic.

### 5. Quick recurrence keybinding (`R`)

Pressing `R` in Normal mode enters a new `EditingRecurrence` mode. The footer shows an inline text input prompt. The user types a natural language recurrence pattern (e.g., "every third thursday", "weekly", "none") and presses Enter. The input is sent to the NLP with a focused system prompt that only parses recurrence patterns (not general task commands). The result sets or clears the recurrence on the selected task.

**Rationale:** `r` is already used for description editing. `R` (shift-R) is a natural mnemonic for "Recurrence". Using NLP for the inline prompt means the user can type natural language like "every third thursday" without learning the internal format. A focused prompt (not the full NLP chat) keeps the response fast and predictable.

**Alternative considered:** A picker UI like priority (`c/h/m/l`) — rejected because the recurrence space is too large for a fixed picker, and the user explicitly asked for natural language input like "every third thursday".

### 6. Recurrence display

- **Table:** A `↻` indicator column appears when any visible task has recurrence set. The column shows `↻` for recurring tasks and is blank for non-recurring tasks.
- **Detail panel:** Shows `Recurrence: weekly` or `Recurrence: monthly (3rd Thu)` or `Recurrence: -` (if none).

## Risks / Trade-offs

- **NLP accuracy for recurrence parsing:** The NLP model might misinterpret ambiguous recurrence patterns. → Mitigation: The focused recurrence prompt is narrow in scope, reducing ambiguity. The confirmation in the status message shows what was set, so the user can verify.
- **Schedule drift for nth-weekday:** Finding "the 5th Friday" in a month that only has 4 Fridays. → Mitigation: Skip to the next month that has the nth weekday occurrence.
- **No recurrence end date:** Tasks repeat forever. → Mitigation: Users can manually remove recurrence. This is a non-goal for v1; can be added later if needed.
