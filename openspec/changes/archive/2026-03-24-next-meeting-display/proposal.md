## Why

The TUI has no visibility into the user's schedule. Adding a "next meeting" indicator on every view gives the user a constant at-a-glance reminder of what's coming up, without leaving the task manager.

## What Changes

- Add a "next meeting" line to the TUI header on all views, showing the next upcoming meeting title and time from Outlook for Mac
- Meeting data is fetched via `osascript` (AppleScript) querying Outlook for Mac — no extra dependencies
- Data is fetched in a background thread at TUI startup and refreshed periodically (every 5 minutes)
- If Outlook is not running, not installed, or returns no events, the header shows nothing (graceful degradation)

## Capabilities

### New Capabilities
- `next-meeting-display`: Fetch the next Outlook calendar event via osascript and display it in the TUI header on all views

### Modified Capabilities

## Impact

- `src/tui.rs` — add meeting fetch logic, background refresh, and header rendering
- No new dependencies — uses `std::process::Command` to call `osascript`
- macOS only — guarded with `#[cfg(target_os = "macos")]`
