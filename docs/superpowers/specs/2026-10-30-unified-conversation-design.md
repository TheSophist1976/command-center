# Unified Agent Conversation Design

## Overview

The web UI has two panels for talking to an agent: `ReviewPanel` (feedback on
finished work) and `QuestionPanel` (answering a `needs-input` question). They
use two notes (`task-N-review-thread`, `task-N-question`), two endpoints, and
both end by setting `changes-requested`. The agent tells them apart by
checking whether the question note ends in `## Answer`. `ReviewPanel` also has
an always-live composer, so feedback can be sent in any state, flipping the
task to `changes-requested`.

This spec replaces both with **one conversation per task**: one note, one
panel, one endpoint. What the composer offers, and what Send does, is derived
from `work_status` by the server.

## Decisions already made (confirmed with the user)

- **One note** per task: `task-N-thread`. Not two notes merged on read.
- **Unsolicited messages are allowed** while the agent is working
  (`todo` / `in-progress`), without changing `work_status`.
- **Approve lives in the panel**, in the `waiting-for-review` composer.
- **Mobile uses the same component** (both panels already render via
  `TaskDetail`).

## Thread format

A single append-only markdown note. Every section is
`## <Kind> — <YYYY-MM-DD>`; the kind is parsed from the heading, never
guessed from a regex over free text.

| Kind | Author | Meaning |
|------|--------|---------|
| `Agent` | agent | Reply / hand-off summary |
| `Question` | agent | Blocking question (task goes `needs-input`) |
| `Answer` | human | Reply to a `Question` |
| `Feedback` | human | Change request on `waiting-for-review` work |
| `Message` | human | Unsolicited note while the agent is working |
| `Approval` | human | Review accepted (optional body) |
| `Snapshot` | system | Pinned review commit (already written today) |

Unknown headings render as a human message. Legacy `## Agent response` and
`## Feedback` headings parse as `Agent` / `Feedback`.

## State machine

The server computes `allowed_action` per task from `work_status`; the UI only
renders it.

| work_status | Composer | Send records | Resulting work_status |
|---|---|---|---|
| `needs-input` | "Answer" (focused, call-to-action) | `Answer` | `changes-requested` |
| `waiting-for-review` | "Request changes" + **Approve** button | `Feedback` / `Approval` | `changes-requested` / `complete` |
| `todo`, `in-progress`, `changes-requested` | "Message the agent" | `Message` | unchanged |
| `complete` or none | none; history collapsed | rejected | — |

Approve sets `complete` only. It never marks the task done (unchanged rule from
the work-status spec). `POST` to a state that does not allow the message kind
returns 409.

### Unread messages

No new column. A thread is *unread* when its last non-`Snapshot` section is
human-authored (`Answer`, `Feedback`, `Message`, `Approval`). An `Agent` or
`Question` section clears it. This derives from the note alone, so it survives
without migrations and the UI and agent compute the same answer.

## API

- `GET /api/tasks/:id/thread` → `{ note, allowed_action, unread }`.
- `POST /api/tasks/:id/thread` with `{ kind, text }` where `kind` is one of
  `answer | feedback | message | approve`. The server validates `kind`
  against `allowed_action`, appends the section, links the note, applies the
  transition, and returns the note — all under the existing write lock, as
  `add_task_review_feedback` does today.
- Removed: `/review` and `/question` routes, `QuestionPanel`, `ReviewPanel`.
- The freeze-before-review snapshot (`record_review_snapshot`) writes into
  `task-N-thread` instead.

## Agent side

- `task next --json` entries carry `thread` (note body or null) and `unread`
  instead of `review_thread` + `question`. `pickup` values stay
  (`review_feedback`, `answered_question`) and are now derived from the last
  section's kind. A new `message` pickup covers an unread `Message` on an
  `in-progress` or `todo` task. Today `task next` already returns
  `in-progress` tasks as `continue`; the unread flag tells the agent there is
  something new to read.
- New CLI: `task thread post <id> --reply "..."` (appends `Agent`) and
  `task thread post <id> --question "..."` (appends `Question` and sets
  `needs-input`), replacing the hand-built `task note add/edit` sequence in
  `AGENTS.md`.
- `AGENTS.md` sections for `changes-requested` and `needs-input` are rewritten
  around the thread. This file must be updated in the same change.

