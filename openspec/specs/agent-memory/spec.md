## ADDED Requirements

### Requirement: Agent memory file storage
Each agent SHALL have an optional memory file at `<task-dir>/Notes/Agents/<agent-name>/memory.md`. The file is created on first write. If absent, the agent has no accumulated memory — this is not an error.

#### Scenario: Memory file path is deterministic
- **WHEN** memory is written for agent `research`
- **THEN** the file SHALL be created at `<task-dir>/Notes/Agents/research/memory.md`

#### Scenario: Missing memory file is not an error
- **WHEN** an agent reads its memory and no `memory.md` exists
- **THEN** the system SHALL return empty content without error

### Requirement: Agent memory CLI subcommand
The CLI SHALL provide `task agent memory <name> show` to print current memory and `task agent memory <name> edit --body "<content>"` to replace the memory body. The `--title` flag is optional; if omitted, the title defaults to `"<agent-name> Memory"`.

#### Scenario: Show memory for an agent
- **WHEN** the user runs `task agent memory research show`
- **THEN** the system SHALL print the title and body of `Notes/Agents/research/memory.md`, or print "No memory found for agent 'research'." if the file does not exist

#### Scenario: Write memory for an agent
- **WHEN** the user runs `task agent memory research edit --body "Mark prefers bullet points over prose."`
- **THEN** the system SHALL write `Notes/Agents/research/memory.md` with that body and exit with code 0

#### Scenario: Agent directory auto-created
- **WHEN** no `Notes/Agents/<name>/` directory exists and memory is written
- **THEN** the directory SHALL be created before the file is written

### Requirement: Memory content guidelines (enforced by skill, not code)
The `work-agent-tasks` skill SHALL instruct agents to:
- Read their memory file at the start of every session before working any task
- Update memory when a pattern has been observed at least twice, a standing fact is established (recurring contact, project preference, known constraint), or a past mistake is being corrected
- NOT update memory for: one-off task details, information that belongs in the task note, or anything likely to change within the current sprint
- Write memory in free-form markdown; suggested sections are: `## Preferences`, `## Patterns`, `## Standing Context`
- Periodically review and prune outdated or contradictory entries

#### Scenario: Agent reads memory before working
- **WHEN** an agent session starts and a memory file exists
- **THEN** the agent SHALL read the memory file before processing any task

#### Scenario: Agent updates memory after observing a pattern
- **WHEN** an agent completes a task and observes a preference or pattern worth recording
- **THEN** the agent SHALL update `Notes/Agents/<name>/memory.md` with the new entry

#### Scenario: Memory does not replace task notes
- **WHEN** an agent completes a task
- **THEN** task-specific output SHALL go in a task note (`task note add`), not in memory
