## Context

The NLP system prompt instructs the Claude model to resolve relative date expressions into `YYYY-MM-DD` strings for `due_date`. The prompt currently says: "resolve relative dates like 'today', 'tomorrow', 'next monday' to absolute dates". It does not define what "next week", "next month", or "next year" should resolve to. Different Claude responses produce different interpretations (e.g., "next week" might land on Wednesday of next week rather than Monday).

## Goals / Non-Goals

**Goals:**
- "next week" always resolves to the Monday of the following calendar week
- "next month" always resolves to the 1st of the following calendar month
- "next year" always resolves to January 1st of the following calendar year

**Non-Goals:**
- Changing how other relative dates ("tomorrow", "next friday", "in 2 weeks") are resolved
- Adding client-side date math (all resolution stays in the LLM prompt)
- Changing the task data model or storage format

## Decisions

**Update the NLP system prompt string only.** The fix is a single-line addition to the `due_date` instruction in `build_system_prompt_raw` in `src/nlp.rs`. No new functions, no data model changes.

Alternatives considered:
- *Client-side post-processing*: Parse the model output and re-map ambiguous dates → rejected because it requires pattern matching on natural language and duplicates logic the model already handles well with the right instructions.
- *Structured date enum*: Add a first-class "period start" field → over-engineered for a prompt-wording fix.

## Risks / Trade-offs

- [Risk] The model ignores the updated instruction in rare cases → Mitigation: the instruction is explicit and deterministic; the model has no reason to deviate. If needed, a test can assert the resolved value for a known date.
- [Trade-off] Week start = Monday (ISO week): some locales start on Sunday. Accepted; ISO Monday is the dominant convention for task-management tools and matches user intent in the bug report.

## Migration Plan

1. Update the prompt string in `build_system_prompt_raw`
2. Run existing NLP tests (`cargo test nlp`)
3. No data migration needed — only future due-date assignments are affected
