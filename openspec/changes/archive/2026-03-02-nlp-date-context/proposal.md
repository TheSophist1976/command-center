## Why

The NLP system prompt includes the user's tasks (with their due dates) but never tells the model what today's date is. When a user says "show overdue tasks", "tasks due this week", or "mark tasks due yesterday as done", the model has to guess — and gets it wrong. Adding the current date to the system prompt is a one-line fix that makes all date-relative queries work correctly.

## What Changes

- Include today's date (formatted as `YYYY-MM-DD` with day-of-week) in the NLP system prompt so the model can reason about relative dates
- Add a rule to the system prompt instructing the model to use the provided date for interpreting relative time references (today, this week, overdue, etc.)

## Capabilities

### New Capabilities

_(none)_

### Modified Capabilities

- `tui-nlp`: The NLP intent interpretation requirement changes — the system prompt SHALL include the current date

## Impact

- `src/nlp.rs`: `build_system_prompt` gains a `today` parameter
- `src/nlp.rs`: `interpret` passes today's date into the prompt builder
- No API changes, no new dependencies (chrono is already used)
