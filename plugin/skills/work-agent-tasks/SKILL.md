---
name: work-agent-tasks
description: Use when asked to "work my tasks", "work tasks", "execute my tasks", "run my tasks", or any similar instruction to dispatch agents for each agent type and work all assigned tasks from the task-manager system.
---

# Work Agent Tasks

Orchestrate task execution by spawning a dedicated subagent for each agent type that has open tasks. Each subagent receives its specific instructions, memory, and tasks.

## Step 1: Discover Agent Types and Their Tasks

Read `~/Library/Application Support/task-manager/config.md` (macOS) or `~/.config/task-manager/config.md` (Linux).

Find all `agent-<name>: <dir>` entries — these are the registered agent types (e.g. `Research`, `Follow-up`, `Writer`, `Reviewer`, `Automator`).

For each agent name, run `task list --agent <name> --status open` **using the exact casing from config.md** (e.g. `Follow-up`, `Automator` — NOT lowercased) against `~/Documents/Mark-main/Tasks/tasks.db` to find its eligible open tasks. The `--agent` filter is a case-sensitive exact match, and tasks are stored with the same casing the `agent-<Name>:` config line uses — `--agent follow-up` silently matches nothing even though `--agent Follow-up` finds real tasks. (This is different from the lowercased/hyphenated convention used for `Notes/Agents/<name>/` file paths in Step 2/3 below — that directory naming is a separate, genuinely-lowercased convention on disk; don't apply it here.) Use `task show <id>` on each to get full detail (description, notes, recurrence, work_status) when building the subagent's task list.

**Skip the following tasks:**
- Recurring tasks (a `recur` value shown in `task show <id>`) whose due date is in the future — these are scheduled for a later cycle and should not be worked early
- Tasks whose `work_status` is already `waiting-for-review` or `complete` — these were already handed back by a prior run and are awaiting the human's review or `task done`. Handback no longer reassigns `agent` (see below), so these tasks stay in the `--agent <name>` filtered list until the human acts on them; re-picking them up would clobber that handback.

**Do NOT skip tasks whose `work_status` is `changes-requested`** — this is exactly what this step should pick up. It means a human left feedback on a `task-<id>-review-thread` note and the task needs another round. See the standing instructions below for how to handle these.

(Tasks with `agent:human` or no `agent` field never show up in an `--agent <name>` filtered list, so no extra filtering is needed for those.)

Build a map of: `agent-name → [list of eligible open tasks]`. Only agents with at least one eligible task need a subagent spawned.

## Step 2: Read Instructions for Each Active Agent

For each agent that has open tasks, read its instructions from:
```
~/Documents/Mark-main/Tasks/Notes/Agents/<agent-name>/instructions.md
```

The `<agent-name>` is lowercased and hyphenated (e.g. `Follow-up` → `follow-up`, `Research` → `research`).

If no file exists at the new path, fall back to the legacy path:
```
~/Documents/Mark-main/Tasks/Notes/Instructions/<agent-name>.md
```

If no instructions file exists at either path, the subagent uses only the standing instructions in this skill.

## Step 3: Read Memory for Each Active Agent

For each agent that has open tasks, read its memory from:
```
~/Documents/Mark-main/Tasks/Notes/Agents/<agent-name>/memory.md
```

Memory contains patterns and preferences learned from past tasks. If no memory file exists, proceed without it — an empty memory is not an error.

Pass the memory content to the subagent alongside the instructions. The subagent should treat memory as persistent context that informs how it works, not as instructions to follow literally.

## Step 4: Spawn Subagents — One Per Task for Automator, One Per Agent Type for Others

**For the `Automator` agent type:** spawn one subagent **per task** (not one for all tasks). Automator tasks are independent and often involve slow MCP calls (Jira, Slack) — parallelizing them cuts total wall-clock time from 15+ minutes to the duration of the slowest single task.

**For all other agent types** (Follow-up, Reviewer, Writer, Research, Notes-Leadership, etc.): spawn one subagent per agent type as before, passing all tasks for that agent in a single prompt.

**The archive task (title matches "Archive Closed Tasks...", `recur:monthly`) no longer needs special sequencing.** Tasks now live in a SQLite database (`tasks.db`), and each `task rm <id>` / `task edit <id>` call is its own atomic, transactional operation — unlike the old markdown file, there is no whole-file rewrite for concurrent subagents to race on. The archive task can run in the same parallel batch as everything else; just have it delete each archived task individually with `task rm <id>` rather than doing any bulk file rewrite.

Each subagent receives:

1. The agent's full instructions from its instructions file
2. The agent's memory from its memory file (if it exists)
3. The specific task(s) assigned to that subagent (task ID, title, description, metadata, attached notes)
4. The following standing instructions (apply to all subagents regardless of agent type):

---

**Standing instructions for all subagents:**

You are working tasks assigned to `agent:<name>`. Follow the instructions provided for your agent type exactly. Apply your memory as persistent context — it reflects patterns and preferences learned from previous work.

**If a task's `work_status` is `changes-requested`**, first read its full review thread — every round, not just the newest — before doing anything else:
```
task note show task-<id>-review-thread
```
This is a running conversation between you and the human; earlier rounds carry context (what was tried, what was explained) that matters for getting this round right. Only after reading the whole thread should you proceed to "For each task, before doing any work" below.

**If a `changes-requested` task has a `task-<id>-question` note ending in an `## Answer — <date>` section**, the human answered your `needs-input` question from the web UI instead of the terminal. Read the question note (`task note show task-<id>-question`), treat that answer as the response to your question, set `--work-status in-progress`, and continue the task — do not ask the question again.

**For each task, before doing any work:**
1. Read the task title and description carefully
2. Read all notes attached to the task (`notes` field, comma-separated slugs, shown by `task show <id>`). Use `task note show <slug>` to read each one.
   - **Instruction notes first**: notes whose slug or title contains `instructions`, `how-to`, or `steps` must be read before anything else — they contain task-specific guidance that overrides or supplements your agent instructions
   - **Other notes**: read for context after instruction notes
3. If anything is unclear after reading the task and all its notes, use `AskUserQuestion` to ask the human before proceeding (see "Any time you have a question" below for the `needs-input` handshake to run alongside it)
4. Update the task title and description to reflect the clarified scope (`task edit <id> --title "..." --description "..."`)
5. Mark the task as started: `task edit <id> --work-status in-progress`

**While working, keep `work_status` current** — it's how the human tracks your progress without you having to mark the task done. If a task genuinely spans multiple distinct phases and you want to signal that, update it again mid-task (e.g. back to `in-progress` after a pause); otherwise setting it once at the start is enough until handback.

**Work tasks in this order:**
1. **Due date** — earliest due date first; tasks with no due date go last
2. **Priority** — within the same due date: `critical` → `high` → `medium` → `low`

Tasks that are overdue (due date before today) come before tasks due today, which come before tasks due in the future.

**Any time you have a question — before starting, or mid-work — ask it and keep going:**
- Before calling `AskUserQuestion`, surface the question in a note so it shows up as a badge in the command-center web UI even if the human isn't watching this terminal:
  ```bash
  task note show task-<id>-question 2>/dev/null   # check whether it already exists
  ```
  If that fails, create it — `task note add`'s slug comes from slugifying the title (there's no `--slug` flag), so the title must be exactly `Task <id> Question` for the slug to come out as `task-<id>-question`:
  ```bash
  task note add "Task <id> Question" --task <id>
  ```
  Either way, set the question text (`--body` replaces the whole body, which is correct since only one question is active at a time) and set the status:
  ```bash
  task note edit task-<id>-question --body "<the question>"
  task edit <id> --work-status needs-input
  ```
