## Context

The TUI header currently shows the view name and task count. We want to add a next-meeting line visible on all views. Meeting data comes from Outlook for Mac via AppleScript, called as a subprocess.

## Goals / Non-Goals

**Goals:**
- Show next upcoming meeting (title + time) in the TUI header on all views
- Fetch asynchronously so startup is not blocked
- Refresh every 5 minutes while the TUI is running
- Degrade silently if Outlook is unavailable

**Non-Goals:**
- Multi-meeting list view
- Meeting join links or details
- Non-macOS support (Linux/Windows Outlook is out of scope)
- Configuration of which calendar to use

## Decisions

### Decision: osascript via AppleScript to query Outlook
Call `osascript -e '<script>'` as a subprocess. The AppleScript queries `Microsoft Outlook` for the first calendar event starting after now.

```applescript
tell application "Microsoft Outlook"
    set now to current date
    set upcoming to (calendar events whose start time > now)
    set upcoming to sort upcoming by start time
    if (count of upcoming) > 0 then
        set evt to item 1 of upcoming
        return (subject of evt) & "|" & (start time of evt as string)
    end if
end tell
```

Output is `<title>|<datetime>` — parse on `|`. If the script fails or returns empty, store `None`.

**Alternatives considered**: EventKit via a compiled Swift snippet — more reliable but requires compilation at deploy time. osascript is simpler and zero-dependency.

### Decision: Background thread with 5-minute refresh
On TUI startup, spawn a thread that:
1. Fetches meeting data immediately
2. Sleeps 5 minutes
3. Repeats

The thread sends results via an `mpsc::channel`. The main event loop checks the channel on each tick (same pattern as the existing `nlp_pending` channel).

### Decision: Store as `Option<String>` on App
`app.next_meeting: Option<String>` — a pre-formatted string like `"10:30 AM — Standup"` ready to render. Formatting happens in the fetch thread, not in the render path.

### Decision: Display in header bar
Render the meeting string in the header bar, right-aligned or on a second line below the view name. Keep it visually distinct (dimmer color or italic style).

If `next_meeting` is `None` (no meeting or Outlook unavailable), the header renders exactly as before — no empty space.

## Risks / Trade-offs

- [Risk] AppleScript is slow (~500ms) → Mitigation: fetch is async, header shows without it until ready
- [Risk] Outlook not running → Mitigation: osascript returns error, stored as `None`, no display
- [Risk] Header becomes crowded with long meeting titles → Mitigation: truncate to ~40 chars
