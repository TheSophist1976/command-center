## 1. Skill File

- [x] 1.1 Create `skills/task-manager/` directory in the repo root
- [x] 1.2 Write `skills/task-manager/SKILL.md` with YAML frontmatter (`name`, `description`) and instructions covering all task commands
- [x] 1.3 Verify skill covers `task list` with `--status`, `--priority`, `--tag` filters
- [x] 1.4 Verify skill covers `task show <id>`
- [x] 1.5 Verify skill covers `task add` with `--priority`, `--tags`, `--due`, `--project` flags
- [x] 1.6 Verify skill covers `task edit <id>` with `--title`, `--priority`, `--tags`, `--due`, `--project` flags
- [x] 1.7 Verify skill covers `task done <id>` and `task undo <id>`
- [x] 1.8 Verify skill covers `task rm <id>`
- [x] 1.9 Verify skill documents valid values: priorities, tag format, due date format (YYYY-MM-DD)
- [x] 1.10 Verify skill explicitly instructs Claude not to run `task tui` or bare `task`

## 2. Deploy Script Integration

- [x] 2.1 Add a skill installation step to `deploy.sh` that copies `skills/task-manager/` to `~/.claude/skills/task-manager/` using `cp -r`
- [x] 2.2 Add a `STATUS_SKILL` variable and update the deploy summary table to show a "Skill:" row with the installed path
- [x] 2.3 Verify the deploy step is idempotent (running `./deploy.sh` twice does not error)

## 3. Verification

- [x] 3.1 Run `./deploy.sh` and confirm `~/.claude/skills/task-manager/SKILL.md` exists after completion
- [x] 3.2 Confirm the summary table shows the skill install path
- [x] 3.3 Run `./deploy.sh` a second time and confirm no errors
