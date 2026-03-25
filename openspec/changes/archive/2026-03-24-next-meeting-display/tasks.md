## 1. Meeting fetch function

- [ ] 1.1 Add `fetch_next_meeting() -> Option<String>` in `src/tui.rs` (or a new `src/calendar.rs`) — runs `osascript` with the Outlook AppleScript, parses output, returns formatted `"HH:MM AM/PM — <title>"` string (truncated to 40 chars), wrapped in `#[cfg(target_os = "macos")]`
- [ ] 1.2 Add a no-op stub `fetch_next_meeting() -> Option<String>` returning `None` under `#[cfg(not(target_os = "macos"))]`

## 2. App state and background refresh

- [ ] 2.1 Add `next_meeting: Option<String>` field to the `App` struct, initialized to `None`
- [ ] 2.2 Add `meeting_rx: Option<mpsc::Receiver<Option<String>>>` field to `App`, initialized to `None`
- [ ] 2.3 In `App::new()` (or the TUI `run()` function), spawn the background refresh thread: fetch immediately, then loop with 5-minute sleep, sending results to the channel
- [ ] 2.4 In the main event loop tick handler, call `meeting_rx.try_recv()` and update `app.next_meeting` when a new value arrives

## 3. Header rendering

- [ ] 3.1 Update `draw_header()` to render `next_meeting` on a second line when `Some` — use a dimmer style (e.g., `Color::Rgb(160, 160, 160)`) and prefix with a calendar icon or `↗` indicator
- [ ] 3.2 Update every layout in `draw()` that sets `Constraint::Length(1)` for the header to instead use `if app.next_meeting.is_some() { Constraint::Length(2) } else { Constraint::Length(1) }`

## 4. Verification

- [ ] 4.1 Run `cargo build --features tui` — confirm zero errors
- [ ] 4.2 Run `cargo test --features tui` — confirm all tests pass
