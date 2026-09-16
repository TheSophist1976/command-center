# Review Feedback Thread Design

## Overview

When an agent finishes a task and sets `work_status: waiting-for-review`, the
human currently has no structured way to give feedback and send the task
back for another round — only `work_status` itself and the general-purpose
Notes system exist, and neither captures a back-and-forth history. This
feature adds a **review thread**: a single, append-only markdown note per
task that both the agent and the human post into, rendered as a readable
thread in the web UI, with a feedback box that posts a new round and flips
the task back into "needs more work" state in one action.

## Decisions already made (confirmed with the user during brainstorming)

- **Trigger mechanism:** the human leaves feedback text, which (in one
  action) both records the feedback and changes `work_status` — there is no
  separate manual step to "reopen" a task.
- **History:** the full round-by-round history is visible, not just the
  latest round — both sides can see everything already tried/said.
- **New `WorkStatus` value:** `changes-requested`, distinct from
  `in-progress` (the agent hasn't started this round yet) and `todo` (this
  task has already been worked at least once). `--work-status` already
  accepts any valid `WorkStatus` value generically, so adding this variant
  is the only backend change needed to make
  `task edit <id> --work-status changes-requested` work everywhere.
- **Storage mechanism:** reuse the existing Notes system (Approach A from
  brainstorming), not a new database table/subsystem. A dedicated per-task
  note holds the whole thread as markdown, with a `## <role> — <date>`
  header per round. No new SQLite column or migration.
- **Markdown rendering:** add `react-markdown` as a new frontend dependency
  to render the thread in the web UI (renders to real React elements, not
  `dangerouslySetInnerHTML`, so it's safe by default for agent- and
  human-authored content).

## Ground truth checked before writing this spec

- `src/task.rs`'s `WorkStatus` enum (four variants: `Todo`, `InProgress`,
  `WaitingForReview`, `Complete`), its `Display`/`FromStr` impls, and the
  existing aliases (`"review"` is already an alias for `WaitingForReview` —
  the new variant's aliases must not collide with it).
- `src/note.rs`'s `Note { slug, title, body }`, `read_note`, `write_note`
  (errors if the file doesn't exist yet; no separate "does it exist" check
  needed beyond that).
- `src/cli.rs`'s `NoteCommand` enum (`List`, `Add`, `Show`, `Edit`, `Rm`,
  `Link`, `Unlink`) — `Edit --body` **replaces** the whole body; there is no
  existing append operation.
- `src/server.rs`'s existing notes handlers (`list_task_notes`,
  `create_task_note`, `open_note`, `unlink_task_note`) all still use the
  `db::load`/mutate/`db::save` + `state.write_lock` pattern — **not** the
  newer atomic single-row functions (`db::get_task`/`db::update_task`/
  `db::append_note`) that a separate, unrelated, already-in-progress
  refactor has added to `src/bin/task.rs`/`src/bin/task_tui.rs`/`src/db.rs`.
  The new review endpoints in this feature must follow `server.rs`'s
  existing dominant pattern (`db::load`/`db::save`/`write_lock`) for
  consistency within that file, not the newer per-row functions used
  elsewhere.
- `web/src/App.tsx`'s `WORK_STATUS_OPTIONS`/`WORK_STATUS_LABEL`/
  `WORK_STATUS_COLOR` constants and the "Work status" `FieldRow` (currently
  the last field before "Project" in the inspector).
- **A pre-existing, unrelated inconsistency, flagged to the user but not
  fixed by this feature:** `src/bin/task.rs`'s `Note` commands resolve notes
  into `<db_path parent>/Notes/` (matching `server.rs`'s `notes_dir()`
  exactly), but `src/bin/task_tui.rs`'s `Note` commands resolve notes into
  `<db_path parent>/` directly (no `Notes/` subdirectory) — a real bug where
  the TUI and the CLI/web read and write regular notes to two different
  directories. This feature is unaffected because **agents use the `task`
  CLI, never `task-tui`**, for all scripted operations — `task.rs`'s
  directory convention is the one that matters here, and it already matches
  `server.rs`. Do not fix the TUI inconsistency as part of this plan; it is
  out of scope and belongs to whatever is already mid-flight on `db.rs`.

