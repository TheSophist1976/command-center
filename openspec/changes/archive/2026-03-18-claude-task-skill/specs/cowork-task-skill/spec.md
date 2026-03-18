## ADDED Requirements

### Requirement: Skill file exists in repository
The repository SHALL contain a Claude Cowork skill file at `skills/task-manager/SKILL.md`. The file SHALL have valid YAML frontmatter with `name` and `description` fields, followed by plain-language instructions for Claude.

#### Scenario: Skill file has required frontmatter
- **WHEN** the file `skills/task-manager/SKILL.md` is read
- **THEN** it SHALL begin with a YAML frontmatter block containing `name:` and `description:` fields

#### Scenario: Skill file has instructions body
- **WHEN** the frontmatter block ends
- **THEN** the remainder of the file SHALL contain instructions describing how to use the `task` CLI to read and edit tasks

### Requirement: Skill covers task listing
The skill instructions SHALL document the `task list` command and its filter flags so Claude can retrieve tasks.

#### Scenario: List all tasks
- **WHEN** Claude needs to read all tasks
- **THEN** the skill SHALL instruct it to run `task list`

#### Scenario: List with status filter
- **WHEN** Claude needs only open or done tasks
- **THEN** the skill SHALL instruct it to use `task list --status open` or `task list --status done`

#### Scenario: List with priority filter
- **WHEN** Claude needs tasks by priority
- **THEN** the skill SHALL instruct it to use `task list --priority <critical|high|medium|low>`

#### Scenario: List with tag filter
- **WHEN** Claude needs tasks matching a tag
- **THEN** the skill SHALL instruct it to use `task list --tag <tag>`

### Requirement: Skill covers task detail
The skill instructions SHALL document the `task show <id>` command so Claude can retrieve full details of a single task.

#### Scenario: Show task by ID
- **WHEN** Claude needs to read full details of a specific task
- **THEN** the skill SHALL instruct it to run `task show <id>`

### Requirement: Skill covers task creation
The skill instructions SHALL document the `task add` command and its flags so Claude can create new tasks.

#### Scenario: Add task with title only
- **WHEN** Claude creates a task with no extra fields
- **THEN** the skill SHALL instruct it to run `task add "<title>"`

#### Scenario: Add task with all fields
- **WHEN** Claude creates a task with priority, tags, due date, or project
- **THEN** the skill SHALL instruct it to use `--priority`, `--tags`, `--due`, and `--project` flags respectively

### Requirement: Skill covers task editing
The skill instructions SHALL document the `task edit <id>` command and its flags so Claude can update existing tasks.

#### Scenario: Edit task fields
- **WHEN** Claude needs to update a task's title, priority, tags, due date, or project
- **THEN** the skill SHALL instruct it to run `task edit <id>` with the appropriate flags (`--title`, `--priority`, `--tags`, `--due`, `--project`)

### Requirement: Skill covers task status changes
The skill instructions SHALL document `task done <id>` and `task undo <id>` so Claude can mark tasks complete or reopen them.

#### Scenario: Complete a task
- **WHEN** Claude needs to mark a task done
- **THEN** the skill SHALL instruct it to run `task done <id>`

#### Scenario: Reopen a task
- **WHEN** Claude needs to reopen a completed task
- **THEN** the skill SHALL instruct it to run `task undo <id>`

### Requirement: Skill covers task deletion
The skill instructions SHALL document `task rm <id>` so Claude can remove tasks when instructed.

#### Scenario: Delete a task
- **WHEN** Claude needs to delete a task
- **THEN** the skill SHALL instruct it to run `task rm <id>`

### Requirement: Skill documents valid field values
The skill instructions SHALL specify the valid values for flags that have constrained inputs.

#### Scenario: Priority values documented
- **WHEN** Claude is composing a command with `--priority`
- **THEN** the skill SHALL indicate valid values are `critical`, `high`, `medium`, `low` with default `medium`

#### Scenario: Tag format documented
- **WHEN** Claude is composing a command with `--tags`
- **THEN** the skill SHALL indicate tags are comma-separated, lowercase, alphanumeric and hyphens only (e.g., `frontend,api-v2`)

#### Scenario: Due date format documented
- **WHEN** Claude is composing a command with `--due`
- **THEN** the skill SHALL indicate the required format is `YYYY-MM-DD`

### Requirement: Skill instructs Claude not to launch the TUI
The skill instructions SHALL explicitly tell Claude not to run `task tui` or `task` with no arguments, as these launch an interactive interface incompatible with Cowork.

#### Scenario: TUI launch is prohibited
- **WHEN** Claude reads the skill
- **THEN** the skill SHALL state that `task tui` and bare `task` SHALL NOT be run

### Requirement: Deploy script installs skill
The `deploy.sh` script SHALL copy `skills/task-manager/` to `~/.claude/skills/task-manager/` as part of local deployment, so the installed skill always matches the deployed binary.

#### Scenario: Skill installed on deploy
- **WHEN** the user runs `./deploy.sh` successfully
- **THEN** `~/.claude/skills/task-manager/SKILL.md` SHALL exist and match the content of `skills/task-manager/SKILL.md` in the repo

#### Scenario: Deploy is idempotent
- **WHEN** the user runs `./deploy.sh` more than once
- **THEN** the skill SHALL be overwritten with the current version without error

#### Scenario: Deploy summary includes skill status
- **WHEN** `deploy.sh` completes
- **THEN** the summary table SHALL include a "Skill:" row indicating the install path
