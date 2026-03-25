## ADDED Requirements

### Requirement: Agent profile config entries
The system SHALL support named agent profiles stored in `config.md` as `agent-<name>: <dir>` entries, where `<name>` is a kebab-case profile name and `<dir>` is the absolute or tilde-prefixed working directory path for that agent.

#### Scenario: Profile stored and retrieved
- **WHEN** the user runs `task config set agent-command-center ~/code/command-center`
- **THEN** the config file SHALL contain `agent-command-center: ~/code/command-center` and the value SHALL be readable via `task config get agent-command-center`

#### Scenario: Multiple profiles coexist
- **WHEN** multiple `agent-*` entries exist in config
- **THEN** each SHALL be independently readable and writable without affecting the others

### Requirement: List all agent profiles
The system SHALL be able to enumerate all `agent-*` entries from config as a list of `(name, dir)` pairs, used by the TUI agent picker and the CWD-based lookup.

#### Scenario: No profiles configured
- **WHEN** no `agent-*` keys exist in config
- **THEN** the profile list SHALL be empty

#### Scenario: Two profiles configured
- **WHEN** config contains `agent-alpha: ~/code/alpha` and `agent-beta: ~/code/beta`
- **THEN** the profile list SHALL contain two entries: `("alpha", "~/code/alpha")` and `("beta", "~/code/beta")`

### Requirement: CWD-based agent lookup
Given a current working directory, the system SHALL find the agent profile whose expanded directory path is a prefix of the CWD (or equal to it). If multiple profiles match, the longest (most specific) match SHALL be used. If no profile matches, the lookup returns no result.

#### Scenario: Exact directory match
- **WHEN** CWD is `/Users/mark/code/command-center` and a profile has dir `/Users/mark/code/command-center`
- **THEN** that profile SHALL be returned

#### Scenario: Subdirectory match
- **WHEN** CWD is `/Users/mark/code/command-center/src/bin` and a profile has dir `/Users/mark/code/command-center`
- **THEN** that profile SHALL be returned (prefix match)

#### Scenario: Tilde expansion applied before comparison
- **WHEN** a profile dir is stored as `~/code/command-center` and CWD is `/Users/mark/code/command-center`
- **THEN** the tilde SHALL be expanded to the home directory before comparison, and the profile SHALL match

#### Scenario: No matching profile
- **WHEN** CWD has no configured profile whose dir is a prefix
- **THEN** the lookup SHALL return no profile

#### Scenario: Most specific match wins
- **WHEN** profiles exist for `~/code` and `~/code/command-center` and CWD is `~/code/command-center/src`
- **THEN** the `~/code/command-center` profile SHALL be returned

### Requirement: TUI agent picker
The TUI SHALL provide an agent assignment mode (`EditingAgent`) invoked with `A` in Normal mode. The picker SHALL list all configured agent profiles by name, plus `human` and `(clear)`. Selecting a profile sets `agent:<name>` on the task. Selecting `human` sets `agent:human`. Selecting `(clear)` removes the `agent` field.

#### Scenario: Picker shows configured profiles
- **WHEN** the user presses `A` with profiles `command-center` and `itential` configured
- **THEN** the picker SHALL list `command-center`, `itential`, `human`, and `(clear)` as options

#### Scenario: No profiles — picker still opens
- **WHEN** no `agent-*` profiles are configured and the user presses `A`
- **THEN** the picker SHALL open showing only `human` and `(clear)`, with a note indicating how to add profiles

#### Scenario: Assign profile to task
- **WHEN** the user selects `command-center` from the picker
- **THEN** the task's `agent` field SHALL be set to `command-center` and the file SHALL be saved

#### Scenario: Mark task as human
- **WHEN** the user selects `human` from the picker
- **THEN** the task's `agent` field SHALL be set to `human` and the file SHALL be saved

#### Scenario: Clear agent from task
- **WHEN** the user selects `(clear)` from the picker
- **THEN** the task's `agent` field SHALL be set to `None` and the `agent` key SHALL be omitted from the serialized metadata

#### Scenario: Esc cancels picker
- **WHEN** the user presses `Esc` in the picker
- **THEN** the task's `agent` field SHALL be unchanged and the mode SHALL return to Normal

### Requirement: AGENTS.md routing rule
The `AGENTS.md` file SHALL document the CWD-based task lookup rule so AI agents know how to find their assigned tasks.

#### Scenario: AGENTS.md contains routing instructions
- **WHEN** an AI agent reads `AGENTS.md`
- **THEN** it SHALL find instructions explaining: (1) read `agent-*` config entries, (2) find the entry whose dir is a prefix of CWD, (3) filter tasks.md to that agent name

### Requirement: Cowork skill routing instructions
The Cowork skill (`skills/task-manager/SKILL.md`) SHALL include agent routing instructions equivalent to those in `AGENTS.md`.

#### Scenario: Skill documents agent filtering
- **WHEN** Claude reads the Cowork skill
- **THEN** it SHALL find instructions to use CWD to identify its agent profile and filter tasks accordingly
