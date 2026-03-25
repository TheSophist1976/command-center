## Why

Notes are stored and managed through the TUI, but there are no CLI subcommands to create, list, view, or delete notes — making notes inaccessible to Cowork (Claude) and any non-TUI workflow. Exposing note operations as CLI subcommands unlocks notes for automation and non-interactive use.

## What Changes

- Add a `note` subcommand group to the CLI with sub-subcommands: `list`, `add`, `show`, `edit`, `rm`
- Wire these subcommands into `main.rs` using the existing `note` module functions
- Add `note` to the `Command` enum in `cli.rs`
- Add a `NoteCommand` enum for the sub-subcommands

## Capabilities

### New Capabilities
- `note-cli`: CLI subcommand interface for creating, listing, viewing, editing, and deleting notes

### Modified Capabilities
- `cli-interface`: The `note` subcommand group is added to the existing CLI structure

## Impact

- `src/cli.rs`: New `NoteCommand` enum and `Note` variant in `Command`
- `src/main.rs`: New `Command::Note` match arm calling `note` module functions
- No changes to `note.rs` — existing functions (`discover_notes`, `read_note`, `write_note`, `delete_note`) are sufficient
- No breaking changes to existing commands
