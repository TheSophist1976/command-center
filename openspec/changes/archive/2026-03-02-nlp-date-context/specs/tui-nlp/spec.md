## MODIFIED Requirements

### Requirement: NLP intent interpretation
The system SHALL provide an `nlp::interpret` function that accepts the current task list, a natural language input string, and a Claude API key, and returns a structured `NlpAction` result. The function SHALL call the Claude API (`claude-haiku-4-5-20251001`) with a system prompt that includes a JSON summary of the current tasks (capped at 200 tasks), the current date formatted as `YYYY-MM-DD (DayOfWeek)`, and instructions to return a JSON object describing the intended action. The system prompt SHALL instruct the model to use the provided date for interpreting relative time references such as "today", "this week", "overdue", "tomorrow", etc.

#### Scenario: System prompt includes current date
- **WHEN** the NLP system prompt is constructed
- **THEN** the prompt SHALL include a line stating today's date in `YYYY-MM-DD (DayOfWeek)` format (e.g., "Today's date is 2026-03-02 (Monday).")

#### Scenario: Relative date query interpreted correctly
- **WHEN** the user inputs "show overdue tasks" and today is 2026-03-02
- **THEN** the model SHALL have access to today's date in the prompt to determine which tasks have due dates before 2026-03-02
