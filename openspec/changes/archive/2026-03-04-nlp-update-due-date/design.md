## Context

The NLP update action lets the model change `priority`, `status`, and `tags` on tasks matching criteria. The `SetFields` struct, system prompt, TUI confirmation preview, and TUI execution all need to learn about `due_date`.

Tasks already store `due_date` as `Option<NaiveDate>`. The NLP system prompt already includes today's date for relative time references.

## Goals / Non-Goals

**Goals:**
- Let the model set or clear due dates via the update action
- Support both absolute dates ("2026-03-10") and the model interpreting relative dates ("today", "next monday") into YYYY-MM-DD format
- Show due date changes in the confirmation preview

**Non-Goals:**
- No new action type — this extends the existing update action
- No changes to the filter action (it doesn't need due_date matching)

## Decisions

### 1. Add `due_date: Option<String>` to SetFields

Use `Option<String>` with YYYY-MM-DD format, matching how the model already sees dates in the task context. The TUI parses the string into `NaiveDate` when applying.

A special value of `""` (empty string) or explicit null means "clear the due date". The model should use `null` to clear.

### 2. System prompt update

Add `"due_date":null` to the `set` field in the update action format example. Add a note that due_date should be in YYYY-MM-DD format and that the model should resolve relative dates using today's date.

### 3. TUI preview and execution

The `format_update_preview` function adds a line like `due_date: 2026-03-01 → 2026-03-10` for each affected task. The execution handler parses the date string and sets `task.due_date`.

## Risks / Trade-offs

- [Date parsing] The model might return an invalid date string → parse with `NaiveDate::parse_from_str` and show an error if it fails.
- [Relative dates] The model must resolve "tomorrow" to "2026-03-05" itself — we don't parse relative dates in the app. The system prompt already provides today's date, so this should work.
