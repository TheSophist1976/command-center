## Context

The NLP API call (`nlp::interpret`) blocks the main thread for 1-10+ seconds (especially with tool-use loops). Currently a static "Thinking..." message is shown. The main event loop cannot animate anything during this blocking call.

Additionally, the overdue check in the task row renderer uses `d < Local::now().date_naive()`, which correctly excludes today — but the user reports tasks due today being shown as overdue, suggesting the comparison may have a subtle issue or the `today` value differs between contexts.

## Goals / Non-Goals

**Goals:**
- Show an animated indicator (e.g., "Thinking." → "Thinking.." → "Thinking...") while waiting for NLP
- Ensure tasks due today are never styled as overdue

**Non-Goals:**
- No async runtime (tokio, etc.) — keep it simple with std::thread
- No progress bar for tool-use steps

## Decisions

### 1. Background thread for NLP call

Spawn `nlp::interpret` on a `std::thread` and poll for the result using `mpsc::channel`. The main event loop continues running with a short poll timeout (~200ms), allowing the UI to redraw and animate the status message. When the result arrives on the channel, process it as before.

Use `std::sync::mpsc::channel` to send the result back. Store the `Receiver` in app state while waiting.

### 2. Animated dots in status message

Cycle through "Thinking", "Thinking.", "Thinking..", "Thinking..." on each redraw (~200ms). Use a simple frame counter in `App` state. Reset when the NLP result arrives.

### 3. Overdue fix: ensure `d < today` not `d <= today`

The current code uses `d < Local::now().date_naive()`. Verify this is correct and consistent across both `View::matches` and the row renderer. If the issue is a timezone edge case where `Local::now()` is called at different times, compute `today` once and pass it through.

## Risks / Trade-offs

- [Thread safety] `nlp::interpret` uses `reqwest::blocking` which is fine on a background thread. Task data is cloned for the thread.
- [Complexity] Adding threading is more complex, but necessary for animation. The channel pattern is straightforward.
