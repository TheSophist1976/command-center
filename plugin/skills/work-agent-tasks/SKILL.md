---
name: work-agent-tasks
description: Use when asked to "work my tasks", "work tasks", "execute my tasks", "run my tasks", or any similar instruction to dispatch agents for each agent type and work all assigned tasks from the task-manager system.
---

# Work Agent Tasks

Orchestrate task execution by giving each agent type that has open tasks its own worker. Each worker receives its specific instructions, memory, and tasks.

A "worker" is whatever your harness offers for delegated work: a subagent, a parallel session, a background job. If your harness has none, work the agent types one after another in the same order; everything below still applies.

The `task` CLI finds `tasks.db` on its own (`default-dir` in the config file, the `TASK_FILE` environment variable, or the global `--file <path>` flag). Never hardcode a database path. See the `task-manager` skill for the full CLI reference, the review-thread flow, and the question handshake.

## Step 1: Discover Agent Types and Their Tasks

Read the config file: `~/Library/Application Support/task-manager/config.md` (macOS) or `~/.config/task-manager/config.md` (Linux).

Find all `agent-<name>: <dir>` entries — these are the registered agent types (e.g. `Research`, `Follow-up`, `Writer`, `Reviewer`, `Automator`).

For each agent name, run `task list --agent <name> --status open` **using the exact casing from the config file** (e.g. `Follow-up`, not `follow-up`). The `--agent` filter is a case-sensitive exact match, so `--agent follow-up` silently matches nothing even though `--agent Follow-up` finds real tasks. Use `task show <id>` on each task to get full detail (description, instructions, notes, recurrence, work_status).

**Skip the following tasks:**
- Recurring tasks (a `recur` value shown in `task show <id>`) whose due date is in the future — these are scheduled for a later cycle and should not be worked early
- Tasks whose `work_status` is already `waiting-for-review` or `complete` — these were already handed back by a prior run and are awaiting the human's review or `task done`. Handback does not reassign `agent`, so these stay in the `--agent <name>` list until the human acts; re-picking them up would clobber that handback.

**Do NOT skip tasks whose `work_status` is `changes-requested`** — this is exactly what this step should pick up. It means a human left feedback on a `task-<id>-review-thread` note, or answered a question, and the task needs another round.

(Tasks with `agent:human` or no `agent` field never show up in an `--agent <name>` list, so no extra filtering is needed for those.)

Build a map of `agent-name → [eligible open tasks]`. Only agents with at least one eligible task need a worker.

## Step 2: Read Instructions and Memory for Each Active Agent

For each agent that has eligible tasks, run (with the same exact name as in the config file — the CLI maps it to the right note on disk, including the legacy location):

```
task agent instructions <name> show
task agent memory <name> show
```

If either prints "No … found", that is not an error: use only the standing instructions in this skill. Memory contains patterns and preferences learned from past tasks; the worker should treat it as persistent context that informs how it works, not as instructions to follow literally.

## Step 3: Start Workers

**One worker per agent type**, given all of that agent's tasks in a single prompt. **Exception — independent, slow tasks:** for agent types whose tasks are independent of each other and often wait on slow external calls (the `Automator` profile is the usual example), start one worker **per task**, so total time is the slowest single task rather than the sum.

Each `task` command that writes is a single atomic transaction, so workers can run concurrently without overwriting each other. That includes archive-style tasks: have them delete each archived task individually with `task rm <id>`.

Each worker receives:

1. The agent's instructions (from Step 2)
2. The agent's memory (from Step 2), if any
3. The specific task(s) it owns: id, title, description, instructions, metadata, attached notes
4. The standing instructions below

Start all workers at once if your harness can run them in parallel. Wait for every worker to finish before reporting.

---

**Standing instructions for all workers:**

You are working tasks assigned to `agent:<name>`. Follow the instructions provided for your agent type exactly. Apply your memory as persistent context — it reflects patterns and preferences learned from previous work.

**If a task's `work_status` is `changes-requested`**, first read its full review thread — every round, not just the newest — before doing anything else:
```
task note show task-<id>-review-thread
```
This is a running conversation between you and the human; earlier rounds carry context (what was tried, what was explained) that matters for getting this round right.

**If a `changes-requested` task has a `task-<id>-question` note ending in an `## Answer — <date>` section**, the human answered your `needs-input` question from the web UI. Read the question note (`task note show task-<id>-question`), treat that answer as the response, set `--work-status in-progress`, and continue — do not ask the question again.

**For each task, before doing any work:**
1. Read the task title and description, **and the task's own instructions**: run `task show <id>` and read the `## Instructions` section if it has one. These are what the human wants done for this specific task and they supplement your agent instructions. Never rewrite them yourself — if they are unclear or conflict with your agent instructions, ask (step 3).
2. Read all notes attached to the task (the `notes` field shown by `task show <id>`) with `task note show <slug>`.
   - **Instruction notes first**: notes whose slug or title contains `instructions`, `how-to`, or `steps` carry task-specific guidance that overrides or supplements your agent instructions.
   - Read the other notes for context afterwards.
