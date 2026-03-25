## REMOVED Requirements

### Requirement: Conversation history management
**Reason**: The NLP console is removed. Multi-turn conversation state (`nlp_messages`) is deleted from `App`.
**Migration**: No replacement.

### Requirement: ShowTasks action
**Reason**: `NlpAction::ShowTasks` is removed with `src/nlp.rs`.
**Migration**: No replacement.

### Requirement: Multi-turn system prompt
**Reason**: The NLP module and its system prompt construction are deleted.
**Migration**: No replacement.
