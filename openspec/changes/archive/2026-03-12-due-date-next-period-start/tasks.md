## 1. Update NLP System Prompt

- [x] 1.1 In `src/nlp.rs`, update the `due_date` instruction in `build_system_prompt_raw` to add an explicit rule: "next week" → Monday of the following calendar week, "next month" → 1st of the following calendar month, "next year" → January 1st of the following calendar year

## 2. Tests

- [x] 2.1 Add a unit test asserting that the system prompt string contains the new "next week/month/year" resolution rule
- [x] 2.2 Run `cargo test` to confirm all existing NLP tests pass