## Migration (no data loss)

Lazy, on first read or write of a task that has legacy notes. Deleting the
legacy notes is allowed only after the merge is proven lossless:

1. Parse both legacy notes into sections; merge into `task-N-thread` ordered by
   section date (stable; review-thread first on ties). A legacy question body
   with no `## ` heading (agents overwrite the whole body today) becomes one
   `Question` section, dated from the note's file mtime. Nothing is dropped,
   including `Snapshot` sections and any non-section preamble.
2. Verify: every legacy section body appears verbatim in the new thread, and
   the section count matches. On any mismatch, abort and leave the legacy notes
   in place.
3. Only then relink the task (`notes` swaps legacy slugs for the thread slug)
   and move the legacy files to `Notes/.migrated/` instead of unlinking, so a
   bad merge is recoverable. Purging `.migrated/` is a manual step.
4. A task migrates in one write-locked step, so a concurrent agent never sees
   half of it. A `migrate` dry-run prints what would change for every task
   without writing.

## Other consumers of the old notes

These all hard-code `task-<id>-question` / `task-<id>-review-thread` or parse
`## Answer` and break silently if the notes disappear:

| Consumer | What it does today | Change |
|---|---|---|
| `plugin/skills/task-manager`, `skills/task-manager` (installed by `task setup`; a test enforces they are identical) | Teaches the review-thread and "Asking a Question" recipes | Rewrite around `task thread post`; keep the two copies identical |
| `plugin/skills/work-agent-tasks` | Reads `-review-thread`, appends `## Agent response`, writes `-question` | Same rewrite |
| `~/.claude/skills/task-manager`, `~/.claude/skills/work-agent-tasks` | Installed copies; **already diverged** from the repo (the installed task-manager has service/`failing` content the repo copy lacks) | Reconcile first (find which is canonical), then update; `task update` refreshes them |
| `command-center-agents` `bin/run-agent.py`, `libexec/agent-report.sh` | Write `task-<id>-question` and set `needs-input` when an agent is stuck | Switch to `task thread post --question` |
| `command-center-agents` `libexec/work-{code,home,vault}.sh`, `skill/SKILL.md` | Prompt strings say "follow its question and review-thread rules"; skill documents the stuck-flagging | Reword; no logic change |
| `vault-maintainer` (`notify.ts`, `answers.ts`, `questions.ts`, `SPEC.md`) | `putQuestion` writes the question note (with append mode); parses Mark's text under `## Answer` | Switch to the thread: `putQuestion` becomes `task thread post --question`; read answers from `Answer` sections. Its multiple-answers behaviour must be preserved by the typed headings |
| `AGENTS.md` (embedded by `src/assets.rs`) | Documents `question` / `review_thread` | Rewrite |
| `.claude/` openspec skills | Unrelated (matched on the word "review") | None |

### Rollout order

Because cron agents run continuously, the cutover cannot be atomic:

1. Ship `task thread post` and the server routes with **compat shims**:
   `task note show|append|edit` on a legacy slug resolves to the thread
   (`edit --body` on `-question` appends a `Question` section instead of
   replacing history), and `/review` + `/question` stay as deprecated aliases
   for `/thread`.
2. Update the skills, runners, `vault-maintainer`, and `AGENTS.md` to the new
   command.
3. Run the migration across all tasks (dry-run first).
4. Remove the shims in a later release once nothing writes legacy slugs.

## UI

One `ConversationPanel` using the chat-bubble layout from `ReviewPanel`
(copy button, snapshot chips, autoscroll). Question sections get an accent
treatment when `needs-input`. The collapsed-history state for non-interactive
statuses is an expandable header ("Conversation · N messages"). The composer
placeholder, button label, and Approve button come from `allowed_action`. An
unread human message shows a small "Not yet seen by agent" marker.

## Testing

- Server: transition table (each work_status × each kind, including 409s),
  unread derivation, snapshot into thread, lazy migration (both legacy notes,
  only one, neither).
- Agent: pickup derivation from last section kind; `message` pickup.
- UI: composer mode per work_status; Approve; mobile layout.

## Out of scope

- Per-message read receipts, attachments, multi-human threads.
- Changing `status` (open/done) from the panel.
