## Context

The task system is used by both the user (human) and AI agents (Claude Code running in specific repos). Currently nothing in the data model separates human tasks from AI tasks, and AI agents have no way to know which working directory a task belongs to without reading a description.

The solution has three parts: (1) named agent profiles in config, (2) an `agent` field on tasks, (3) a CWD-based lookup rule documented in AGENTS.md and the Cowork skill.

## Goals / Non-Goals

**Goals:**
- AI agents can identify their tasks using only their current working directory
- Humans can assign/unassign agent on any task from the TUI
- `agent:human` explicitly marks a task as human-only
- Tasks with no `agent` field are unassigned (neither human nor AI)
- Config stores agent profiles as simple `agent-<name>: <dir>` key-value pairs — no new file format

**Non-Goals:**
- Multi-agent assignment (one agent per task)
- Enforcing that AI agents only work on their assigned tasks (this is a convention, not a lock)
- Any CLI subcommand for managing agent profiles (use `task config set`)
- Any filtering in the TUI based on agent (visibility is unchanged)

## Decisions

### Decision: `agent-<name>: <dir>` in existing config.md
Store agent profiles as regular config entries with a `agent-` prefix. No separate file, no structured format.

**Rationale**: The existing `read_config_value` / `write_config_value` system doesn't support multi-value keys — we need a list of profiles. Using a prefix convention (`agent-command-center: ~/code/command-center`) lets us enumerate all agent profiles by scanning config lines for the `agent-` prefix, without changing the config format.

**Alternatives considered**: A separate `agents.md` file — rejected, more moving parts. JSON value in a single key — rejected, breaks the plain-text-editable contract.

### Decision: `agent` stored as a plain string on Task
The `agent` field stores the profile name (`command-center`), the literal `human`, or is absent (`None` = unassigned). No validation against configured profiles at parse time — the field is treated as an opaque string.

**Rationale**: Keeps the parser simple. Validation would require the config at parse time, creating a coupling we don't want.

### Decision: CWD prefix matching for agent lookup
When an AI agent wants to find its tasks, it: (1) reads all `agent-*` config entries, (2) finds the entry whose expanded directory is a prefix of the current working directory, (3) uses that profile name to filter tasks.

**Rationale**: An agent running in `~/code/command-center/src` should match a profile configured as `~/code/command-center`. Prefix matching handles all subdirectory cases naturally.

### Decision: TUI uses a picker modal for agent assignment
A new `EditingAgent` mode shows a vertical list: configured profiles + `human` + `(clear)`. Invoked with `A` in Normal mode (uppercase to avoid collision with `a` = add task).

## Risks / Trade-offs

- [Risk] Config prefix scanning (`agent-*`) could conflict with future config keys that happen to start with `agent-` → Mitigation: document the convention; in practice unlikely.
- [Risk] AI agent reads stale config — profile dir doesn't match current project → Mitigation: this is a user configuration problem, not a code problem. The lookup is explicit and visible.
- [Risk] TUI picker is empty if no agent profiles are configured → Mitigation: picker still shows `human` and `(clear)` options; a message explains how to add profiles.

## Migration Plan

No migration needed. The `agent` field is optional — existing task files parse correctly with `agent: None` for all tasks. No format version bump required.
