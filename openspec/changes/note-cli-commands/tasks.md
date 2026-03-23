## 1. CLI Structure

- [x] 1.1 Add `NoteCommand` enum to `src/cli.rs` with variants: `List`, `Add { title: String, task: Option<u32> }`, `Show { slug: String }`, `Edit { slug: String, title: Option<String>, body: Option<String> }`, `Rm { slug: String }`, `Link { slug: String, task_id: u32 }`, `Unlink { task_id: u32 }`
- [x] 1.2 Add `Note { subcommand: NoteCommand }` variant to the `Command` enum in `src/cli.rs`
- [x] 1.3 Update `use cli::{..., NoteCommand}` import in `src/main.rs`

## 2. Command Implementation

- [x] 2.1 Implement `Command::Note` match arm in `src/main.rs` — dispatch to `NoteCommand` variants
- [x] 2.2 Implement `NoteCommand::List` — call `note::discover_notes`, print `<slug>  <title>` per line
- [x] 2.3 Implement `NoteCommand::Add` — slugify title, call `note::unique_slug`, call `note::write_note` with empty body, print file path; if `--task <id>` given, also load task file, set task's `note` field to the new slug, and save
- [x] 2.4 Implement `NoteCommand::Show` — resolve path from slug, call `note::read_note`, print title then body
- [x] 2.5 Implement `NoteCommand::Edit` — read existing note, apply `--title` / `--body` overrides, write back, print file path; error if neither flag given
- [x] 2.6 Implement `NoteCommand::Rm` — call `note::delete_note`, print confirmation; propagate error on failure
- [x] 2.7 Implement `NoteCommand::Link` — load task file, find task by id, set `note` field to slug, save; error if task not found
- [x] 2.8 Implement `NoteCommand::Unlink` — load task file, find task by id, clear `note` field to `None`, save; error if task not found

## 3. Skill Update

- [x] 3.1 Update `skills/task-manager/SKILL.md` to document `task note list/add/show/edit/rm/link/unlink` commands, valid flags, and output format
- [x] 3.2 Remove the line in the skill that says the CLI only supports `tui`, `auth`, and `config`
- [x] 3.3 Run `./deploy.sh` to install the updated skill to `~/.claude/skills/task-manager/`

## 4. Verification

- [x] 4.1 Run `cargo build` and confirm no compile errors
- [x] 4.2 Smoke-test `task note list`, `task note add`, `task note show`, `task note edit`, `task note rm`, `task note link`, `task note unlink` against a test directory
- [x] 4.3 Verify `task --help` lists `note` subcommand
- [x] 4.4 Verify `task note --help` lists all sub-subcommands
