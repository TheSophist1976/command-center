## 1. Migrate Existing Instruction Files

- [x] 1.1 Create `Notes/Agents/` subdirectories and move each existing instruction file: `Notes/Instructions/<name>.md` → `Notes/Agents/<name>/instructions.md` for all five agents (research, follow-up, writer, reviewer, automator)
- [x] 1.2 Verify original `Notes/Instructions/` files are removed after migration

## 2. CLI — Update Instruction Path

- [x] 2.1 Update `src/bin/task.rs`: change `instructions_dir` from `task_dir/Notes/Instructions/` to `task_dir/Notes/Agents/<name>/`; add legacy fallback read from old path when new path doesn't exist
- [x] 2.2 Ensure writes always go to the new `Notes/Agents/<name>/instructions.md` path

## 3. CLI — Add Memory Subcommand

- [x] 3.1 Add `AgentMemoryCommand` enum (show / edit) to `src/cli.rs` alongside existing `AgentInstructionsCommand`
- [x] 3.2 Add `Memory` variant to `AgentCommand` in `src/cli.rs`
- [x] 3.3 Implement `task agent memory <name> show` in `src/bin/task.rs`: read `Notes/Agents/<name>/memory.md`, print title + body, or "No memory found for agent '<name>'." if absent
- [x] 3.4 Implement `task agent memory <name> edit --body/--title` in `src/bin/task.rs`: write to `Notes/Agents/<name>/memory.md`, auto-creating directory; default title is `"<name> Memory"` if not provided

## 4. Update `work-agent-tasks` Skill

- [x] 4.1 Update `~/.claude/skills/work-agent-tasks/SKILL.md`: add Step 2.5 between "Read Instructions" and "Find Assigned Tasks" — read `Notes/Agents/<name>/memory.md` and load it alongside instructions
- [x] 4.2 Add a "Updating Memory" section to the skill explaining: when to update (pattern seen 2+ times, standing fact, correcting a mistake), what NOT to store (one-off task details, task note content), format (free-form markdown, suggested sections: Preferences / Patterns / Standing Context), and how to write (use `task agent memory <name> edit` or write file directly)

## 5. Update Agent Instruction Files

- [x] 5.1 Add a `## Memory` section to each agent's `instructions.md` (research, follow-up, writer, reviewer, automator) explaining: read memory at session start before working tasks; update memory when you observe a pattern or standing preference worth remembering; use `task agent memory <name> show/edit`
- [x] 5.2 Customize the Memory guidance per agent type (e.g. Reviewer memory focuses on feedback style preferences; Writer on document tone/format preferences; Research on known sources and contacts)

## 6. Update AGENTS.md

- [x] 6.1 Update the "Reading Your Instructions" section to reflect new path `Notes/Agents/<name>/instructions.md`
- [x] 6.2 Add a "Reading and Updating Memory" section documenting the memory file location, when to update, and the `task agent memory` CLI commands
