## ADDED Requirements

### Requirement: CreateNote NLP action
The NLP system SHALL support a `create_note` action that creates a new markdown note file. The action SHALL accept a `title` (required), `content` (required), and optional `task_id` to auto-link the note to a task. On success, the system SHALL display a confirmation message with the note title.

#### Scenario: AI creates a note
- **WHEN** the AI responds with `{"action":"create_note","title":"Sprint Plan","content":"## Goals\n- Ship feature X"}`
- **THEN** the system SHALL create `sprint-plan.md` with the specified content and display a confirmation

#### Scenario: AI creates a note linked to a task
- **WHEN** the AI responds with `{"action":"create_note","title":"Research Notes","content":"Findings...","task_id":5}`
- **THEN** the system SHALL create the note file AND set task 5's `note` field to the note's slug

### Requirement: EditNote NLP action
The NLP system SHALL support an `edit_note` action that replaces the body content of an existing note. The action SHALL accept a `slug` (required) and `content` (required). If the slug does not match an existing note file, the system SHALL display an error message.

#### Scenario: AI edits an existing note
- **WHEN** the AI responds with `{"action":"edit_note","slug":"sprint-plan","content":"## Updated Goals\n- Ship feature Y"}`
- **THEN** the system SHALL update `sprint-plan.md` with the new content, preserving the title heading

#### Scenario: AI edits non-existent note
- **WHEN** the AI responds with `{"action":"edit_note","slug":"nonexistent","content":"..."}`
- **THEN** the system SHALL display an error message indicating the note was not found

### Requirement: NLP system prompt includes note instructions
The NLP system prompt SHALL include instructions for creating and editing notes, with format examples and valid action names. The prompt SHALL explain that notes are markdown files and the AI can use markdown formatting in note content.

#### Scenario: System prompt describes note actions
- **WHEN** the NLP system prompt is generated
- **THEN** it SHALL include documentation for `create_note` and `edit_note` actions with JSON format examples

### Requirement: NLP note context
The NLP system SHALL include a list of existing note slugs in the task context provided to the AI, so the AI can reference and edit existing notes by slug.

#### Scenario: Note slugs in context
- **WHEN** the task directory contains notes `sprint-plan.md` and `meeting-notes.md`
- **THEN** the NLP context SHALL include these note slugs so the AI can reference them
