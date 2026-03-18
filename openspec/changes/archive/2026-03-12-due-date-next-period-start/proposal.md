## Why

When users say "next week", "next month", or "next year" in natural language commands, the due date is set inconsistently — sometimes to the middle of the period, sometimes to a day that depends on when the command is run. Users intuitively expect "next week" to mean the start of next week (Monday), "next month" to mean the 1st, and "next year" to mean January 1st.

## What Changes

- The NLP system prompt is updated to explicitly define how "next week / next month / next year" relative date expressions are resolved:
  - `next week` → Monday of the following calendar week
  - `next month` → 1st day of the following calendar month
  - `next year` → January 1st of the following calendar year

## Capabilities

### New Capabilities

*(none)*

### Modified Capabilities

- `tui-nlp`: The requirement for resolving relative date expressions in `due_date` is being tightened — specifically for "next week/month/year" phrases, which must now resolve to the **first day** of the respective period.

## Impact

- `src/nlp.rs`: `build_system_prompt_raw` — the instruction for resolving relative dates in `due_date` must be updated
- No data model changes; no storage changes
- No breaking changes to existing task data
