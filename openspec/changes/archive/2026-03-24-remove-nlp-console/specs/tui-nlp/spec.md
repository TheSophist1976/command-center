## REMOVED Requirements

### Requirement: NLP intent interpretation
**Reason**: The NLP console is removed entirely. The Claude API dependency is eliminated.
**Migration**: No replacement. Use Claude sessions (`C` key) for AI-assisted task work.

### Requirement: NLP action types
**Reason**: `NlpAction` enum and all variants are deleted with `src/nlp.rs`.
**Migration**: No replacement.

### Requirement: NLP input mode in TUI
**Reason**: The `:` keybinding no longer enters NlpChat mode. The mode is removed.
**Migration**: No replacement.

### Requirement: NLP filter execution
**Reason**: `NlpAction::Filter` is removed with the NLP module.
**Migration**: Use the manual filter (`f` key) for task filtering.

### Requirement: Action summary in chat
**Reason**: The chat panel and NlpAction types are removed.
**Migration**: No replacement.

### Requirement: NLP bulk update execution with confirmation
**Reason**: `NlpAction::Update` and `Mode::ConfirmingNlp` are removed.
**Migration**: Use the `:` command interface for bulk updates if available, or edit tasks individually.

### Requirement: NLP error display
**Reason**: The NLP module and its error paths are removed.
**Migration**: No replacement.

### Requirement: NLP recurrence execution in TUI
**Reason**: `NlpAction::SetRecurrence` and the NLP-driven recurrence path are removed.
**Migration**: Use the `R` keybinding for manual recurrence editing.
