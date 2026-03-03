## Approach

Add action summary messages to the chat panel after each NLP response, and enhance the update confirmation to show per-task before→after details. All changes in `src/tui.rs` — the NLP module already returns all needed data.

## Action Summaries

After each NLP response, push a descriptive `ChatMessage::Assistant` to `chat_history` before executing the action:

| Action | Summary shown in chat |
|--------|----------------------|
| Filter | `Filtering: status=open, priority=high` (listing non-null criteria) |
| Update | `Updating: match {status=open, tag=frontend} → set {priority=high}` |
| ShowTasks | (already shows text + task list — no change needed) |
| Message | (already shows the text — no change needed) |

Format criteria as `field=value` pairs, omitting null fields.

## Detailed Update Confirmation

Currently the `ConfirmingNlp` mode shows only the footer: `"Set priority high (5 tasks) — y/n"`.

Enhanced flow:
1. When entering `ConfirmingNlp`, push chat messages listing each affected task with before→after:
   ```
   Task #3 "Fix login bug": priority Medium → High
   Task #7 "Update docs": priority Low → High
   ```
2. The footer still shows the y/n prompt with count.

### What to show per task

For each field in `set_fields` that is `Some`:
- **priority**: `priority {old} → {new}`
- **status**: `status {old} → {new}`
- **tags**: `tags [{old}] → [{new}]`

Only show fields that will actually change (skip if old == new).

### Truncation

If more than 10 tasks match, show first 10 and add `"... and N more tasks"`.

## Structure

Helper function `fn format_action_summary(action: &NlpAction) -> String` formats the action into a human-readable summary.

Helper function `fn format_update_preview(tasks: &[Task], indices: &[usize], set_fields: &SetFields) -> Vec<String>` generates per-task change lines.

## Decisions

- **No new ChatMessage variant** — use existing `ChatMessage::Assistant` for summaries
- **Show summary before executing** — user sees what the AI decided before the action takes effect
- **Truncate at 10 tasks** — prevent the chat panel from being overwhelmed
- **Skip no-op fields** — don't show "priority Medium → Medium"