3. If anything is unclear, ask — see "Questions" below.
4. Update the task title and description to reflect the clarified scope (`task edit <id> --title "..." --description "..."`); leave `--instructions` as the human wrote it.
5. Mark the task started: `task edit <id> --work-status in-progress`

**While working, keep `work_status` current** — it is how the human tracks your progress without you marking the task done. Set it once at the start; update it mid-task only if the task has distinct phases or you pause for a question.

**Work tasks in this order:**
1. **Due date** — earliest first; tasks with no due date go last. Overdue before due today before due later.
2. **Priority** — within the same due date: `critical` → `high` → `medium` → `low`

**Questions — before starting or mid-work:**
- Follow the "Asking a Question" steps in the `task-manager` skill: write the question to the `task-<id>-question` note and set `--work-status needs-input`, so it shows as a badge in the web UI even if the human is not watching your session.
- Then put the question to the human with whatever interactive question mechanism your harness provides, and continue once answered: set `--work-status` back to `in-progress` (or the prior status) and finish the task in the same session.
- If your harness cannot ask interactively, leave the task at `needs-input`, move on to your next task, and list it in your report. The human answers in the web UI and the task comes back as `changes-requested`.
- Never guess an answer, and never silently skip part of a task because something was unclear.

**After completing each task, hand it back for review:**
- Make sure the result is reflected in a note linked to the task (create one with `task note add "<title>" --task <id>` if none exists).
- **If this was a `changes-requested` round** (a `task-<id>-review-thread` note exists), append your response to that thread:
  ```
  task note append task-<id>-review-thread --body "## Agent response — <YYYY-MM-DD>

  <summary of what changed>"
  ```
- Set `task edit <id> --work-status waiting-for-review`. (`updated` is set automatically.)
- Consider whether to update memory (see below).

**Recurring tasks (a `recur` value is set): do NOT touch the due date.** Leave it exactly as it is and only set `work_status`. The human reviews, decides the cycle is done, and runs `task done <id>` themselves, which computes the next due date and spawns the next occurrence. Pre-advancing `--due` yourself duplicates that logic and makes the task vanish from "due today" views before anyone reviewed it — even if old memory notes say to.

**Rules:**
- Never run `task done <id>` yourself — `--work-status waiting-for-review` signals readiness; the human reviews and closes the task.
- Never edit `--due` on a recurring task.
- Never reassign a task's `agent` field to `human`. The task stays owned by your agent profile; `work_status` alone signals handback.
- Never set `--work-status complete` as a substitute for handback or for `task done`.
- If a task cannot be completed even after asking, write a note explaining why and set `--work-status waiting-for-review` so it surfaces for review.
- Do not fabricate results.

---

## Updating Memory

After completing tasks, each worker should consider whether to update its agent's memory. Memory captures learned context that improves future work — it is not a task log.

**Update memory when:**
- A preference or pattern has been observed at least twice
- A standing fact is established (a recurring contact, a project constraint, a consistent tone preference)
- A past mistake is being corrected

**Do NOT update memory for:**
- One-off task details (those belong in the task note)
- Anything specific to a single task that will not recur
- Anything likely to change within the current sprint

**How to write:** `task agent memory <name> edit --body "<updated content>"`

Memory is free-form markdown. Suggested sections: `## Preferences`, `## Patterns`, `## Standing Context`. Review and prune outdated entries periodically.

## Step 4: Report Back

After all workers finish, summarize to the human:
- Which agents ran and how many tasks each worked
- Any memory updates made
- Tasks now `waiting-for-review`
- Tasks that were `changes-requested` rounds — call these out, since the human already reviewed them once
- Questions that came up and how they were resolved, and any tasks left at `needs-input`
- Tasks that could not be completed, and why

## Quick Reference

| Action | Command |
|--------|---------|
| List an agent's open tasks | `task list --agent <name> --status open` |
| Full task detail (incl. `## Instructions`) | `task show <id>` |
| Mark started | `task edit <id> --work-status in-progress` |
| Read a review thread | `task note show task-<id>-review-thread` |
| Respond to feedback | `task note append task-<id>-review-thread --body "## Agent response — <date>\n\n<summary>"` |
| Ask a question | write `task-<id>-question` note + `--work-status needs-input` (see `task-manager`) |
| Hand back for review | `task edit <id> --work-status waiting-for-review` |
| Read agent instructions / memory | `task agent instructions <name> show` / `task agent memory <name> show` |
| Update memory | `task agent memory <name> edit --body "<content>"` |
| Create a linked note | `task note add "<title>" --task <id>` |
| Mark done | `task done <id>` — the human's call; also what advances a recurring task |
| Delete (archive) a task | `task rm <id>` |