## Design

### 1. Identifying the review thread — no new database field

Each task's review thread is a note with a **deterministic slug**:
`task-<id>-review-thread` (e.g. `task-558-review-thread`), title
`Review — <task title>`. No new `Task` field, no migration — the slug is
computed from the task id wherever it's needed (server, and documented for
agents in the skill update). "Does a review thread exist yet" is just "does
`task-<id>-review-thread.md` exist," not a lookup through `task.notes`
(though the note *is* also linked via `task.notes` once created, so it shows
up like any other linked note through existing mechanisms — see the display
filtering note in section 4).

### 2. New CLI primitive: `task note append <slug> --body "<text>"`

Appends `<text>` to the note's existing body, separated by a blank line
(does not touch the title). Requires the note to already exist — same
convention as `Edit`/`Rm`/`Show` (errors if the slug isn't found). This is a
generically useful primitive beyond this feature (any note that needs a
running log benefits from it), not something invented solely for review
threads.

**`src/note.rs`** — new function, alongside `read_note`/`write_note`:

```rust
/// Appends a markdown section to an existing note's body, separated from
/// whatever came before by a blank line. The note must already exist.
pub fn append_to_note(dir: &Path, slug: &str, section: &str) -> Result<PathBuf, String> {
    let path = dir.join(format!("{}.md", slug));
    let mut n = read_note(&path)?;
    if n.body.trim().is_empty() {
        n.body = section.to_string();
    } else {
        n.body = format!("{}\n\n{}", n.body.trim_end(), section);
    }
    write_note(dir, &n)
}
```

**`src/cli.rs`** — new `NoteCommand::Append` variant:

```rust
    /// Append a section to a note's existing body
    Append {
        /// Note slug
        slug: String,

        /// Markdown text to append (including its own header, if any)
        #[arg(long)]
        body: String,
    },
```

**`src/bin/task.rs`** — new match arm (in the existing `Command::Note`
handler, alongside `NoteCommand::Edit`/`Rm`/etc.):

```rust
                NoteCommand::Append { slug, body } => {
                    let file_path = task::note::append_to_note(&dir, &slug, &body).map_err(|e| (1, e))?;
                    println!("{}", file_path.display());
                    Ok(())
                }
```

**`src/bin/task_tui.rs`** — the exact same match arm must be added here too.
The `work_status` feature's final review caught a real build break from a
duplicated `Command::Edit` match arm in `task_tui.rs` never being updated
alongside `task.rs`; `Command::Note`'s match is likewise duplicated between
the two binaries (confirmed: both files independently implement the full
`NoteCommand` match). **Do not skip `task_tui.rs`.**

CLI callers control the full section text (including its own
`## <header> — <date>` line) via `--body`; the CLI itself has no opinion on
header format — that convention lives in the `work-agent-tasks` skill
(section 6) for agents, and in the server handler (section 3) for the
human's web-submitted feedback.

### 3. Two new server endpoints

**`src/server.rs`** — a slug helper next to the existing `notes_dir`:

```rust
fn review_thread_slug(task_id: u32) -> String {
    format!("task-{}-review-thread", task_id)
}
```

**`GET /api/tasks/:id/review`** — returns the review thread's content, or
`null` if no thread exists yet. Reuses the existing `NoteResponse` struct
(`{ slug, title, body }`) already defined in `server.rs` for the notes
endpoints — no new response type needed.

```rust
async fn get_task_review(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<Option<NoteResponse>>, (StatusCode, Json<serde_json::Value>)> {
    let task_file = db::load(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    if task_file.find_task(id).is_none() {
        return Err(app_error(StatusCode::NOT_FOUND, format!("Task {} not found", id)));
    }
    let dir = notes_dir(&state);
    let slug = review_thread_slug(id);
    let note = crate::note::read_note(&dir.join(format!("{}.md", slug)))
        .ok()
        .map(NoteResponse::from);
    Ok(Json(note))
}
```

**`POST /api/tasks/:id/review`** with body `{ "text": "<feedback>" }` —
creates the review-thread note if it doesn't exist yet, links it to the
task (if not already linked — same `if !t.notes.contains(&slug)` idiom
already used by `create_task_note`), appends a `## Feedback — <date>`
section, and sets `work_status` to `changes-requested`. All in one
`write_lock`-guarded `db::load`/mutate/`db::save` cycle, matching every
other write handler in this file.

```rust
#[derive(Deserialize)]
pub struct FeedbackRequest {
    pub text: String,
}

async fn add_task_review_feedback(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<u32>,
    Json(req): Json<FeedbackRequest>,
) -> Result<Json<NoteResponse>, (StatusCode, Json<serde_json::Value>)> {
    let text = req.text.trim();
    if text.is_empty() {
        return Err(app_error(StatusCode::BAD_REQUEST, "feedback text must not be empty"));
    }

    let _guard = state.write_lock.lock().await;
    let mut task_file = db::load(&state.db_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let dir = notes_dir(&state);
    let slug = review_thread_slug(id);
    let note_path = dir.join(format!("{}.md", slug));

    let task_title = task_file
        .find_task(id)
        .ok_or_else(|| app_error(StatusCode::NOT_FOUND, format!("Task {} not found", id)))?
        .title
        .clone();

    if !note_path.exists() {
        crate::note::write_note(
            &dir,
            &crate::note::Note { slug: slug.clone(), title: format!("Review — {}", task_title), body: String::new() },
        )
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    }

    let today = chrono::Local::now().date_naive().format("%Y-%m-%d");
    let section = format!("## Feedback — {}\n\n{}", today, text);
    crate::note::append_to_note(&dir, &slug, &section)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    {
        let t = task_file
            .find_task_mut(id)
            .expect("checked above");
        if !t.notes.contains(&slug) {
            t.notes.push(slug.clone());
        }
        t.work_status = Some(crate::task::WorkStatus::ChangesRequested);
        t.updated = Some(chrono::Utc::now());
    }
    db::save(&state.db_path, &task_file)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;

    let note = crate::note::read_note(&note_path)
        .map_err(|e| app_error(StatusCode::INTERNAL_SERVER_ERROR, e))?;
    Ok(Json(NoteResponse::from(note)))
}
```

Router addition (alongside the other `/api/tasks/:id/...` routes):

```rust
.route("/api/tasks/:id/review", get(get_task_review).post(add_task_review_feedback))
```

`GET`/`POST /api/tasks/:id/review` need no path-traversal validation the way
note-slug endpoints do — the slug is always server-computed from a `u32`
task id via `review_thread_slug`, never taken from user input.

### 4. `WorkStatus` gets a 5th value: `changes-requested`

**`src/task.rs`**:

```rust
pub enum WorkStatus {
    Todo,
    InProgress,
    WaitingForReview,
    ChangesRequested,
    Complete,
}
```

```rust
            WorkStatus::ChangesRequested => write!(f, "changes-requested"),
```

```rust
            "changes-requested" | "changes_requested" | "changesrequested" | "changes" => Ok(WorkStatus::ChangesRequested),
```

(placed so as not to collide with `"review"`, already claimed by
`WaitingForReview`). Update the `_ =>` error message's "Valid values" list
to include `changes-requested`.

No other backend change is required for `--work-status changes-requested`
to work end-to-end — the CLI flag, the server's clearing-pattern parser, and
`db.rs`'s persistence all already operate generically over every
`WorkStatus` variant via `Display`/`FromStr`.

### 5. Web: a "Review" panel in the inspector

**New dependency**: `react-markdown` in `web/package.json`.

**New file `web/src/components/ReviewPanel.tsx`** (mirrors the existing
`NotesSection.tsx` pattern: a focused component owning its own compose-box
state, taking data + callbacks as props):

- Props: `review: Note | null`, `onSendFeedback: (text: string) => void`.
- Renders `review.body` through `<ReactMarkdown>{review.body}</ReactMarkdown>`
  if `review` is non-null, else a placeholder ("No review thread yet.").
- A `<textarea>` + "Send feedback" button below the rendered thread.
  Submitting calls `onSendFeedback(text)` and clears the box; does not
  clear it if the parent call fails (surfacing the error is the parent
  App's existing `setError` convention — `ReviewPanel` itself does no
  fetching).

**`web/src/api.ts`** — two new functions:

```typescript
export async function fetchTaskReview(taskId: number): Promise<Note | null> {
  return jsonOrThrow(await fetch(`/api/tasks/${taskId}/review`));
}

export async function postTaskFeedback(taskId: number, text: string): Promise<Note> {
  return jsonOrThrow(
    await fetch(`/api/tasks/${taskId}/review`, {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ text }),
    }),
  );
}
```

**`web/src/App.tsx`**:

- New state: `const [review, setReview] = useState<Note | null>(null);`
- New effect, mirroring the existing `taskNotes` effect (fetch on
  `selected?.id` change, clear to `null` when nothing is selected):
  ```typescript
  useEffect(() => {
    if (!selected) {
      setReview(null);
      return;
    }
    fetchTaskReview(selected.id)
      .then(setReview)
      .catch((e) => setError(String(e)));
  }, [selected?.id]);
  ```
- New handler:
  ```typescript
  async function handleSendFeedback(text: string) {
    if (!selected) return;
    try {
      const note = await postTaskFeedback(selected.id, text);
      setReview(note);
      const updated = { ...selected, work_status: 'changes-requested' as const };
      setSelected(updated);
      setTasks((prev) => prev.map((t) => (t.id === updated.id ? updated : t)));
    } catch (e) {
      setError(String(e));
    }
  }
  ```
  (The existing SSE subscription will also eventually reconcile this from
  the file-watcher broadcast, but updating local state immediately avoids a
  visible lag between clicking "Send feedback" and the badge/dropdown
  reflecting the new status.)
- `WORK_STATUS_OPTIONS`/`WORK_STATUS_LABEL` get a `changes-requested` /
  `'Changes Requested'` entry each. `WORK_STATUS_COLOR` gets
  `'changes-requested': 'var(--danger)'` — every other token in
  `tokens.css` is already claimed by an existing status/priority meaning;
  reusing `--danger` (already "needs attention" for overdue tasks and
  critical priority) is semantically consistent rather than a clash, since
  changes-requested is likewise "this needs action now."
- New `<FieldRow label="Review">`-equivalent block placed directly after
  the existing "Work status" `FieldRow` (before "Project"), rendering
  `<ReviewPanel review={review} onSendFeedback={handleSendFeedback} />`.
  This is not itself a `FieldRow`/`EditableField` (the panel has its own
  internal layout, unlike the single-line fields around it) — a plain
  wrapping `<div>` with the same left margin/spacing as the surrounding
  `FieldRow`s is sufficient; match whatever spacing constant the
  `NotesSection` block already uses below the fields column, since
  `ReviewPanel` is closer in shape to `NotesSection` (a whole content
  block) than to a single-line field.
- **Filter the review-thread note out of the general Notes section**: in
  the `taskNotes`-driven list rendered by `NotesSection`, exclude any note
  whose slug equals `task-${selected.id}-review-thread` (compute this once,
  e.g. `const reviewSlug = selected ? `task-${selected.id}-review-thread` : null;`
  and filter `taskNotes` by `n.slug !== reviewSlug` before passing to
  `<NotesSection>`). Without this, the same content would appear twice: once
  in the general Notes list (which links out to an external editor) and
  once in the dedicated Review panel (rendered inline as markdown) —
  confusing and redundant.

### 6. `work-agent-tasks` skill update (outside this git repo)

**File**: `~/.claude/skills/work-agent-tasks/SKILL.md` — not tracked by this
repository's git, edited directly the same way the `work_status` feature's
skill updates were (see that feature's session history). Not part of the
implementation plan's tracked tasks below, but must be done once the
backend/CLI pieces land, and is listed here so it isn't forgotten.

Add to Step 1 (task discovery)'s "Skip the following tasks" list: **do not
skip** `changes-requested` tasks — they are exactly the tasks this step
should pick up (this is the whole point of the feature), called out
explicitly so it isn't confused with the already-documented "skip
`waiting-for-review`/`complete`" rule from the prior feature.

Add a new step to "Standing instructions for all subagents," before "For
each task, before doing any work": when a task's `work_status` is
`changes-requested`, first read the full review thread —
`task note show task-<id>-review-thread` — in full, not just the latest
entry, so the agent has context from every prior round before acting.

Update the "hand it back for review" section: after doing the requested
work, append a response to the same thread —
``task note append task-<id>-review-thread --body "## Agent response — <YYYY-MM-DD>\n\n<summary of what changed>"``
— then set `work_status` back to `waiting-for-review` as already documented.

Update the Quick Reference table with the new command shape.

Also update `~/.claude/skills/task-manager/SKILL.md` (the CLI reference)
with `task note append <slug> --body "<text>"` in its Notes command table,
and `changes-requested` in its Work status valid-values line — the same
kind of sync `task-manager/SKILL.md` already needed after the `work_status`
and `--recur` features shipped.

## Testing

- `src/note.rs`: unit tests for `append_to_note` — appending to an empty
  body, appending to a non-empty body (blank-line separator), and erroring
  when the slug doesn't exist.
- `src/task.rs`: extend the existing `test_work_status_from_str_and_display`
  test with `changes-requested` and its aliases; a kebab-case serde
  round-trip test entry for the new variant, mirroring the existing
  `test_task_serialize_work_status_kebab_case` test.
- `src/commands.rs`/CLI: a test that `task note append <slug> --body "..."`
  round-trips through `task note show <slug>` (create a note via `add`,
  append twice, confirm both sections present in order, separated by a
  blank line).
- `src/server.rs`:
  - `test_get_task_review_returns_null_when_no_thread_exists`
  - `test_post_task_review_creates_thread_and_sets_changes_requested` —
    posts feedback to a task with no existing thread, asserts the response
    note's body contains the feedback under a `## Feedback —` header, and
    that a follow-up `GET /api/tasks/:id` (or the same response's parent
    task state via a second `db::load` in the test) shows
    `work_status: "changes-requested"`.
  - `test_post_task_review_appends_to_existing_thread` — post feedback
    twice, assert both sections present in the second response's body, in
    order.
  - `test_post_task_review_empty_text_returns_400`
  - `test_post_task_review_links_note_to_task` — assert the task's `notes`
    field contains the review-thread slug after posting, and that posting
    twice doesn't duplicate the link.
- `web/`: no test runner in this project (confirmed for every prior
  frontend feature) — a clean `npm run build` is the acceptance bar, same
  as always.

## Out of scope

- Fixing the pre-existing `task-tui`/`task`/`server.rs` notes-directory
  inconsistency (see "Ground truth checked" above) — flagged, not fixed,
  here.
- Any UI for browsing/searching review threads outside their owning task's
  inspector.
- Notifying anyone (agent or human) that a new round has arrived beyond the
  existing SSE-driven live task list refresh and the agent's own
  `work-agent-tasks` polling cadence.
- Structured round metadata (round numbers as data, per-round timestamps as
  queryable fields, author-type badges) — the thread is plain markdown text,
  same as every other note in this system.
