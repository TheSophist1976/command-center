## 1. Markdown line parser

- [x] 1.1 Create `style_markdown_line` function that takes a line string and `in_code_block: bool`, returns `(Vec<Span>, bool)` with styled spans and updated code block state
- [x] 1.2 Implement heading detection: lines starting with `# `, `## `, `### ` styled as bold with appropriate colors
- [x] 1.3 Implement code block fence detection: lines matching `` ``` `` (with optional language tag) toggle code block state
- [x] 1.4 Implement code block line styling: lines inside code blocks styled with Green foreground, no inline parsing
- [x] 1.5 Implement blockquote detection: lines starting with `> ` styled as italic + dimmed
- [x] 1.6 Implement list marker detection: `- `, `* `, `N. ` prefixes styled with accent color, rest of line parsed for inline elements

## 2. Inline element parsing

- [x] 2.1 Implement inline scanner that processes a string and emits `Vec<Span>` for bold (`**`), italic (`*`/`_`), and inline code (`` ` ``) markers
- [x] 2.2 Handle unclosed markers gracefully (treat as plain text)
- [x] 2.3 Ensure inline parsing is skipped for headings, code blocks, and blockquotes (which get full-line styling)

## 3. Editor rendering integration

- [x] 3.1 Update `draw_note_editor` in `src/tui.rs` to track `in_code_block` state starting from the first line in the viewport (scanning from line 0 through viewport_offset)
- [x] 3.2 Replace `Paragraph::new(display)` with `Paragraph::new(Line::from(spans))` using output from `style_markdown_line`
- [x] 3.3 Ensure line truncation to `text_width` still works correctly with styled spans

## 4. Testing

- [x] 4.1 Add unit tests for heading, code block, blockquote, and list detection
- [x] 4.2 Add unit tests for inline bold, italic, and code parsing
- [x] 4.3 Add unit tests for unclosed markers and edge cases (empty lines, mixed syntax)
- [ ] 4.4 Manual verification: open a note with mixed markdown and confirm visual styling renders correctly
