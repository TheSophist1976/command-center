# Claude Code Instructions

## Keeping AGENTS.md in Sync

`AGENTS.md` documents the `tasks.md` file format and editing rules for AI agents. It must stay accurate as the application evolves.

**Update `AGENTS.md` whenever any of the following change:**

- The `tasks.md` file format — header fields (`format`, `next-id`), task heading structure, metadata comment layout, or description handling (see `src/parser.rs` `serialize()` and `parse()`)
- Metadata fields — adding, removing, or renaming fields (`id`, `priority`, `tags`, `due`, `project`, `recur`, `note`, `created`, `updated`)
- Valid values for any field — priority levels, recurrence patterns, date formats, encoding rules
- Task operations exposed to users — new ways to add, edit, complete, reopen, or delete tasks
- The `format_version` written by the serializer

When making such a change, review `AGENTS.md` and update the affected section before committing.
