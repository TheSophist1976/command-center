## Context

The TUI already has a working async pattern for NLP: `nlp_pending: Option<mpsc::Receiver<...>>` with a spinner that cycles "Thinking", "Thinking.", etc. during the ~200ms event loop tick. The pattern works well but is hardcoded to NLP. Three other operations block the UI:

1. **Todoist import** (`todoist::run_import`) — network calls to fetch tasks, can take 2-5s
2. **Slack sync** (`sync_slack_inbox`) — fetches messages from multiple channels, resolves users, 2-10s
3. **Slack channel discovery** (`open_slack_channel_picker` → `fetch_channels`) — lists all conversations, 1-3s

The event loop uses a 200ms poll timeout, which is already suitable for spinner animation.

## Goals / Non-Goals

**Goals:**
- Show animated spinner in the footer during all blocking network operations
- Keep the TUI responsive (redraws, quit with `q`) while operations run
- Allow Esc to cancel/dismiss an in-progress operation
- Consolidate the NLP spinner pattern into a shared mechanism

**Non-Goals:**
- Progress bars or percentage tracking — spinner is sufficient
- Concurrent operations — one background task at a time
- Streaming/partial results — operation completes then result is applied

## Decisions

### 1. Unified BackgroundTask enum

**Decision:** Replace the NLP-specific `nlp_pending` field with a generic `bg_task` field that handles all async operations.

```rust
enum BgTaskKind {
    TodoistImport,
    SlackSync,
    SlackChannelFetch,
}

// In App:
bg_task: Option<(BgTaskKind, mpsc::Receiver<BgTaskResult>)>,
bg_spinner_frame: u8,
```

`BgTaskResult` is an enum with variants for each operation's return type. When the receiver has a value, the result is applied based on the `BgTaskKind`.

**Why:** Single field, single check in the event loop, single spinner. The NLP pending stays separate since it has its own mode (NlpChat) and different lifecycle.

### 2. Spinner display

**Decision:** Reuse the existing footer status message area. While a background task is active, the footer shows an animated message like "Syncing Slack ⠋" cycling through braille spinner chars `⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏` at 200ms intervals (one per event loop tick).

Each `BgTaskKind` has a display label: "Importing from Todoist", "Syncing Slack", "Loading channels".

### 3. Result application

**Decision:** When the background task completes, the result is applied in the main event loop (same thread that owns App). This avoids any shared-state issues. The pattern:

1. Spawn thread, send result over channel
2. Each event loop tick: `try_recv()` on the channel
3. On `Ok(result)`: apply result to App state, clear `bg_task`
4. On `Empty`: increment spinner, continue
5. On `Disconnected`: clear `bg_task`, show error

### 4. Cancellation via Esc

**Decision:** Pressing Esc while a background task is running sets `bg_task = None` (drops the receiver). The spawned thread continues but its result is discarded when it tries to send. This is simple and avoids needing cancellation tokens.

### 5. Keep NLP separate

**Decision:** Don't merge the NLP async pattern into this system. NLP has its own mode (NlpChat), conversation state, and different result handling. Merging would add complexity for no gain. The two systems coexist independently.

## Risks / Trade-offs

**[Thread leaks on cancel]** → Cancelled threads run to completion but their results are dropped. Acceptable since these are short-lived HTTP calls (seconds, not minutes).

**[One-at-a-time limitation]** → If user presses `s` (Slack sync) while an import is running, the second operation is rejected with a status message. Simple and avoids race conditions.

**[Todoist import mutates task_file]** → `run_import` takes `&mut task_file`. For the background thread, we clone the task_file, run import on the clone, then merge results back. Alternatively, pass only the token and return the imported tasks for the main thread to insert.
