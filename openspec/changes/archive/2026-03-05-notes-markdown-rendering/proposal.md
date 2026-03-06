## Why

The Notes editor currently displays raw markdown text with no visual distinction between headings, bold, code, lists, etc. Users write notes in markdown but cannot see how they look until they open the file externally. Rendering inline markdown styling in the editor improves readability and makes the note-taking experience significantly better.

## What Changes

- The note editor will render markdown syntax with visual styling (colors, bold, italic) instead of displaying raw text
- Supported markdown elements: headings (`#`), bold (`**`), italic (`*`), inline code (`` ` ``), code blocks (` ``` `), lists (`-`, `*`, numbered), blockquotes (`>`)
- Styling is applied at render time only — the underlying buffer remains raw markdown text
- No new dependencies; styling uses ratatui's existing `Span` and `Style` capabilities

## Capabilities

### New Capabilities
- `note-markdown-render`: Markdown-to-styled-spans rendering for the inline note editor

### Modified Capabilities
- `note-tui`: The note editor rendering changes from plain text to markdown-styled output

## Impact

- `src/tui.rs`: `draw_note_editor` function updated to apply markdown styling per line
- New module or function for parsing markdown lines into styled ratatui Spans
- No new crate dependencies (ratatui already supports styled text)
- No changes to note storage format — notes remain plain markdown files
