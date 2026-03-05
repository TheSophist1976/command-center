## Why

The task manager stores tasks but has no way to capture longer-form information — meeting notes, project plans, reference material, or detailed context. Users need a notes system that lives alongside tasks and can optionally be linked to specific tasks for deeper context.

## What Changes

- Introduce markdown note files (`.md`) stored in the same directory as the task file
- Notes have a title (used as filename) and markdown body content
- Notes can optionally be attached to a task via a `note` metadata field on the task
- TUI gains a Notes view to list, create, and edit notes with an inline multi-line editor
- AI can create and edit notes through new NLP actions (`create_note`, `edit_note`)
- Notes are standalone files — deleting a task does not delete its linked note, and notes can exist without any task attachment

## Capabilities

### New Capabilities

- `note-storage`: Defines how notes are stored as markdown files, naming conventions, and the notes index/discovery mechanism
- `note-tui`: TUI view for listing notes, creating new notes, editing note content with an inline multi-line markdown editor, and navigating between notes and linked tasks
- `note-task-link`: Optional linking between notes and tasks via a `note` metadata field on the task, with bidirectional navigation in the TUI
- `note-nlp`: NLP actions for AI-driven note creation and editing through the chat interface

### Modified Capabilities

_None_ — this is an additive feature with no changes to existing spec behavior.

## Impact

- `src/tui.rs`: New Notes view, note editor mode, note list rendering, keybindings
- `src/task.rs`: New `note` field on Task struct for optional note attachment
- `src/parser.rs`: Parse/serialize `note` metadata key in task comments
- `src/nlp.rs`: New `CreateNote` and `EditNote` NLP action variants, updated system prompt
- New note file I/O (read/write `.md` files in the task directory)
