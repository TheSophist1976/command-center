## Context

The `build_system_prompt` function in `src/nlp.rs` constructs the system prompt sent to the Claude API. It includes a JSON dump of the user's tasks (with due dates) but does not include today's date. The model cannot reason about relative time ("overdue", "due this week", "due tomorrow") without knowing the current date.

## Goals / Non-Goals

**Goals:**
- Include today's date in the NLP system prompt
- Enable the model to correctly interpret relative date references

**Non-Goals:**
- Timezone handling — use local date (chrono `Local::now()`)
- Changing the NLP action types or response format

## Decisions

### 1. Add today's date to `build_system_prompt`

Change `build_system_prompt(task_context: &str)` to `build_system_prompt(task_context: &str, today: &str)`. Insert a line like `Today's date is YYYY-MM-DD (DayOfWeek).` near the top of the prompt, before the task data. Include the day-of-week name so the model can reason about "this week" more naturally.

The caller (`interpret`) will format `Local::now()` as `"%Y-%m-%d (%A)"` and pass it in.

### 2. Add a date-awareness rule to the system prompt

Add a rule in the existing Rules section: "Use the provided current date to interpret relative time references such as 'today', 'this week', 'overdue', 'tomorrow', etc."

## Risks / Trade-offs

- **Minimal risk**: This is a string interpolation change in the system prompt. No structural changes to the code.
- **Date accuracy**: Uses the machine's local date. If the user's system clock is wrong, the model will get the wrong date. This is acceptable — same assumption the rest of the app makes.
