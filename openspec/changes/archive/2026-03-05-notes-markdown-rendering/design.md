## Context

The note editor in `src/tui.rs` (`draw_note_editor`) renders each line as a plain `Paragraph` widget with no styling. Notes are stored as markdown files, but users see only raw syntax characters in the editor. The editor operates on a `Vec<String>` of lines with a cursor — styling must be applied at render time without altering the buffer.

## Goals / Non-Goals

**Goals:**
- Render markdown syntax with visual styling (colors, bold, italic) in the note editor
- Parse inline markdown elements per-line into styled ratatui `Span`s
- Handle block-level elements (headings, code blocks, blockquotes, lists) via line-prefix detection

**Non-Goals:**
- Full CommonMark/GFM compliance — this is visual hinting, not a spec-compliant renderer
- WYSIWYG editing (hiding syntax characters) — raw markdown remains visible
- Link rendering or clickable URLs
- Image rendering
- Adding external markdown parsing crates

## Decisions

### Line-by-line styling with a custom parser

Parse each line independently into a `Vec<Span>` using pattern matching. This avoids pulling in a full markdown AST library and keeps the implementation simple.

**Alternative considered**: Using `pulldown-cmark` for parsing. Rejected because it produces a block-level AST that doesn't map cleanly to the line-by-line rendering model of the editor. The overhead of a full parser isn't justified for visual hints.

### Block-level detection via line prefix

Detect block elements by inspecting the start of each line:
- `# `, `## `, `### ` → heading styles (bold + color, diminishing intensity)
- `` ``` `` → toggle code block state (track across lines)
- `> ` → blockquote style (italic + dimmed color)
- `- `, `* `, `1. ` → list marker styled differently from content

Code block state is the only cross-line state needed. Track it with a boolean passed through the line iteration.

### Inline element parsing via character scanning

Within a line (outside code blocks), scan for:
- `**text**` → bold
- `*text*` / `_text_` → italic
- `` `code` `` → inline code (colored background or distinct foreground)

Use a simple state-machine scanner that emits spans. Nesting (e.g., bold inside italic) is not supported — first match wins.

### Rendering integration

Replace the plain `Paragraph::new(display)` in `draw_note_editor` with `Paragraph::new(Line::from(spans))` where `spans` is the styled output from the parser. The line number column remains unstyled.

## Risks / Trade-offs

- [Imperfect parsing] Some edge cases (nested emphasis, escaped characters) won't render correctly → Acceptable for visual hints; users see raw markdown anyway
- [Performance on large notes] Per-line parsing on every frame → Negligible cost for typical note sizes; ratatui already rebuilds the widget tree each frame
- [Code block state across lines] Requires iterating all visible lines in order → Already the case in the current rendering loop
