## REMOVED Requirements

### Requirement: By Agent view rendering
**Reason**: Grouping is now a cross-cutting feature available on any view via `:group agent`. A dedicated `View::ByAgent` is no longer needed.
**Migration**: Use `:group agent` to group any view by agent. The `View::ByAgent` variant is removed from the view cycle.

### Requirement: By Agent view section header styling
**Reason**: Section header styling is now part of the generic grouping renderer used by `:group`.
**Migration**: No change — header styling is preserved in the generic grouping implementation.
