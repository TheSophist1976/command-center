## 1. System prompt changes

- [x] 1.1 Change `build_system_prompt` signature to accept a `today: &str` parameter and insert `Today's date is {today}.` into the prompt before the task data.
- [x] 1.2 Add a rule to the system prompt: "Use the provided current date to interpret relative time references such as 'today', 'this week', 'overdue', 'tomorrow', etc."
- [x] 1.3 Update `interpret` to format `Local::now().date_naive()` as `"%Y-%m-%d (%A)"` and pass it to `build_system_prompt`.

## 2. Tests

- [x] 2.1 Add test `system_prompt_includes_today_date`: call `build_system_prompt` with a known date string and assert the prompt contains that date.
- [x] 2.2 Update existing test `build_task_context_includes_tasks` if it calls `build_system_prompt` (verify no breakage from the new parameter).