- Then use `AskUserQuestion` immediately, whether the question arises before you start or partway through the work
- Once answered, set `task edit <id> --work-status in-progress` (or whatever status the task was in before the question) so the `needs-input` badge clears, then continue working the task to completion in the same session — do not stop, park, or hand the task back just because a question came up
- Never guess at an answer instead of asking, and never silently skip part of a task because something was unclear

**After completing each task, hand it back for review:**
- Ensure the artifact is reflected in the task's linked note (create one via `task note add "<title>" --task <id>` and link it via `task note link <slug> <id>` if none exists yet)
- **If this was a `changes-requested` round** (a `task-<id>-review-thread` note already exists), append your response to that same thread instead of only relying on the artifact note:
  ```
  task note append task-<id>-review-thread --body "## Agent response — <YYYY-MM-DD>

  <summary of what changed>"
  ```
  This keeps the whole back-and-forth in one place the human can read top to bottom.
- Set `task edit <id> --work-status waiting-for-review`
- (`task edit` sets the `updated` timestamp automatically — no need to set it yourself)
- Consider whether to update memory (see Updating Memory below)

**For recurring tasks (a `recur` value is set): do NOT touch the due date yourself.** Leave it exactly as it is and only set `work_status`. The human reviews the note, decides the cycle is actually done, and runs `task done <id>` themselves — `done` already knows the recurrence rule and computes the correct next due date automatically, spawning the next occurrence at that point. An agent pre-advancing `--due` on a still-open task both duplicates that logic and makes the task silently vanish from "due today" views before the human ever reviewed it — do not do this, even if old memory notes say to.

**Rules:**
- Never mark a task complete (`task done <id>`) yourself — setting `--work-status waiting-for-review` signals readiness; the human reviews and marks it done
- **Never edit `--due` on a recurring task.** Only `done` (run by the human) advances it, via the recurrence engine's next-occurrence spawn.
- **Never reassign a task's `agent` field to `human`.** The task stays owned by your agent profile; `work_status` alone signals handback. The human finds review-ready tasks by filtering/grouping on `work_status` (the web UI's Work status group-by/badge, or `task show <id>`), not by `agent`.
- **Never set `--work-status complete` as a substitute for handback or for `task done`** — it does not mark the task done. Use `waiting-for-review` as described above; only the human decides when a task is actually `complete`/done.
- If a task genuinely cannot be completed even after asking clarifying questions, write a note explaining why and set `--work-status waiting-for-review` so it surfaces for review
- Do not fabricate results

See the `task-manager` skill for the full CLI command reference and note commands.

---

## Updating Memory

After completing tasks, each subagent should consider whether to update its memory file. Memory captures learned context that improves future work — it is not a task log.

**Update memory when:**
- A preference or pattern has been observed at least twice (e.g. "Mark prefers bullet points over prose in summaries")
- A standing fact is established (e.g. a recurring contact, a known project constraint, a consistent tone preference)
- A past mistake is being corrected (e.g. "Previously wrote formal tone — Mark prefers casual for internal docs")

**Do NOT update memory for:**
- One-off task details (those belong in the task note)
- Information specific to a single task that won't recur
- Anything likely to change within the current sprint

**How to write:**
Use `task agent memory <name> edit --body "<updated content>"` or write the file directly at `Notes/Agents/<name>/memory.md`.

**Suggested memory sections:**
```markdown
## Preferences
- Mark prefers bullet points over prose in summaries
- Casual tone for internal documents; formal for external

## Patterns
- Follow-up tasks for Jira issues often require checking the related Slack thread too
- Research tasks on architecture topics: check Confluence before Slack

## Standing Context
- Primary Jira project: ITS
- Slack workspace: itential
```

Memory is free-form markdown — add, edit, or remove sections as needed. Review and prune outdated entries periodically.

## Step 5: Run All Subagents in Parallel

All subagents are independent — spawn them all in a single message so they run concurrently. This includes: every per-task Automator subagent (including archive, per the note in Step 4) AND every per-type subagent for other agents. Send one message with all Agent tool calls at once. There is no longer a sequencing requirement for the archive task — each subagent's `task` CLI calls are individually transactional against the SQLite database, so concurrent writes from different subagents can't clobber each other.

Any subagent with a question — at any point in its work, not just at the start — sets `--work-status needs-input` and writes the question to `task-<id>-question` (see "Any time you have a question" in Step 4 above) before surfacing it via `AskUserQuestion`, then clears back to its prior status and keeps working once answered. Batch questions across tasks where possible to avoid repeated interruptions, but never let a question stop a subagent from finishing its task.

Wait for all subagents to report completion before moving to Step 6 — do not proceed on partial results.

## Step 6: Report Back

After all subagents complete, summarize to the human:
- Which agents ran and how many tasks each worked
- Any memory updates made
- Tasks now marked `work_status: waiting-for-review` (ready for your review)
- Any tasks that were `changes-requested` rounds — note these explicitly, since the human already reviewed this task once and left feedback
- Any questions that came up mid-work and how they were resolved
- Any tasks that could not be completed and why

## Quick Reference

| Action | Location / Command |
|--------|-------------------|
| Config file (macOS) | `~/Library/Application Support/task-manager/config.md` |
| Tasks database | `~/Documents/Mark-main/Tasks/tasks.db` |
| List an agent's open tasks | `task list --agent <name> --status open` |
| Show full task detail | `task show <id>` |
| Mark task started | `task edit <id> --work-status in-progress` |
| Read a review thread in full | `task note show task-<id>-review-thread` |
| Respond to feedback | `task note append task-<id>-review-thread --body "## Agent response — <date>\n\n<summary>"` |
| Mark done | `task done <id>` (never do this yourself for others' tasks — this is also what advances a recurring task's due date and spawns the next occurrence, so leave `--due` alone on recurring tasks and let the human trigger this) |
| Delete (archive) a task | `task rm <id>` |
| Agent instructions | `~/Documents/Mark-main/Tasks/Notes/Agents/<name>/instructions.md` |
| Agent memory | `~/Documents/Mark-main/Tasks/Notes/Agents/<name>/memory.md` |
| Show memory | `task agent memory <name> show` |
| Update memory | `task agent memory <name> edit --body "<content>"` |
| Create note | `task note add "<title>" --task <id>` |
| Ask a question, any time | write question to `task-<id>-question` note (title exactly `Task <id> Question` if creating) + `--work-status needs-input`, then `AskUserQuestion` — clear back to prior status once answered, then keep working |
| Hand back for review | `task edit <id> --work-status waiting-for-review` |
| Timestamp format | ISO 8601 UTC e.g. `2026-05-08T14:00:00+00:00` (set automatically by the CLI) |
