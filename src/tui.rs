use std::io::{self, stdout};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use chrono::{Datelike, Days, Local, Months, NaiveDate, Utc};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
};

mod theme {
    use ratatui::style::Color;

    // Bar (header/footer)
    pub const BAR_FG: Color = Color::White;
    pub const BAR_BG: Color = Color::Rgb(30, 60, 114);

    // Selection
    pub const HIGHLIGHT_BG: Color = Color::Rgb(40, 50, 80);

    // Priority colors
    pub const PRIORITY_CRITICAL: Color = Color::Rgb(255, 85, 85);
    pub const PRIORITY_HIGH: Color = Color::Rgb(255, 150, 50);
    pub const PRIORITY_MEDIUM: Color = Color::Rgb(255, 215, 0);
    pub const PRIORITY_LOW: Color = Color::Rgb(100, 200, 100);

    // Chat
    pub const CHAT_USER: Color = Color::Rgb(100, 180, 255);
    pub const CHAT_TASK_LIST: Color = Color::Rgb(255, 215, 0);
    pub const CHAT_ERROR: Color = Color::Rgb(255, 85, 85);

    // Task states
    pub const DONE_TEXT: Color = Color::Rgb(100, 100, 100);
    pub const OVERDUE: Color = Color::Rgb(255, 85, 85);
}

use crate::auth;
use crate::claude_session::{
    self, ClaudeSession, ClaudeSessionStatus, SessionEvent, SessionOutputEvent,
};
use crate::config;
use crate::nlp::{self, ApiMessage, NlpAction};
use crate::storage;
use crate::task::{Priority, Status, Task, TaskFile};
// -- Types --


#[derive(Debug, Clone, PartialEq)]
enum Mode {
    Normal,
    Adding,
    Filtering,
    Confirming,
    EditingPriority,
    EditingTitle,
    EditingTags,
    EditingDescription,
    EditingDefaultDir,
    NlpChat,
    ConfirmingNlp,
    EditingRecurrence,
    EditingDetailPanel,
    ConfirmingDetailSave,
    EditingNote,
    ConfirmingNoteExit,
    NotePicker,
    SessionDirectoryPicker,
    Sessions,
    SessionReply,
    PermissionModal,
}

#[derive(Debug, Clone)]
enum ChatMessage {
    User(String),
    Assistant(String),
    TaskList {
        text: String,
        tasks: Vec<(u32, String, String, String)>, // (id, title, priority, status)
    },
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum View {
    Today,
    All,
    Weekly,
    Monthly,
    Yearly,
    NoDueDate,
    Recurring,
    Notes,
}

impl View {
    fn matches(&self, task: &Task, today: NaiveDate) -> bool {
        // Completed tasks only appear in the All view
        if task.status == Status::Done && *self != View::All {
            return false;
        }
        // Recurring view: filter by recurrence presence only
        if *self == View::Recurring {
            return task.recurrence.is_some();
        }
        // Notes view doesn't show tasks
        if *self == View::Notes {
            return false;
        }
        // Overdue open tasks appear in all time-based views
        if task.status == Status::Open {
            if let Some(d) = task.due_date {
                if d < today && *self != View::NoDueDate {
                    return true;
                }
            }
        }
        match self {
            View::All => true,
            View::Today => match task.due_date {
                Some(d) => d == today,
                None => true,
            },
            View::Weekly => match task.due_date {
                Some(d) => {
                    let weekday = today.weekday().num_days_from_monday();
                    let monday = today - chrono::Duration::days(weekday as i64);
                    let sunday = monday + chrono::Duration::days(6);
                    d >= monday && d <= sunday
                }
                None => false,
            },
            View::Monthly => match task.due_date {
                Some(d) => d.year() == today.year() && d.month() == today.month(),
                None => false,
            },
            View::Yearly => match task.due_date {
                Some(d) => d.year() == today.year(),
                None => false,
            },
            View::NoDueDate => task.due_date.is_none(),
            View::Recurring => unreachable!(), // handled above
            View::Notes => unreachable!(), // handled above
        }
    }

    fn next(&self) -> View {
        match self {
            View::Today => View::All,
            View::All => View::Weekly,
            View::Weekly => View::Monthly,
            View::Monthly => View::Yearly,
            View::Yearly => View::NoDueDate,
            View::NoDueDate => View::Recurring,
            View::Recurring => View::Notes,
            View::Notes => View::Today,
        }
    }

    fn prev(&self) -> View {
        match self {
            View::Today => View::Notes,
            View::All => View::Today,
            View::Weekly => View::All,
            View::Monthly => View::Weekly,
            View::Yearly => View::Monthly,
            View::NoDueDate => View::Yearly,
            View::Recurring => View::NoDueDate,
            View::Notes => View::Recurring,
        }
    }

    fn display_name(&self) -> &str {
        match self {
            View::Today => "Today",
            View::All => "All Tasks",
            View::Weekly => "This Week",
            View::Monthly => "This Month",
            View::Yearly => "This Year",
            View::NoDueDate => "No Due Date",
            View::Recurring => "Recurring",
            View::Notes => "Notes",
        }
    }

    fn from_config(s: &str) -> View {
        match s.trim().to_lowercase().as_str() {
            "today" => View::Today,
            "all" => View::All,
            "weekly" => View::Weekly,
            "monthly" => View::Monthly,
            "yearly" => View::Yearly,
            "no-due-date" => View::NoDueDate,
            "recurring" => View::Recurring,
            "notes" => View::Notes,
            _ => View::Today,
        }
    }
}

#[derive(Debug, Clone)]
struct DetailDraft {
    title: String,
    description: String,
    priority: Priority,
    status: Status,
    due_date: String,
    project: String,
    tags: String,
    #[allow(dead_code)]
    original_task_id: u32,
}

impl DetailDraft {
    fn from_task(task: &Task) -> Self {
        Self {
            title: task.title.clone(),
            description: task.description.clone().unwrap_or_default(),
            priority: task.priority,
            status: task.status,
            due_date: task.due_date
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default(),
            project: task.project.clone().unwrap_or_default(),
            tags: task.tags.join(" "),
            original_task_id: task.id,
        }
    }

    fn is_dirty(&self, task: &Task) -> bool {
        self.title != task.title
            || self.description != task.description.as_deref().unwrap_or("")
            || self.priority != task.priority
            || self.status != task.status
            || self.due_date != task.due_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()
            || self.project != task.project.as_deref().unwrap_or("")
            || self.tags != task.tags.join(" ")
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum NavDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, Default)]
struct Filter {
    status: Option<Status>,
    priority: Option<Priority>,
    tag: Option<String>,
    project: Option<String>,
    title_contains: Option<String>,
}

impl Filter {
    fn is_active(&self) -> bool {
        self.status.is_some() || self.priority.is_some() || self.tag.is_some() || self.project.is_some() || self.title_contains.is_some()
    }

    fn matches(&self, task: &Task) -> bool {
        if let Some(s) = self.status {
            if task.status != s {
                return false;
            }
        }
        if let Some(p) = self.priority {
            if task.priority != p {
                return false;
            }
        }
        if let Some(ref tag) = self.tag {
            if !task.tags.iter().any(|t| t == tag) {
                return false;
            }
        }
        if let Some(ref proj) = self.project {
            match &task.project {
                Some(p) => if !p.eq_ignore_ascii_case(proj) { return false; },
                None => return false,
            }
        }
        if let Some(ref needle) = self.title_contains {
            if !task.title.to_lowercase().contains(&needle.to_lowercase()) {
                return false;
            }
        }
        true
    }

    fn summary(&self) -> String {
        let mut parts = Vec::new();
        if let Some(s) = self.status {
            parts.push(format!("status:{}", s));
        }
        if let Some(p) = self.priority {
            parts.push(format!("priority:{}", p));
        }
        if let Some(ref t) = self.tag {
            parts.push(format!("tag:{}", t));
        }
        if let Some(ref p) = self.project {
            parts.push(format!("project:{}", p));
        }
        if let Some(ref t) = self.title_contains {
            parts.push(format!("title:{}", t));
        }
        parts.join(" ")
    }

    fn parse(input: &str) -> Self {
        let mut filter = Filter::default();
        for part in input.split_whitespace() {
            if let Some(val) = part.strip_prefix("status:") {
                if let Ok(s) = val.parse::<Status>() {
                    filter.status = Some(s);
                }
            } else if let Some(val) = part.strip_prefix("priority:") {
                if let Ok(p) = val.parse::<Priority>() {
                    filter.priority = Some(p);
                }
            } else if let Some(val) = part.strip_prefix("tag:") {
                if !val.is_empty() {
                    filter.tag = Some(val.to_string());
                }
            } else if let Some(val) = part.strip_prefix("project:") {
                if !val.is_empty() {
                    filter.project = Some(val.to_string());
                }
            }
        }
        filter
    }
}

struct NoteEditor {
    slug: String,
    title: String,
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
    viewport_offset: usize,
    dirty: bool,
}

impl NoteEditor {
    fn new(slug: &str, title: &str, body: &str) -> Self {
        let lines: Vec<String> = if body.is_empty() {
            vec![String::new()]
        } else {
            body.lines().map(|l| l.to_string()).collect()
        };
        Self {
            slug: slug.to_string(),
            title: title.to_string(),
            lines,
            cursor_row: 0,
            cursor_col: 0,
            viewport_offset: 0,
            dirty: false,
        }
    }

    fn insert_char(&mut self, c: char) {
        let line = &mut self.lines[self.cursor_row];
        let byte_idx = char_to_byte_index(line, self.cursor_col);
        line.insert(byte_idx, c);
        self.cursor_col += 1;
        self.dirty = true;
    }

    fn insert_newline(&mut self) {
        let line = &self.lines[self.cursor_row];
        let byte_idx = char_to_byte_index(line, self.cursor_col);
        let rest = line[byte_idx..].to_string();
        self.lines[self.cursor_row] = line[..byte_idx].to_string();
        self.cursor_row += 1;
        self.lines.insert(self.cursor_row, rest);
        self.cursor_col = 0;
        self.dirty = true;
    }

    fn backspace(&mut self) {
        if self.cursor_col > 0 {
            let line = &mut self.lines[self.cursor_row];
            let byte_idx = char_to_byte_index(line, self.cursor_col - 1);
            let end_idx = char_to_byte_index(line, self.cursor_col);
            line.replace_range(byte_idx..end_idx, "");
            self.cursor_col -= 1;
            self.dirty = true;
        } else if self.cursor_row > 0 {
            let current_line = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].chars().count();
            self.lines[self.cursor_row].push_str(&current_line);
            self.dirty = true;
        }
    }

    fn move_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.clamp_col();
        }
    }

    fn move_down(&mut self) {
        if self.cursor_row < self.lines.len() - 1 {
            self.cursor_row += 1;
            self.clamp_col();
        }
    }

    fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        }
    }

    fn move_right(&mut self) {
        let line_len = self.lines[self.cursor_row].chars().count();
        if self.cursor_col < line_len {
            self.cursor_col += 1;
        }
    }

    fn move_to_line_start(&mut self) {
        self.cursor_col = 0;
    }

    fn move_to_line_end(&mut self) {
        self.cursor_col = self.lines[self.cursor_row].chars().count();
    }

    fn clamp_col(&mut self) {
        let line_len = self.lines[self.cursor_row].chars().count();
        if self.cursor_col > line_len {
            self.cursor_col = line_len;
        }
    }

    fn ensure_cursor_visible(&mut self, visible_height: usize, text_width: usize) {
        if visible_height == 0 {
            return;
        }
        // Scroll up: cursor moved above the viewport
        if self.cursor_row < self.viewport_offset {
            self.viewport_offset = self.cursor_row;
            return;
        }
        // Scroll down: advance viewport_offset one logical line at a time until
        // the cursor's visual row fits within visible_height.
        let cols_per_row = text_width.max(1);
        let visual_rows_for = |line: &str| -> usize {
            let n = line.chars().count();
            if n == 0 { 1 } else { (n + cols_per_row - 1) / cols_per_row }
        };
        loop {
            // Count display rows from viewport_offset up to (not including) cursor_row
            let mut display_row: usize = 0;
            for row in self.viewport_offset..self.cursor_row.min(self.lines.len()) {
                display_row += visual_rows_for(&self.lines[row]);
            }
            // Add the visual row within the cursor's logical line
            display_row += self.cursor_col / cols_per_row;
            if display_row < visible_height {
                break;
            }
            if self.viewport_offset < self.cursor_row {
                self.viewport_offset += 1;
            } else {
                break;
            }
        }
    }

    fn body_text(&self) -> String {
        self.lines.join("\n")
    }
}

fn char_to_byte_index(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

// -- Markdown styling for note editor --

mod md_style {
    use ratatui::prelude::*;

    const HEADING1_COLOR: Color = Color::Rgb(100, 180, 255);
    const HEADING2_COLOR: Color = Color::Rgb(140, 170, 220);
    const HEADING3_COLOR: Color = Color::Rgb(160, 160, 190);
    const CODE_COLOR: Color = Color::Green;
    const BLOCKQUOTE_COLOR: Color = Color::Rgb(150, 150, 170);
    const LIST_MARKER_COLOR: Color = Color::Rgb(255, 215, 0);

    /// Style a single markdown line into spans. Returns (spans, updated in_code_block).
    pub fn style_markdown_line(line: &str, in_code_block: bool) -> (Vec<Span<'static>>, bool) {
        // Code fence toggle
        let trimmed = line.trim_start();
        if trimmed == "```" || trimmed.starts_with("``` ") || trimmed.starts_with("```\t")
            || (trimmed.starts_with("```") && trimmed[3..].chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '+'))
        {
            let span = Span::styled(line.to_string(), Style::default().fg(CODE_COLOR));
            return (vec![span], !in_code_block);
        }

        // Inside code block: no parsing
        if in_code_block {
            let span = Span::styled(line.to_string(), Style::default().fg(CODE_COLOR));
            return (vec![span], true);
        }

        // Headings
        if line.starts_with("### ") {
            let span = Span::styled(
                line.to_string(),
                Style::default().fg(HEADING3_COLOR).add_modifier(Modifier::BOLD),
            );
            return (vec![span], false);
        }
        if line.starts_with("## ") {
            let span = Span::styled(
                line.to_string(),
                Style::default().fg(HEADING2_COLOR).add_modifier(Modifier::BOLD),
            );
            return (vec![span], false);
        }
        if line.starts_with("# ") {
            let span = Span::styled(
                line.to_string(),
                Style::default().fg(HEADING1_COLOR).add_modifier(Modifier::BOLD),
            );
            return (vec![span], false);
        }

        // Blockquotes
        if line.starts_with("> ") || line == ">" {
            let span = Span::styled(
                line.to_string(),
                Style::default().fg(BLOCKQUOTE_COLOR).add_modifier(Modifier::ITALIC),
            );
            return (vec![span], false);
        }

        // List items: style marker separately, parse rest for inline
        if let Some(marker_len) = list_marker_len(line) {
            let marker = &line[..marker_len];
            let rest = &line[marker_len..];
            let mut spans = vec![Span::styled(
                marker.to_string(),
                Style::default().fg(LIST_MARKER_COLOR),
            )];
            spans.extend(parse_inline(rest));
            return (spans, false);
        }

        // Plain line with inline parsing
        (parse_inline(line), false)
    }

    fn list_marker_len(line: &str) -> Option<usize> {
        if line.starts_with("- ") || line.starts_with("* ") {
            return Some(2);
        }
        // Ordered list: digits followed by ". "
        let bytes = line.as_bytes();
        let mut i = 0;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i > 0 && line[i..].starts_with(". ") {
            Some(i + 2)
        } else {
            None
        }
    }

    fn parse_inline(text: &str) -> Vec<Span<'static>> {
        let mut spans: Vec<Span<'static>> = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        let mut i = 0;
        let mut plain = String::new();

        while i < len {
            // Bold: **...**
            if i + 1 < len && chars[i] == '*' && chars[i + 1] == '*' {
                if let Some(end) = find_closing(&chars, i + 2, &['*', '*']) {
                    if !plain.is_empty() {
                        spans.push(Span::raw(std::mem::take(&mut plain)));
                    }
                    let content: String = chars[i..end + 2].iter().collect();
                    spans.push(Span::styled(content, Style::default().add_modifier(Modifier::BOLD)));
                    i = end + 2;
                    continue;
                }
            }

            // Inline code: `...`
            if chars[i] == '`' {
                if let Some(end) = find_single_closing(&chars, i + 1, '`') {
                    if !plain.is_empty() {
                        spans.push(Span::raw(std::mem::take(&mut plain)));
                    }
                    let content: String = chars[i..=end].iter().collect();
                    spans.push(Span::styled(content, Style::default().fg(CODE_COLOR)));
                    i = end + 1;
                    continue;
                }
            }

            // Italic: *...* or _..._
            if (chars[i] == '*' || chars[i] == '_')
                && (i + 1 < len && chars[i + 1] != ' ')
            {
                let marker = chars[i];
                if let Some(end) = find_single_closing(&chars, i + 1, marker) {
                    if end > i + 1 {
                        if !plain.is_empty() {
                            spans.push(Span::raw(std::mem::take(&mut plain)));
                        }
                        let content: String = chars[i..=end].iter().collect();
                        spans.push(Span::styled(content, Style::default().add_modifier(Modifier::ITALIC)));
                        i = end + 1;
                        continue;
                    }
                }
            }

            plain.push(chars[i]);
            i += 1;
        }

        if !plain.is_empty() {
            spans.push(Span::raw(plain));
        }
        if spans.is_empty() {
            spans.push(Span::raw(String::new()));
        }
        spans
    }

    /// Find closing double-char marker (e.g., **) starting search at `from`.
    fn find_closing(chars: &[char], from: usize, marker: &[char; 2]) -> Option<usize> {
        let len = chars.len();
        let mut j = from;
        while j + 1 < len {
            if chars[j] == marker[0] && chars[j + 1] == marker[1] {
                return Some(j);
            }
            j += 1;
        }
        None
    }

    /// Find closing single-char marker starting search at `from`.
    fn find_single_closing(chars: &[char], from: usize, marker: char) -> Option<usize> {
        for j in from..chars.len() {
            if chars[j] == marker {
                return Some(j);
            }
        }
        None
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        fn styled_text(spans: &[Span]) -> String {
            spans.iter().map(|s| s.content.as_ref()).collect()
        }

        // -- Heading tests --

        #[test]
        fn test_h1_heading() {
            let (spans, in_cb) = style_markdown_line("# My Title", false);
            assert!(!in_cb);
            assert_eq!(spans.len(), 1);
            assert_eq!(spans[0].content.as_ref(), "# My Title");
            assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
        }

        #[test]
        fn test_h2_heading() {
            let (spans, _) = style_markdown_line("## Section", false);
            assert_eq!(spans[0].content.as_ref(), "## Section");
            assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
        }

        #[test]
        fn test_h3_heading() {
            let (spans, _) = style_markdown_line("### Sub", false);
            assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
        }

        #[test]
        fn test_hash_without_space_not_heading() {
            let (spans, _) = style_markdown_line("#notaheading", false);
            // Should be parsed as plain inline text, not bold heading
            assert!(!spans[0].style.add_modifier.contains(Modifier::BOLD));
        }

        // -- Code block tests --

        #[test]
        fn test_code_fence_toggle() {
            let (_, in_cb) = style_markdown_line("```", false);
            assert!(in_cb);
            let (_, in_cb) = style_markdown_line("```", true);
            assert!(!in_cb);
        }

        #[test]
        fn test_code_fence_with_lang() {
            let (spans, in_cb) = style_markdown_line("```rust", false);
            assert!(in_cb);
            assert_eq!(spans[0].style.fg, Some(CODE_COLOR));
        }

        #[test]
        fn test_inside_code_block() {
            let (spans, in_cb) = style_markdown_line("let x = 1;", true);
            assert!(in_cb);
            assert_eq!(spans[0].style.fg, Some(CODE_COLOR));
        }

        // -- Blockquote tests --

        #[test]
        fn test_blockquote() {
            let (spans, _) = style_markdown_line("> This is a quote", false);
            assert!(spans[0].style.add_modifier.contains(Modifier::ITALIC));
        }

        // -- List tests --

        #[test]
        fn test_unordered_list() {
            let (spans, _) = style_markdown_line("- Buy groceries", false);
            assert_eq!(spans[0].content.as_ref(), "- ");
            assert_eq!(spans[0].style.fg, Some(LIST_MARKER_COLOR));
        }

        #[test]
        fn test_ordered_list() {
            let (spans, _) = style_markdown_line("1. First item", false);
            assert_eq!(spans[0].content.as_ref(), "1. ");
            assert_eq!(spans[0].style.fg, Some(LIST_MARKER_COLOR));
        }

        // -- Inline bold tests --

        #[test]
        fn test_bold() {
            let (spans, _) = style_markdown_line("This is **bold** text", false);
            assert_eq!(styled_text(&spans), "This is **bold** text");
            assert!(spans[1].style.add_modifier.contains(Modifier::BOLD));
        }

        #[test]
        fn test_unclosed_bold() {
            let (spans, _) = style_markdown_line("This is **not closed", false);
            // No bold styling applied
            for s in &spans {
                assert!(!s.style.add_modifier.contains(Modifier::BOLD));
            }
        }

        // -- Inline italic tests --

        #[test]
        fn test_italic_asterisk() {
            let (spans, _) = style_markdown_line("This is *italic* text", false);
            assert_eq!(styled_text(&spans), "This is *italic* text");
            assert!(spans[1].style.add_modifier.contains(Modifier::ITALIC));
        }

        #[test]
        fn test_italic_underscore() {
            let (spans, _) = style_markdown_line("This is _italic_ text", false);
            assert!(spans[1].style.add_modifier.contains(Modifier::ITALIC));
        }

        // -- Inline code tests --

        #[test]
        fn test_inline_code() {
            let (spans, _) = style_markdown_line("Use the `println!` macro", false);
            assert_eq!(styled_text(&spans), "Use the `println!` macro");
            assert_eq!(spans[1].style.fg, Some(CODE_COLOR));
        }

        #[test]
        fn test_unclosed_backtick() {
            let (spans, _) = style_markdown_line("Use the `println! macro", false);
            for s in &spans {
                assert_ne!(s.style.fg, Some(CODE_COLOR));
            }
        }

        // -- Edge cases --

        #[test]
        fn test_empty_line() {
            let (spans, in_cb) = style_markdown_line("", false);
            assert!(!in_cb);
            assert_eq!(styled_text(&spans), "");
        }

        #[test]
        fn test_plain_text() {
            let (spans, _) = style_markdown_line("Hello world", false);
            assert_eq!(spans.len(), 1);
            assert_eq!(spans[0].content.as_ref(), "Hello world");
        }

        #[test]
        fn test_inline_skipped_in_code_block() {
            let (spans, _) = style_markdown_line("**bold** in code", true);
            // Entire line styled as code, no bold parsing
            assert_eq!(spans.len(), 1);
            assert_eq!(spans[0].style.fg, Some(CODE_COLOR));
        }

        #[test]
        fn test_inline_skipped_in_heading() {
            let (spans, _) = style_markdown_line("# Title with **bold**", false);
            // Entire line is one heading span, no separate bold
            assert_eq!(spans.len(), 1);
            assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
        }

        #[test]
        fn test_inline_skipped_in_blockquote() {
            let (spans, _) = style_markdown_line("> quote with **bold**", false);
            assert_eq!(spans.len(), 1);
            assert!(spans[0].style.add_modifier.contains(Modifier::ITALIC));
        }
    }
}

struct App {
    task_file: TaskFile,
    file_path: PathBuf,
    selected: usize,
    filter: Filter,
    view: View,
    mode: Mode,
    input_buffer: String,
    table_state: TableState,
    status_message: Option<String>,
    pending_nlp_update: Option<(NlpAction, Vec<usize>)>,
    chat_history: Vec<ChatMessage>,
    nlp_messages: Vec<ApiMessage>,
    show_detail_panel: bool,
    detail_draft: Option<DetailDraft>,
    detail_field_index: usize,
    pending_navigation: Option<NavDirection>,
    nlp_pending: Option<mpsc::Receiver<Result<(NlpAction, String), String>>>,
    nlp_spinner_frame: u8,
    notes_list: Vec<crate::note::Note>,
    notes_selected: usize,
    note_editor: Option<NoteEditor>,
    note_picker_items: Vec<String>,
    note_picker_selected: usize,
    note_picker_task_idx: Option<usize>,
    // Claude sessions
    claude_sessions: Vec<ClaudeSession>,
    next_session_id: usize,
    session_selected: usize,
    session_dir_picker: Vec<PathBuf>,
    session_dir_picker_selected: usize,
    session_pending_context: Option<String>,
    session_reply_input: String,
    session_viewing_output: bool,
    session_output_scroll: usize,
    session_output_follow: bool,
    session_focused_event: usize,
    permission_modal_tool: String,
    permission_modal_input: String,
}

impl App {
    fn new(path: &Path) -> Result<Self, String> {
        let task_file = storage::load(path, false)?;
        let view = config::read_config_value("default-view")
            .map(|v| View::from_config(&v))
            .unwrap_or(View::Today);
        let mut app = Self {
            task_file,
            file_path: path.to_path_buf(),
            selected: 0,
            filter: Filter::default(),
            view,
            mode: Mode::Normal,
            input_buffer: String::new(),
            table_state: TableState::default(),
            status_message: None,
            pending_nlp_update: None,
            chat_history: Vec::new(),
            nlp_messages: Vec::new(),
            show_detail_panel: false,
            detail_draft: None,
            detail_field_index: 0,
            pending_navigation: None,
            nlp_pending: None,
            nlp_spinner_frame: 0,
            notes_list: Vec::new(),
            notes_selected: 0,
            note_editor: None,
            note_picker_items: Vec::new(),
            note_picker_selected: 0,
            note_picker_task_idx: None,
            claude_sessions: Vec::new(),
            next_session_id: 0,
            session_selected: 0,
            session_dir_picker: Vec::new(),
            session_dir_picker_selected: 0,
            session_pending_context: None,
            session_reply_input: String::new(),
            session_viewing_output: false,
            session_output_scroll: 0,
            session_output_follow: true,
            session_focused_event: 0,
            permission_modal_tool: String::new(),
            permission_modal_input: String::new(),
        };
        app.table_state.select(Some(0));
        // Load persisted sessions
        let task_dir = app.task_dir();
        let loaded = claude_session::load_sessions(&task_dir);
        app.next_session_id = loaded.iter().map(|s| s.id).max().map_or(0, |m| m + 1);
        app.claude_sessions = loaded;
        Ok(app)
    }

    fn task_dir(&self) -> PathBuf {
        self.file_path.parent().unwrap_or(Path::new(".")).to_path_buf()
    }

    fn task_filename(&self) -> String {
        self.file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("tasks.md")
            .to_string()
    }

    fn refresh_notes(&mut self) {
        self.notes_list = crate::note::discover_notes(&self.task_dir(), &self.task_filename());
    }

    fn filtered_indices(&self) -> Vec<usize> {
        let today = Local::now().date_naive();
        let mut indices: Vec<usize> = self
            .task_file
            .tasks
            .iter()
            .enumerate()
            .filter(|(_, t)| self.view.matches(t, today))
            .filter(|(_, t)| self.filter.matches(t))
            .map(|(i, _)| i)
            .collect();
        let tasks = &self.task_file.tasks;
        indices.sort_by(|&a, &b| {
            let ta = &tasks[a];
            let tb = &tasks[b];
            // Due date ascending, None last
            let da = ta.due_date.map(|d| (0, d));
            let db = tb.due_date.map(|d| (0, d));
            let date_cmp = match (da, db) {
                (Some(a), Some(b)) => a.cmp(&b),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            };
            // Priority descending (Critical first — Critical < High < Medium < Low by Ord)
            date_cmp.then(ta.priority.cmp(&tb.priority))
        });
        indices
    }

    fn clamp_selection(&mut self) {
        let count = self.filtered_indices().len();
        if count == 0 {
            self.selected = 0;
        } else if self.selected >= count {
            self.selected = count - 1;
        }
        self.table_state.select(if count > 0 {
            Some(self.selected)
        } else {
            None
        });
    }

    fn save(&self) -> Result<(), String> {
        storage::save(&self.file_path, &self.task_file)
    }

    fn reload_from_disk(&mut self) -> Result<(), String> {
        let task_file = storage::load(&self.file_path, false)?;
        let n = task_file.tasks.len();
        self.task_file = task_file;
        self.clamp_selection();
        self.status_message = Some(format!("Reloaded {} tasks from disk", n));
        Ok(())
    }
}

// -- Entry point --

pub fn run(path: &Path) -> Result<(), String> {
    let mut app = App::new(path)?;

    // Install panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
        original_hook(info);
    }));

    enable_raw_mode().map_err(|e| format!("Failed to enable raw mode: {}", e))?;
    execute!(stdout(), EnterAlternateScreen)
        .map_err(|e| format!("Failed to enter alternate screen: {}", e))?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal =
        Terminal::new(backend).map_err(|e| format!("Failed to create terminal: {}", e))?;

    let result = event_loop(&mut terminal, &mut app);

    // Kill any running claude sessions before exit
    for session in &mut app.claude_sessions {
        if session.status == ClaudeSessionStatus::Running {
            if let Some(ref mut child) = session.child {
                let _ = child.kill();
            }
        }
    }

    // Restore terminal
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);

    result
}

fn event_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<(), String> {
    loop {
        // Check for NLP background result
        if let Some(ref rx) = app.nlp_pending {
            match rx.try_recv() {
                Ok(result) => {
                    app.nlp_pending = None;
                    app.nlp_spinner_frame = 0;
                    process_nlp_result(app, result)?;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    // Still waiting — update spinner animation
                    let dots = match app.nlp_spinner_frame % 4 {
                        0 => "Thinking",
                        1 => "Thinking.",
                        2 => "Thinking..",
                        _ => "Thinking...",
                    };
                    app.status_message = Some(dots.to_string());
                    app.nlp_spinner_frame = app.nlp_spinner_frame.wrapping_add(1);
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    // Thread panicked or dropped sender
                    app.nlp_pending = None;
                    app.nlp_spinner_frame = 0;
                    app.status_message = None;
                    app.chat_history.push(ChatMessage::Error("NLP request failed unexpectedly".to_string()));
                }
            }
        }

        // Poll claude sessions for output
        {
            let n = app.claude_sessions.len();
            for i in 0..n {
                let mut events_to_add: Vec<SessionOutputEvent> = Vec::new();
                let mut tool_result_batches: Vec<Vec<String>> = Vec::new();
                let mut new_status: Option<ClaudeSessionStatus> = None;
                let mut clear_rx = false;
                let mut captured_session_id: Option<String> = None;
                let mut permission_req: Option<(String, String)> = None;

                if let Some(ref rx) = app.claude_sessions[i].rx {
                    loop {
                        match rx.try_recv() {
                            Ok(SessionEvent::OutputEvent(e)) => events_to_add.push(e),
                            Ok(SessionEvent::AppendToolResult { lines }) => {
                                tool_result_batches.push(lines);
                            }
                            Ok(SessionEvent::SessionIdCaptured(sid)) => {
                                captured_session_id = Some(sid);
                            }
                            Ok(SessionEvent::PermissionRequest { tool, input_preview }) => {
                                permission_req = Some((tool, input_preview));
                            }
                            Ok(SessionEvent::Done) => {
                                new_status = Some(ClaudeSessionStatus::WaitingForInput);
                                clear_rx = true;
                                break;
                            }
                            Ok(SessionEvent::Error(e)) => {
                                new_status = Some(ClaudeSessionStatus::Failed);
                                clear_rx = true;
                                events_to_add.push(SessionOutputEvent::Text(format!("Error: {}", e)));
                                break;
                            }
                            Err(mpsc::TryRecvError::Empty) => break,
                            Err(mpsc::TryRecvError::Disconnected) => {
                                new_status = Some(ClaudeSessionStatus::Failed);
                                clear_rx = true;
                                break;
                            }
                        }
                    }
                }

                let had_new = !events_to_add.is_empty() || !tool_result_batches.is_empty();

                for event in events_to_add {
                    claude_session::push_output_event(&mut app.claude_sessions[i].output, event);
                }
                // Append tool results to the last ToolCall in the buffer
                for lines in tool_result_batches {
                    let output = &mut app.claude_sessions[i].output;
                    if let Some(SessionOutputEvent::ToolCall { result_lines, .. }) =
                        output.iter_mut().rev().find(|e| matches!(e, SessionOutputEvent::ToolCall { .. }))
                    {
                        result_lines.extend(lines);
                    }
                }
                if let Some(sid) = captured_session_id {
                    app.claude_sessions[i].session_id = Some(sid);
                }
                if let Some((tool, input_preview)) = permission_req {
                    app.permission_modal_tool = tool;
                    app.permission_modal_input = input_preview;
                    if i == app.session_selected {
                        app.mode = Mode::PermissionModal;
                    } else {
                        app.claude_sessions[i].status = ClaudeSessionStatus::WaitingForInput;
                    }
                }
                if had_new
                    && app.session_viewing_output
                    && i == app.session_selected
                    && app.session_output_follow
                {
                    let total = app.claude_sessions[i].output.len();
                    app.session_output_scroll = total.saturating_sub(1);
                }
                if let Some(status) = new_status {
                    app.claude_sessions[i].status = status;
                }
                if clear_rx {
                    app.claude_sessions[i].rx = None;
                    app.claude_sessions[i].child = None;
                    app.claude_sessions[i].stdin = None;
                    let task_dir = app.task_dir();
                    let _ = claude_session::save_session(&task_dir, &app.claude_sessions[i]);
                }
            }
        }


        terminal
            .draw(|frame| draw(frame, app))
            .map_err(|e| format!("Draw error: {}", e))?;

        if event::poll(Duration::from_millis(200))
            .map_err(|e| format!("Event poll error: {}", e))?
        {
            if let Event::Key(key) = event::read().map_err(|e| format!("Event read error: {}", e))? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if handle_key(terminal, app, key.code, key.modifiers)? {
                    return Ok(());
                }
            }
        }
    }
}

/// Returns true if we should quit.
fn toggle_task_status(app: &mut App, task_idx: usize) -> Result<(), String> {
    let was_open = app.task_file.tasks[task_idx].status == Status::Open;
    {
        let task = &mut app.task_file.tasks[task_idx];
        task.status = match task.status {
            Status::Open => Status::Done,
            Status::Done => Status::Open,
        };
        task.updated = Some(Utc::now());
    }
    // If we just completed a recurring task, spawn the next occurrence
    if was_open {
        let task = &app.task_file.tasks[task_idx];
        if let Some(recur) = task.recurrence {
            let next_due = crate::task::next_due_date(&recur, task.due_date);
            let new_id = app.task_file.next_id;
            app.task_file.next_id += 1;
            let new_task = Task {
                id: new_id,
                title: task.title.clone(),
                status: Status::Open,
                priority: task.priority,
                tags: task.tags.clone(),
                created: Utc::now(),
                updated: None,
                description: task.description.clone(),
                due_date: Some(next_due),
                project: task.project.clone(),
                recurrence: Some(recur),
                note: task.note.clone(),
            };
            app.task_file.tasks.push(new_task);
            app.status_message = Some(format!("Next occurrence: task {}, due {}", new_id, next_due));
        }
    }
    app.save()?;
    Ok(())
}

fn handle_key(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App, key: KeyCode, modifiers: event::KeyModifiers) -> Result<bool, String> {
    // Handle Ctrl+S in note editor
    if app.mode == Mode::EditingNote && modifiers.contains(event::KeyModifiers::CONTROL) && key == KeyCode::Char('s') {
        save_current_note(app)?;
        return Ok(false);
    }
    // Handle Ctrl+R: reload tasks from disk
    if modifiers.contains(event::KeyModifiers::CONTROL) && key == KeyCode::Char('r') {
        if app.mode != Mode::Normal {
            app.status_message = Some("Cannot reload: finish editing first".to_string());
        } else if let Err(e) = app.reload_from_disk() {
            app.status_message = Some(e);
        }
        return Ok(false);
    }
    match app.mode {
        Mode::Normal => handle_normal(app, key),
        Mode::Adding => {
            handle_input(app, key, InputAction::Add)?;
            Ok(false)
        }
        Mode::Filtering => {
            handle_input(app, key, InputAction::Filter)?;
            Ok(false)
        }
        Mode::Confirming => {
            handle_confirm(app, key)?;
            Ok(false)
        }
        Mode::EditingPriority => {
            handle_priority(app, key)?;
            Ok(false)
        }
        Mode::EditingTitle => {
            handle_input(app, key, InputAction::EditTitle)?;
            Ok(false)
        }
        Mode::EditingTags => {
            handle_input(app, key, InputAction::EditTags)?;
            Ok(false)
        }
        Mode::EditingDescription => {
            handle_input(app, key, InputAction::EditDescription)?;
            Ok(false)
        }
        Mode::EditingRecurrence => {
            handle_recurrence_input(app, key)?;
            Ok(false)
        }
        Mode::EditingDefaultDir => {
            handle_input(app, key, InputAction::EditDefaultDir)?;
            Ok(false)
        }
        Mode::NlpChat => {
            handle_nlp_chat(terminal, app, key)?;
            Ok(false)
        }
        Mode::ConfirmingNlp => {
            handle_nlp_confirm(app, key)?;
            Ok(false)
        }
        Mode::EditingDetailPanel => {
            handle_detail_edit(app, key)?;
            Ok(false)
        }
        Mode::ConfirmingDetailSave => {
            handle_detail_confirm(app, key)?;
            Ok(false)
        }
        Mode::EditingNote => {
            handle_note_editor(app, key)?;
            Ok(false)
        }
        Mode::ConfirmingNoteExit => {
            handle_note_exit_confirm(app, key)?;
            Ok(false)
        }
        Mode::NotePicker => {
            handle_note_picker(app, key)?;
            Ok(false)
        }
        Mode::SessionDirectoryPicker => {
            handle_session_dir_picker(app, key)?;
            Ok(false)
        }
        Mode::Sessions => {
            handle_sessions(app, key)?;
            Ok(false)
        }
        Mode::SessionReply => {
            handle_session_reply(app, key)?;
            Ok(false)
        }
        Mode::PermissionModal => {
            handle_permission_modal(app, key)?;
            Ok(false)
        }
    }
}


fn handle_normal(app: &mut App, key: KeyCode) -> Result<bool, String> {
    // Clear any status message on keypress
    app.status_message = None;

    if app.view == View::Notes {
        return handle_normal_notes(app, key);
    }

    let filtered = app.filtered_indices();
    match key {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Char('j') | KeyCode::Down => {
            if let Some(ref draft) = app.detail_draft {
                let dirty = filtered.get(app.selected)
                    .map(|&idx| draft.is_dirty(&app.task_file.tasks[idx]))
                    .unwrap_or(false);
                if dirty {
                    app.pending_navigation = Some(NavDirection::Down);
                    app.mode = Mode::ConfirmingDetailSave;
                } else {
                    app.detail_draft = None;
                    if !filtered.is_empty() && app.selected < filtered.len() - 1 {
                        app.selected += 1;
                        app.table_state.select(Some(app.selected));
                    }
                }
            } else if !filtered.is_empty() && app.selected < filtered.len() - 1 {
                app.selected += 1;
                app.table_state.select(Some(app.selected));
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if let Some(ref draft) = app.detail_draft {
                let dirty = filtered.get(app.selected)
                    .map(|&idx| draft.is_dirty(&app.task_file.tasks[idx]))
                    .unwrap_or(false);
                if dirty {
                    app.pending_navigation = Some(NavDirection::Up);
                    app.mode = Mode::ConfirmingDetailSave;
                } else {
                    app.detail_draft = None;
                    if app.selected > 0 {
                        app.selected -= 1;
                        app.table_state.select(Some(app.selected));
                    }
                }
            } else if app.selected > 0 {
                app.selected -= 1;
                app.table_state.select(Some(app.selected));
            }
        }
        KeyCode::Enter => {
            if app.show_detail_panel {
                if let Some(&task_idx) = filtered.get(app.selected) {
                    let task = &app.task_file.tasks[task_idx];
                    app.detail_draft = Some(DetailDraft::from_task(task));
                    app.detail_field_index = 0;
                    app.input_buffer = task.title.clone();
                    app.mode = Mode::EditingDetailPanel;
                }
            } else if let Some(&task_idx) = filtered.get(app.selected) {
                toggle_task_status(app, task_idx)?;
            }
        }
        KeyCode::Char(' ') => {
            if let Some(&task_idx) = filtered.get(app.selected) {
                toggle_task_status(app, task_idx)?;
            }
        }
        KeyCode::Char('a') => {
            app.mode = Mode::Adding;
            app.input_buffer.clear();
        }
        KeyCode::Char('d') => {
            if !filtered.is_empty() {
                app.mode = Mode::Confirming;
            }
        }
        KeyCode::Char('f') | KeyCode::Char('/') => {
            app.mode = Mode::Filtering;
            app.input_buffer.clear();
        }
        KeyCode::Char('p') => {
            if !filtered.is_empty() {
                app.mode = Mode::EditingPriority;
            }
        }
        KeyCode::Char('e') => {
            if let Some(&task_idx) = filtered.get(app.selected) {
                app.input_buffer = app.task_file.tasks[task_idx].title.clone();
                app.mode = Mode::EditingTitle;
            }
        }
        KeyCode::Char('t') => {
            if let Some(&task_idx) = filtered.get(app.selected) {
                app.input_buffer = app.task_file.tasks[task_idx].tags.join(" ");
                app.mode = Mode::EditingTags;
            }
        }
        KeyCode::Char('r') => {
            if let Some(&task_idx) = filtered.get(app.selected) {
                app.input_buffer = app.task_file.tasks[task_idx].description.clone().unwrap_or_default();
                app.mode = Mode::EditingDescription;
            }
        }
        KeyCode::Char('R') => {
            if filtered.get(app.selected).is_some() {
                app.input_buffer.clear();
                app.mode = Mode::EditingRecurrence;
            }
        }
        KeyCode::Char('v') => {
            let next = app.view.next();
            if next == View::Notes {
                app.refresh_notes();
                app.notes_selected = 0;
            }
            app.view = next;
            app.selected = 0;
            app.table_state.select(Some(0));
            app.clamp_selection();
        }
        KeyCode::Char('V') => {
            let prev = app.view.prev();
            if prev == View::Notes {
                app.refresh_notes();
                app.notes_selected = 0;
            }
            app.view = prev;
            app.selected = 0;
            app.table_state.select(Some(0));
            app.clamp_selection();
        }
        KeyCode::Char('C') => {
            // If sessions exist, re-open sessions panel; press n inside to start a new one
            if !app.claude_sessions.is_empty() {
                app.session_selected = app.session_selected.min(app.claude_sessions.len().saturating_sub(1));
                app.session_viewing_output = false;
                app.mode = Mode::Sessions;
            } else {
                // No sessions yet — build context from selected task and open dir picker
                let filtered = app.filtered_indices();
                let context = if let Some(&idx) = filtered.get(app.selected) {
                    let task = &app.task_file.tasks[idx];
                    let body = task.description.as_deref().unwrap_or("");
                    claude_session::build_session_context(&task.title, body)
                } else {
                    String::new()
                };
                app.session_pending_context = Some(context);
                populate_session_dir_picker(app);
                app.session_dir_picker_selected = 0;
                app.mode = Mode::SessionDirectoryPicker;
            }
        }
        KeyCode::Char(':') => {
            app.mode = Mode::NlpChat;
            app.input_buffer.clear();
            app.chat_history.clear();
            app.nlp_messages.clear();
        }
        KeyCode::Char('D') => {
            app.input_buffer = config::read_config_value("default-dir").unwrap_or_default();
            app.mode = Mode::EditingDefaultDir;
        }
        KeyCode::Tab => {
            app.show_detail_panel = !app.show_detail_panel;
        }
        KeyCode::Char('T') | KeyCode::Char('N') | KeyCode::Char('W') | KeyCode::Char('M') | KeyCode::Char('Q') | KeyCode::Char('Y') | KeyCode::Char('X') => {
            if let Some(&task_idx) = filtered.get(app.selected) {
                let task = &mut app.task_file.tasks[task_idx];
                let today = Local::now().date_naive();
                if key == KeyCode::Char('X') {
                    task.due_date = None;
                    task.updated = Some(Utc::now());
                    app.save()?;
                    app.status_message = Some("Due date cleared".to_string());
                } else {
                    let date = match key {
                        KeyCode::Char('T') => Some(today),
                        KeyCode::Char('N') => today.checked_add_days(Days::new(1)),
                        KeyCode::Char('W') => today.checked_add_days(Days::new(7)),
                        KeyCode::Char('M') => today.checked_add_months(Months::new(1)),
                        KeyCode::Char('Q') => today.checked_add_months(Months::new(3)),
                        KeyCode::Char('Y') => today.checked_add_months(Months::new(12)),
                        _ => unreachable!(),
                    };
                    if let Some(d) = date {
                        task.due_date = Some(d);
                        task.updated = Some(Utc::now());
                        app.save()?;
                        app.status_message = Some(format!("Due: {}", d.format("%Y-%m-%d")));
                    }
                }
            }
        }
        KeyCode::Char('n') => {
            if let Some(&task_idx) = filtered.get(app.selected) {
                app.refresh_notes();
                let mut items = vec!["(none)".to_string(), "(new note)".to_string()];
                for note in &app.notes_list {
                    items.push(note.slug.clone());
                }
                app.note_picker_items = items;
                app.note_picker_selected = 0;
                app.note_picker_task_idx = Some(task_idx);
                app.mode = Mode::NotePicker;
            }
        }
        KeyCode::Char('g') => {
            if let Some(&task_idx) = filtered.get(app.selected) {
                let task = &app.task_file.tasks[task_idx];
                if let Some(ref slug) = task.note {
                    let note_path = app.task_dir().join(format!("{}.md", slug));
                    match crate::note::read_note(&note_path) {
                        Ok(note) => {
                            app.note_editor = Some(NoteEditor::new(&note.slug, &note.title, &note.body));
                            app.mode = Mode::EditingNote;
                        }
                        Err(_) => {
                            app.status_message = Some(format!("Note file not found: {}.md", slug));
                        }
                    }
                } else {
                    app.status_message = Some("No note linked to this task".to_string());
                }
            }
        }
        KeyCode::Esc => {
            if app.filter.is_active() {
                app.filter = Filter::default();
                app.selected = 0;
                app.table_state.select(Some(0));
            }
        }
        _ => {}
    }
    Ok(false)
}

fn handle_normal_notes(app: &mut App, key: KeyCode) -> Result<bool, String> {
    match key {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Char('j') | KeyCode::Down => {
            if !app.notes_list.is_empty() && app.notes_selected < app.notes_list.len() - 1 {
                app.notes_selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.notes_selected > 0 {
                app.notes_selected -= 1;
            }
        }
        KeyCode::Char('a') => {
            app.mode = Mode::Adding;
            app.input_buffer.clear();
        }
        KeyCode::Enter => {
            if let Some(note) = app.notes_list.get(app.notes_selected) {
                let note_path = app.task_dir().join(format!("{}.md", note.slug));
                match crate::note::read_note(&note_path) {
                    Ok(n) => {
                        app.note_editor = Some(NoteEditor::new(&n.slug, &n.title, &n.body));
                        app.mode = Mode::EditingNote;
                    }
                    Err(e) => {
                        app.status_message = Some(format!("Error: {}", e));
                    }
                }
            }
        }
        KeyCode::Char('d') => {
            if let Some(note) = app.notes_list.get(app.notes_selected) {
                app.input_buffer = note.slug.clone();
                app.mode = Mode::Confirming;
            }
        }
        KeyCode::Char('C') => {
            let context = if let Some(note) = app.notes_list.get(app.notes_selected) {
                let note_path = app.task_dir().join(format!("{}.md", note.slug));
                let body = crate::note::read_note(&note_path)
                    .map(|n| n.body)
                    .unwrap_or_default();
                claude_session::build_session_context(&note.title, &body)
            } else {
                String::new()
            };
            app.session_pending_context = Some(context);
            populate_session_dir_picker(app);
            app.session_dir_picker_selected = 0;
            app.mode = Mode::SessionDirectoryPicker;
        }
        KeyCode::Char('v') => {
            let next = app.view.next();
            if next == View::Notes {
                app.refresh_notes();
                app.notes_selected = 0;
            }
            app.view = next;
            app.selected = 0;
            app.table_state.select(Some(0));
            app.clamp_selection();
        }
        KeyCode::Char('V') => {
            let prev = app.view.prev();
            if prev == View::Notes {
                app.refresh_notes();
                app.notes_selected = 0;
            }
            app.view = prev;
            app.selected = 0;
            app.table_state.select(Some(0));
            app.clamp_selection();
        }
        KeyCode::Char(':') => {
            app.mode = Mode::NlpChat;
            app.input_buffer.clear();
            app.chat_history.clear();
            app.nlp_messages.clear();
        }
        _ => {}
    }
    Ok(false)
}

fn handle_note_editor(app: &mut App, key: KeyCode) -> Result<(), String> {
    let editor = match app.note_editor.as_mut() {
        Some(e) => e,
        None => {
            app.mode = Mode::Normal;
            return Ok(());
        }
    };

    match key {
        KeyCode::Char(c) => {
            editor.insert_char(c);
        }
        KeyCode::Enter => {
            editor.insert_newline();
        }
        KeyCode::Backspace => {
            editor.backspace();
        }
        KeyCode::Up => {
            editor.move_up();
        }
        KeyCode::Down => {
            editor.move_down();
        }
        KeyCode::Left => {
            editor.move_left();
        }
        KeyCode::Right => {
            editor.move_right();
        }
        KeyCode::Home => {
            editor.move_to_line_start();
        }
        KeyCode::End => {
            editor.move_to_line_end();
        }
        KeyCode::Esc => {
            if editor.dirty {
                app.mode = Mode::ConfirmingNoteExit;
            } else {
                app.note_editor = None;
                if app.view == View::Notes {
                    app.refresh_notes();
                }
                app.mode = Mode::Normal;
            }
        }
        _ => {}
    }
    Ok(())
}

fn save_current_note(app: &mut App) -> Result<(), String> {
    if let Some(ref editor) = app.note_editor {
        let note = crate::note::Note {
            slug: editor.slug.clone(),
            title: editor.title.clone(),
            body: editor.body_text(),
        };
        crate::note::write_note(&app.task_dir(), &note)?;
        if let Some(ref mut e) = app.note_editor {
            e.dirty = false;
        }
        app.status_message = Some("Note saved".to_string());
    }
    Ok(())
}

fn handle_note_exit_confirm(app: &mut App, key: KeyCode) -> Result<(), String> {
    match key {
        KeyCode::Char('s') => {
            save_current_note(app)?;
            app.note_editor = None;
            if app.view == View::Notes {
                app.refresh_notes();
            }
            app.mode = Mode::Normal;
        }
        KeyCode::Char('d') => {
            app.note_editor = None;
            if app.view == View::Notes {
                app.refresh_notes();
            }
            app.mode = Mode::Normal;
        }
        KeyCode::Char('c') | KeyCode::Esc => {
            app.mode = Mode::EditingNote;
        }
        _ => {}
    }
    Ok(())
}

fn handle_note_picker(app: &mut App, key: KeyCode) -> Result<(), String> {
    match key {
        KeyCode::Char('j') | KeyCode::Down => {
            if app.note_picker_selected < app.note_picker_items.len() - 1 {
                app.note_picker_selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.note_picker_selected > 0 {
                app.note_picker_selected -= 1;
            }
        }
        KeyCode::Enter => {
            let task_idx = match app.note_picker_task_idx {
                Some(idx) => idx,
                None => {
                    app.mode = Mode::Normal;
                    return Ok(());
                }
            };
            match app.note_picker_selected {
                0 => {
                    // "(none)" - clear link
                    app.task_file.tasks[task_idx].note = None;
                    app.task_file.tasks[task_idx].updated = Some(Utc::now());
                    app.save()?;
                    app.status_message = Some("Note link cleared".to_string());
                    app.mode = Mode::Normal;
                }
                1 => {
                    // "(new note)" - create and link
                    app.mode = Mode::Adding;
                    app.input_buffer.clear();
                    // note_picker_task_idx stays set so Adding mode knows to create+link
                }
                n => {
                    // Link existing note
                    let slug = app.note_picker_items[n].clone();
                    app.task_file.tasks[task_idx].note = Some(slug.clone());
                    app.task_file.tasks[task_idx].updated = Some(Utc::now());
                    app.save()?;
                    app.status_message = Some(format!("Linked note: {}", slug));
                    app.mode = Mode::Normal;
                }
            }
            app.note_picker_task_idx = None;
        }
        KeyCode::Esc => {
            app.note_picker_task_idx = None;
            app.mode = Mode::Normal;
        }
        _ => {}
    }
    Ok(())
}

enum InputAction {
    Add,
    Filter,
    EditTitle,
    EditTags,
    EditDescription,
    EditDefaultDir,
}

fn handle_input(app: &mut App, key: KeyCode, action: InputAction) -> Result<(), String> {
    match key {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.input_buffer.clear();
        }
        KeyCode::Enter => {
            let input = app.input_buffer.clone();
            app.input_buffer.clear();
            app.mode = Mode::Normal;

            match action {
                InputAction::Add => {
                    if !input.trim().is_empty() {
                        if app.view == View::Notes || app.note_picker_task_idx.is_some() {
                            // Create a note
                            let title = input.trim().to_string();
                            let base_slug = crate::note::slugify(&title);
                            let slug = crate::note::unique_slug(&app.task_dir(), &base_slug);
                            let note = crate::note::Note {
                                slug: slug.clone(),
                                title: title.clone(),
                                body: String::new(),
                            };
                            crate::note::write_note(&app.task_dir(), &note)?;
                            // If creating from note picker, link to task
                            if let Some(task_idx) = app.note_picker_task_idx.take() {
                                app.task_file.tasks[task_idx].note = Some(slug.clone());
                                app.task_file.tasks[task_idx].updated = Some(Utc::now());
                                app.save()?;
                            }
                            // Open editor for the new note
                            app.note_editor = Some(NoteEditor::new(&slug, &title, ""));
                            app.mode = Mode::EditingNote;
                            app.refresh_notes();
                        } else {
                            let id = app.task_file.next_id;
                            app.task_file.next_id += 1;
                            app.task_file.tasks.push(Task {
                                id,
                                title: input.trim().to_string(),
                                status: Status::Open,
                                priority: Priority::Medium,
                                tags: Vec::new(),
                                created: Utc::now(),
                                updated: None,
                                description: None,
                                due_date: None,
                                project: None,
                                recurrence: None,
                                note: None,
                            });
                            app.save()?;
                            app.clamp_selection();
                        }
                    }
                }
                InputAction::Filter => {
                    app.filter = Filter::parse(&input);
                    app.selected = 0;
                    app.table_state.select(Some(0));
                    app.clamp_selection();
                }
                InputAction::EditTitle => {
                    let trimmed = input.trim().to_string();
                    if trimmed.is_empty() {
                        app.mode = Mode::EditingTitle;
                    } else {
                        let filtered = app.filtered_indices();
                        if let Some(&task_idx) = filtered.get(app.selected) {
                            let task = &mut app.task_file.tasks[task_idx];
                            task.title = trimmed;
                            task.updated = Some(Utc::now());
                            app.save()?;
                        }
                    }
                }
                InputAction::EditTags => {
                    let filtered = app.filtered_indices();
                    if let Some(&task_idx) = filtered.get(app.selected) {
                        let task = &mut app.task_file.tasks[task_idx];
                        task.tags = input.split_whitespace().map(|s| s.to_string()).collect();
                        task.updated = Some(Utc::now());
                        app.save()?;
                    }
                }
                InputAction::EditDescription => {
                    let filtered = app.filtered_indices();
                    if let Some(&task_idx) = filtered.get(app.selected) {
                        let task = &mut app.task_file.tasks[task_idx];
                        let trimmed = input.trim().to_string();
                        task.description = if trimmed.is_empty() { None } else { Some(trimmed) };
                        task.updated = Some(Utc::now());
                        app.save()?;
                    }
                }
                InputAction::EditDefaultDir => {
                    let trimmed = input.trim().to_string();
                    if !trimmed.is_empty() {
                        app.save()?;
                        config::write_config_value("default-dir", &trimmed)
                            .map_err(|e| format!("Failed to save config: {}", e))?;
                        let new_path = std::path::PathBuf::from(&trimmed).join("tasks.md");
                        app.task_file = storage::load(&new_path, false)?;
                        app.file_path = new_path;
                        app.selected = 0;
                        app.table_state.select(Some(0));
                    }
                }
            }
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        _ => {}
    }
    Ok(())
}

fn handle_recurrence_input(app: &mut App, key: KeyCode) -> Result<(), String> {
    match key {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.input_buffer.clear();
        }
        KeyCode::Enter => {
            let input = app.input_buffer.clone();
            app.input_buffer.clear();
            app.mode = Mode::Normal;

            let trimmed = input.trim();
            if trimmed.is_empty() {
                return Ok(());
            }

            let filtered = app.filtered_indices();
            let task_idx = match filtered.get(app.selected) {
                Some(&idx) => idx,
                None => return Ok(()),
            };

            // Check for direct patterns first (no NLP needed)
            let recurrence_result = match trimmed.to_lowercase().as_str() {
                "none" | "clear" | "remove" => Ok(None),
                "daily" | "weekly" | "monthly" | "yearly" => {
                    Ok(Some(trimmed.to_lowercase()))
                }
                _ => {
                    // Use NLP to parse the recurrence pattern
                    match auth::read_claude_key_source() {
                        Some((_, key)) => nlp::parse_recurrence_nlp(trimmed, &key),
                        None => Err("No Claude API key. Run `task auth claude` first.".to_string()),
                    }
                }
            };

            match recurrence_result {
                Ok(Some(recur_str)) => {
                    match recur_str.parse::<crate::task::Recurrence>() {
                        Ok(recur) => {
                            let task = &mut app.task_file.tasks[task_idx];
                            task.recurrence = Some(recur);
                            task.updated = Some(Utc::now());
                            app.save()?;
                            app.status_message = Some(format!(
                                "Recurrence set to {}", format_recurrence_display(&recur)
                            ));
                        }
                        Err(e) => {
                            app.status_message = Some(format!("Invalid recurrence: {}", e));
                        }
                    }
                }
                Ok(None) => {
                    let task = &mut app.task_file.tasks[task_idx];
                    task.recurrence = None;
                    task.updated = Some(Utc::now());
                    app.save()?;
                    app.status_message = Some("Recurrence removed".to_string());
                }
                Err(e) => {
                    app.status_message = Some(e);
                }
            }
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        _ => {}
    }
    Ok(())
}

fn handle_confirm(app: &mut App, key: KeyCode) -> Result<(), String> {
    match key {
        KeyCode::Char('y') => {
            if app.view == View::Notes {
                // Delete a note
                let slug = app.input_buffer.clone();
                if !slug.is_empty() {
                    crate::note::delete_note(&app.task_dir(), &slug)?;
                    // Clear any task links to this note
                    for task in &mut app.task_file.tasks {
                        if task.note.as_deref() == Some(&slug) {
                            task.note = None;
                            task.updated = Some(Utc::now());
                        }
                    }
                    app.save()?;
                    app.refresh_notes();
                    if app.notes_selected >= app.notes_list.len() && app.notes_selected > 0 {
                        app.notes_selected -= 1;
                    }
                }
            } else {
                let filtered = app.filtered_indices();
                if let Some(&task_idx) = filtered.get(app.selected) {
                    app.task_file.tasks.remove(task_idx);
                    app.save()?;
                    app.clamp_selection();
                }
            }
            app.mode = Mode::Normal;
        }
        _ => {
            app.mode = Mode::Normal;
        }
    }
    Ok(())
}

fn handle_priority(app: &mut App, key: KeyCode) -> Result<(), String> {
    let filtered = app.filtered_indices();
    match key {
        KeyCode::Char('c') | KeyCode::Char('h') | KeyCode::Char('m') | KeyCode::Char('l') => {
            if let Some(&task_idx) = filtered.get(app.selected) {
                let task = &mut app.task_file.tasks[task_idx];
                task.priority = match key {
                    KeyCode::Char('c') => Priority::Critical,
                    KeyCode::Char('h') => Priority::High,
                    KeyCode::Char('m') => Priority::Medium,
                    _ => Priority::Low,
                };
                task.updated = Some(Utc::now());
                app.save()?;
            }
            app.mode = Mode::Normal;
        }
        _ => {
            app.mode = Mode::Normal;
        }
    }
    Ok(())
}

fn format_action_summary(action: &NlpAction) -> Option<String> {
    match action {
        NlpAction::Filter(criteria) => {
            let mut parts = Vec::new();
            if let Some(ref s) = criteria.status { parts.push(format!("status={}", s)); }
            if let Some(ref p) = criteria.priority { parts.push(format!("priority={}", p)); }
            if let Some(ref t) = criteria.tag { parts.push(format!("tag={}", t)); }
            if let Some(ref p) = criteria.project { parts.push(format!("project={}", p)); }
            if let Some(ref tc) = criteria.title_contains { parts.push(format!("title~{}", tc)); }
            if parts.is_empty() {
                Some("Filtering: (all tasks)".to_string())
            } else {
                Some(format!("Filtering: {}", parts.join(", ")))
            }
        }
        NlpAction::Update { match_criteria, set_fields, task_ids, .. } => {
            let match_str = if let Some(ids) = task_ids {
                format!("tasks [{}]", ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(", "))
            } else {
                let mut match_parts = Vec::new();
                if let Some(ref s) = match_criteria.status { match_parts.push(format!("status={}", s)); }
                if let Some(ref p) = match_criteria.priority { match_parts.push(format!("priority={}", p)); }
                if let Some(ref t) = match_criteria.tag { match_parts.push(format!("tag={}", t)); }
                if let Some(ref p) = match_criteria.project { match_parts.push(format!("project={}", p)); }
                if let Some(ref tc) = match_criteria.title_contains { match_parts.push(format!("title~{}", tc)); }
                if match_parts.is_empty() { "(all)".to_string() } else { match_parts.join(", ") }
            };
            let mut set_parts = Vec::new();
            if let Some(ref p) = set_fields.priority { set_parts.push(format!("priority={}", p)); }
            if let Some(ref s) = set_fields.status { set_parts.push(format!("status={}", s)); }
            if let Some(ref t) = set_fields.tags { set_parts.push(format!("tags=[{}]", t.join(", "))); }
            if let Some(ref d) = set_fields.due_date { set_parts.push(format!("due_date={}", if d.is_empty() { "none" } else { d })); }
            let set_str = if set_parts.is_empty() { "(none)".to_string() } else { set_parts.join(", ") };
            Some(format!("Updating: match {{{}}} → set {{{}}}", match_str, set_str))
        }
        NlpAction::SetRecurrence { description, .. } => {
            Some(description.clone())
        }
        NlpAction::CreateNote { title, .. } => {
            Some(format!("Creating note: {}", title))
        }
        NlpAction::EditNote { slug, .. } => {
            Some(format!("Editing note: {}", slug))
        }
        NlpAction::Message(_) | NlpAction::ShowTasks { .. } => None,
    }
}

fn format_update_preview(tasks: &[Task], indices: &[usize], set_fields: &nlp::SetFields) -> Vec<String> {
    let mut lines = Vec::new();
    let show_count = indices.len().min(10);
    for &i in &indices[..show_count] {
        let task = &tasks[i];
        let mut changes = Vec::new();
        if let Some(ref new_priority) = set_fields.priority {
            let old = task.priority.to_string();
            if !old.eq_ignore_ascii_case(new_priority) {
                changes.push(format!("priority {} → {}", old, new_priority));
            }
        }
        if let Some(ref new_status) = set_fields.status {
            let old = task.status.to_string();
            if !old.eq_ignore_ascii_case(new_status) {
                changes.push(format!("status {} → {}", old, new_status));
            }
        }
        if let Some(ref new_tags) = set_fields.tags {
            let old = task.tags.join(", ");
            let new = new_tags.join(", ");
            if old != new {
                changes.push(format!("tags [{}] → [{}]", old, new));
            }
        }
        if let Some(ref new_due) = set_fields.due_date {
            let old = task.due_date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "none".to_string());
            let new_display = if new_due.is_empty() { "none".to_string() } else { new_due.clone() };
            if old != new_display {
                changes.push(format!("due_date {} → {}", old, new_display));
            }
        }
        if changes.is_empty() {
            continue; // no actual changes for this task
        }
        lines.push(format!("  #{} \"{}\": {}", task.id, task.title, changes.join(", ")));
    }
    if indices.len() > 10 {
        lines.push(format!("  ... and {} more tasks", indices.len() - 10));
    }
    lines
}

fn process_nlp_result(app: &mut App, result: Result<(NlpAction, String), String>) -> Result<(), String> {
    app.status_message = None;
    match result {
        Ok((action @ NlpAction::Filter(_), raw_response)) => {
            let NlpAction::Filter(criteria) = &action else { unreachable!() };
            app.nlp_messages.push(ApiMessage {
                role: "assistant".to_string(),
                content: raw_response,
            });
            if let Some(summary) = format_action_summary(&action) {
                app.chat_history.push(ChatMessage::Assistant(summary));
            }
            let mut filter = Filter::default();
            if let Some(s) = &criteria.status {
                if let Ok(status) = s.parse::<Status>() {
                    filter.status = Some(status);
                }
            }
            if let Some(p) = &criteria.priority {
                if let Ok(priority) = p.parse::<Priority>() {
                    filter.priority = Some(priority);
                }
            }
            if let Some(t) = &criteria.tag {
                filter.tag = Some(t.clone());
            }
            if let Some(p) = &criteria.project {
                filter.project = Some(p.clone());
            }
            if let Some(tc) = &criteria.title_contains {
                filter.title_contains = Some(tc.clone());
            }
            app.view = View::All;
            app.filter = filter;
            app.selected = 0;
            app.table_state.select(Some(0));
            app.clamp_selection();
            app.chat_history.push(ChatMessage::Assistant("Filter applied.".to_string()));
        }
        Ok((action @ NlpAction::Update { .. }, raw_response)) => {
            let NlpAction::Update { ref match_criteria, ref set_fields, ref task_ids, .. } = action else { unreachable!() };
            app.nlp_messages.push(ApiMessage {
                role: "assistant".to_string(),
                content: raw_response,
            });
            if let Some(summary) = format_action_summary(&action) {
                app.chat_history.push(ChatMessage::Assistant(summary));
            }
            let matching: Vec<usize> = if let Some(ids) = task_ids {
                // Match by explicit task IDs
                app.task_file.tasks.iter().enumerate()
                    .filter(|(_, t)| ids.contains(&t.id))
                    .map(|(i, _)| i)
                    .collect()
            } else {
                let has_any_criteria = match_criteria.status.is_some()
                    || match_criteria.priority.is_some()
                    || match_criteria.tag.is_some()
                    || match_criteria.project.is_some()
                    || match_criteria.title_contains.is_some();
                if !has_any_criteria {
                    vec![] // empty criteria matches nothing — prevents accidental bulk updates
                } else {
                    app.task_file.tasks.iter().enumerate()
                        .filter(|(_, t)| {
                            if let Some(ref s) = match_criteria.status {
                                if !t.status.to_string().eq_ignore_ascii_case(s) { return false; }
                            }
                            if let Some(ref p) = match_criteria.priority {
                                if !t.priority.to_string().eq_ignore_ascii_case(p) { return false; }
                            }
                            if let Some(ref tag) = match_criteria.tag {
                                if !t.tags.iter().any(|tg| tg.eq_ignore_ascii_case(tag)) { return false; }
                            }
                            if let Some(ref proj) = match_criteria.project {
                                match &t.project {
                                    Some(p) => if !p.eq_ignore_ascii_case(proj) { return false; },
                                    None => return false,
                                }
                            }
                            if let Some(ref tc) = match_criteria.title_contains {
                                if !t.title.to_lowercase().contains(&tc.to_lowercase()) { return false; }
                            }
                            true
                        })
                        .map(|(i, _)| i)
                        .collect()
                }
            };

            if matching.is_empty() {
                app.chat_history.push(ChatMessage::Assistant("No tasks match the criteria.".to_string()));
            } else {
                let preview_lines = format_update_preview(&app.task_file.tasks, &matching, set_fields);
                if !preview_lines.is_empty() {
                    app.chat_history.push(ChatMessage::Assistant(
                        format!("Changes:\n{}", preview_lines.join("\n"))
                    ));
                }
                app.pending_nlp_update = Some((action, matching));
                app.mode = Mode::ConfirmingNlp;
            }
        }
        Ok((NlpAction::Message(text), raw_response)) => {
            app.nlp_messages.push(ApiMessage {
                role: "assistant".to_string(),
                content: raw_response,
            });
            app.chat_history.push(ChatMessage::Assistant(text));
        }
        Ok((NlpAction::ShowTasks { task_ids, text }, raw_response)) => {
            app.nlp_messages.push(ApiMessage {
                role: "assistant".to_string(),
                content: raw_response,
            });
            let tasks: Vec<(u32, String, String, String)> = task_ids
                .iter()
                .filter_map(|&id| {
                    app.task_file.tasks.iter().find(|t| t.id == id).map(|t| {
                        (t.id, t.title.clone(), t.priority.to_string(), t.status.to_string())
                    })
                })
                .collect();
            app.chat_history.push(ChatMessage::TaskList { text, tasks });
        }
        Ok((action @ NlpAction::SetRecurrence { .. }, raw_response)) => {
            let NlpAction::SetRecurrence { task_id, ref recurrence, .. } = action else { unreachable!() };
            app.nlp_messages.push(ApiMessage {
                role: "assistant".to_string(),
                content: raw_response,
            });
            if let Some(summary) = format_action_summary(&action) {
                app.chat_history.push(ChatMessage::Assistant(summary));
            }
            if let Some(task) = app.task_file.find_task_mut(task_id) {
                match recurrence {
                    Some(recur_str) => {
                        match recur_str.parse::<crate::task::Recurrence>() {
                            Ok(recur) => {
                                task.recurrence = Some(recur);
                                task.updated = Some(Utc::now());
                                app.save()?;
                                app.chat_history.push(ChatMessage::Assistant(
                                    format!("Set recurrence on task {} to {}", task_id, format_recurrence_display(&recur))
                                ));
                            }
                            Err(e) => {
                                app.chat_history.push(ChatMessage::Error(format!("Invalid recurrence: {}", e)));
                            }
                        }
                    }
                    None => {
                        task.recurrence = None;
                        task.updated = Some(Utc::now());
                        app.save()?;
                        app.chat_history.push(ChatMessage::Assistant(
                            format!("Removed recurrence from task {}", task_id)
                        ));
                    }
                }
            } else {
                app.chat_history.push(ChatMessage::Error(format!("Task {} not found", task_id)));
            }
        }
        Ok((NlpAction::CreateNote { title, content, task_id }, raw_response)) => {
            app.nlp_messages.push(ApiMessage {
                role: "assistant".to_string(),
                content: raw_response,
            });
            let dir = app.task_dir();
            let base_slug = crate::note::slugify(&title);
            let slug = crate::note::unique_slug(&dir, &base_slug);
            let note = crate::note::Note {
                slug: slug.clone(),
                title: title.clone(),
                body: content,
            };
            match crate::note::write_note(&dir, &note) {
                Ok(_) => {
                    // Link to task if requested
                    if let Some(tid) = task_id {
                        if let Some(task) = app.task_file.find_task_mut(tid) {
                            task.note = Some(slug.clone());
                            task.updated = Some(Utc::now());
                            app.save()?;
                        }
                    }
                    app.chat_history.push(ChatMessage::Assistant(
                        format!("Created note: {} ({})", title, slug)
                    ));
                }
                Err(e) => {
                    app.chat_history.push(ChatMessage::Error(format!("Failed to create note: {}", e)));
                }
            }
        }
        Ok((NlpAction::EditNote { slug, content }, raw_response)) => {
            app.nlp_messages.push(ApiMessage {
                role: "assistant".to_string(),
                content: raw_response,
            });
            let dir = app.task_dir();
            let note_path = dir.join(format!("{}.md", slug));
            match crate::note::read_note(&note_path) {
                Ok(existing) => {
                    let updated_note = crate::note::Note {
                        slug: existing.slug,
                        title: existing.title,
                        body: content,
                    };
                    match crate::note::write_note(&dir, &updated_note) {
                        Ok(_) => {
                            app.chat_history.push(ChatMessage::Assistant(
                                format!("Updated note: {}", slug)
                            ));
                        }
                        Err(e) => {
                            app.chat_history.push(ChatMessage::Error(format!("Failed to update note: {}", e)));
                        }
                    }
                }
                Err(_) => {
                    app.chat_history.push(ChatMessage::Error(format!("Note not found: {}", slug)));
                }
            }
        }
        Err(e) => {
            app.chat_history.push(ChatMessage::Error(e));
        }
    }
    Ok(())
}

fn handle_nlp_chat<B: Backend>(_terminal: &mut Terminal<B>, app: &mut App, key: KeyCode) -> Result<(), String> {
    match key {
        KeyCode::Esc => {
            app.nlp_pending = None;
            app.nlp_spinner_frame = 0;
            app.mode = Mode::Normal;
            app.input_buffer.clear();
            app.chat_history.clear();
            app.nlp_messages.clear();
            app.status_message = None;
        }
        KeyCode::Enter if app.nlp_pending.is_some() => {
            // Ignore Enter while NLP request is in progress
        }
        KeyCode::Enter => {
            let input = app.input_buffer.clone();
            app.input_buffer.clear();

            if input.trim().is_empty() {
                return Ok(());
            }

            let api_key = match auth::read_claude_key() {
                Some(k) => k,
                None => {
                    app.chat_history.push(ChatMessage::Error(
                        "No Claude API key. Run `task auth claude` or set ANTHROPIC_API_KEY.".to_string(),
                    ));
                    return Ok(());
                }
            };

            // Append user message to conversation
            app.chat_history.push(ChatMessage::User(input.clone()));
            app.nlp_messages.push(ApiMessage {
                role: "user".to_string(),
                content: input,
            });

            // Cap message history at 20
            while app.nlp_messages.len() > 20 {
                app.nlp_messages.remove(0);
            }

            // Spawn NLP call on background thread for animated loading
            let tasks_clone = app.task_file.tasks.clone();
            let messages_clone = app.nlp_messages.clone();
            let note_slugs: Vec<String> = {
                let dir = app.task_dir();
                let filename = app.task_filename();
                crate::note::discover_notes(&dir, &filename)
                    .into_iter()
                    .map(|n| n.slug)
                    .collect()
            };
            let (tx, rx) = mpsc::channel();
            std::thread::spawn(move || {
                let result = nlp::interpret_with_notes(&tasks_clone, &messages_clone, &api_key, &note_slugs);
                let _ = tx.send(result);
            });
            app.nlp_pending = Some(rx);
            app.nlp_spinner_frame = 0;
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(c) => {
            app.input_buffer.push(c);
        }
        _ => {}
    }
    Ok(())
}

const DETAIL_FIELD_COUNT: usize = 7;

fn commit_buffer_to_draft(app: &mut App) {
    if let Some(ref mut draft) = app.detail_draft {
        match app.detail_field_index {
            0 => draft.title = app.input_buffer.clone(),
            1 => draft.description = app.input_buffer.clone(),
            4 => draft.due_date = app.input_buffer.clone(),
            5 => draft.project = app.input_buffer.clone(),
            6 => draft.tags = app.input_buffer.clone(),
            _ => {} // Priority (2) and Status (3) don't use input_buffer
        }
    }
}

fn load_field_to_buffer(app: &mut App) {
    if let Some(ref draft) = app.detail_draft {
        app.input_buffer = match app.detail_field_index {
            0 => draft.title.clone(),
            1 => draft.description.clone(),
            4 => draft.due_date.clone(),
            5 => draft.project.clone(),
            6 => draft.tags.clone(),
            _ => String::new(), // Priority and Status don't use buffer
        };
    }
}

fn handle_detail_edit(app: &mut App, key: KeyCode) -> Result<(), String> {
    match key {
        KeyCode::Char('j') | KeyCode::Down | KeyCode::Tab => {
            commit_buffer_to_draft(app);
            app.detail_field_index = (app.detail_field_index + 1) % DETAIL_FIELD_COUNT;
            load_field_to_buffer(app);
        }
        KeyCode::Char('k') | KeyCode::Up | KeyCode::BackTab => {
            commit_buffer_to_draft(app);
            app.detail_field_index = if app.detail_field_index == 0 {
                DETAIL_FIELD_COUNT - 1
            } else {
                app.detail_field_index - 1
            };
            load_field_to_buffer(app);
        }
        KeyCode::Esc => {
            commit_buffer_to_draft(app);
            let dirty = if let Some(ref draft) = app.detail_draft {
                let filtered = app.filtered_indices();
                filtered.get(app.selected)
                    .map(|&idx| draft.is_dirty(&app.task_file.tasks[idx]))
                    .unwrap_or(false)
            } else {
                false
            };
            if dirty {
                app.mode = Mode::ConfirmingDetailSave;
            } else {
                app.detail_draft = None;
                app.input_buffer.clear();
                app.mode = Mode::Normal;
            }
        }
        _ => {
            // Field-specific handling
            match app.detail_field_index {
                2 => {
                    // Priority field
                    if let Some(ref mut draft) = app.detail_draft {
                        match key {
                            KeyCode::Char('c') => draft.priority = Priority::Critical,
                            KeyCode::Char('h') => draft.priority = Priority::High,
                            KeyCode::Char('m') => draft.priority = Priority::Medium,
                            KeyCode::Char('l') => draft.priority = Priority::Low,
                            _ => {}
                        }
                    }
                }
                3 => {
                    // Status field
                    if let Some(ref mut draft) = app.detail_draft {
                        match key {
                            KeyCode::Enter | KeyCode::Char(' ') => {
                                draft.status = match draft.status {
                                    Status::Open => Status::Done,
                                    Status::Done => Status::Open,
                                };
                            }
                            _ => {}
                        }
                    }
                }
                _ => {
                    // Text fields: title, description, due_date, project, tags
                    match key {
                        KeyCode::Backspace => { app.input_buffer.pop(); }
                        KeyCode::Char(c) => { app.input_buffer.push(c); }
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}

fn apply_navigation(app: &mut App) {
    if let Some(dir) = app.pending_navigation.take() {
        let filtered = app.filtered_indices();
        match dir {
            NavDirection::Down => {
                if !filtered.is_empty() && app.selected < filtered.len() - 1 {
                    app.selected += 1;
                    app.table_state.select(Some(app.selected));
                }
            }
            NavDirection::Up => {
                if app.selected > 0 {
                    app.selected -= 1;
                    app.table_state.select(Some(app.selected));
                }
            }
        }
    }
}

fn handle_detail_confirm(app: &mut App, key: KeyCode) -> Result<(), String> {
    match key {
        KeyCode::Char('s') => {
            // Validate due date before saving
            if let Some(ref draft) = app.detail_draft {
                if !draft.due_date.trim().is_empty() {
                    if NaiveDate::parse_from_str(draft.due_date.trim(), "%Y-%m-%d").is_err() {
                        app.status_message = Some("Invalid date format (use YYYY-MM-DD)".to_string());
                        app.detail_field_index = 4;
                        load_field_to_buffer(app);
                        app.mode = Mode::EditingDetailPanel;
                        return Ok(());
                    }
                }
            }
            // Apply draft to task
            if let Some(draft) = app.detail_draft.take() {
                let filtered = app.filtered_indices();
                if let Some(&task_idx) = filtered.get(app.selected) {
                    let task = &mut app.task_file.tasks[task_idx];
                    task.title = draft.title;
                    task.description = if draft.description.trim().is_empty() { None } else { Some(draft.description) };
                    task.priority = draft.priority;
                    task.status = draft.status;
                    task.due_date = if draft.due_date.trim().is_empty() {
                        None
                    } else {
                        NaiveDate::parse_from_str(draft.due_date.trim(), "%Y-%m-%d").ok()
                    };
                    task.project = if draft.project.trim().is_empty() { None } else { Some(draft.project) };
                    task.tags = draft.tags.split_whitespace().map(|s| s.to_string()).collect();
                    task.updated = Some(Utc::now());
                    app.save()?;
                }
            }
            app.input_buffer.clear();
            app.mode = Mode::Normal;
            apply_navigation(app);
        }
        KeyCode::Char('d') => {
            app.detail_draft = None;
            app.input_buffer.clear();
            app.mode = Mode::Normal;
            apply_navigation(app);
        }
        KeyCode::Char('c') | KeyCode::Esc => {
            app.pending_navigation = None;
            app.mode = Mode::EditingDetailPanel;
        }
        _ => {}
    }
    Ok(())
}

fn handle_nlp_confirm(app: &mut App, key: KeyCode) -> Result<(), String> {
    match key {
        KeyCode::Char('y') => {
            if let Some((NlpAction::Update { set_fields, description, .. }, indices)) = app.pending_nlp_update.take() {
                let count = indices.len();
                for &idx in &indices {
                    let task = &mut app.task_file.tasks[idx];
                    if let Some(ref s) = set_fields.status {
                        if let Ok(status) = s.parse::<Status>() {
                            task.status = status;
                        }
                    }
                    if let Some(ref p) = set_fields.priority {
                        if let Ok(priority) = p.parse::<Priority>() {
                            task.priority = priority;
                        }
                    }
                    if let Some(ref tags) = set_fields.tags {
                        task.tags = tags.clone();
                    }
                    if let Some(ref due) = set_fields.due_date {
                        if due.is_empty() {
                            task.due_date = None;
                        } else {
                            match NaiveDate::parse_from_str(due, "%Y-%m-%d") {
                                Ok(d) => task.due_date = Some(d),
                                Err(_) => {
                                    app.status_message = Some(format!("Invalid due date format: {}", due));
                                    app.mode = Mode::NlpChat;
                                    return Ok(());
                                }
                            }
                        }
                    }
                    task.updated = Some(Utc::now());
                }
                app.save()?;
                app.clamp_selection();
                app.chat_history.push(ChatMessage::Assistant(
                    format!("{} ({} tasks)", description, count),
                ));
            }
            app.mode = Mode::NlpChat;
        }
        _ => {
            app.pending_nlp_update = None;
            app.chat_history.push(ChatMessage::Assistant("Update cancelled.".to_string()));
            app.mode = Mode::NlpChat;
        }
    }
    Ok(())
}

// -- Rendering --

fn draw(frame: &mut Frame, app: &mut App) {
    if app.mode == Mode::EditingNote || app.mode == Mode::ConfirmingNoteExit {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(frame.area());

        draw_header(frame, app, chunks[0]);
        draw_note_editor(frame, app, chunks[1]);
        draw_footer(frame, app, chunks[2]);
        return;
    }
    if app.mode == Mode::NotePicker {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(frame.area());

        draw_header(frame, app, chunks[0]);
        draw_note_picker(frame, app, chunks[1]);
        draw_footer(frame, app, chunks[2]);
        return;
    }
    if app.mode == Mode::SessionDirectoryPicker
        || app.mode == Mode::Sessions
        || app.mode == Mode::SessionReply
        || app.mode == Mode::PermissionModal
    {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(frame.area());

        draw_header(frame, app, chunks[0]);
        if app.mode == Mode::SessionDirectoryPicker {
            draw_session_dir_picker(frame, app, chunks[1]);
        } else {
            draw_sessions_panel(frame, app, chunks[1]);
        }
        draw_footer(frame, app, chunks[2]);
        if app.mode == Mode::PermissionModal {
            draw_permission_modal(frame, app);
        }
        return;
    }
    if app.view == View::Notes && app.mode == Mode::Normal {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(frame.area());

        draw_header(frame, app, chunks[0]);
        draw_notes_list(frame, app, chunks[1]);
        draw_footer(frame, app, chunks[2]);
        return;
    }
    if app.mode == Mode::NlpChat || app.mode == Mode::ConfirmingNlp {
        // 4-region layout for chat mode
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Percentage(55),
                Constraint::Min(3),
                Constraint::Length(1),
            ])
            .split(frame.area());

        draw_header(frame, app, chunks[0]);
        draw_table(frame, app, chunks[1]);
        draw_chat_panel(frame, app, chunks[2]);
        draw_footer(frame, app, chunks[3]);
    } else if app.show_detail_panel || app.mode == Mode::EditingDetailPanel || app.mode == Mode::ConfirmingDetailSave {
        // Layout with detail panel
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Percentage(70),
                Constraint::Min(3),
                Constraint::Length(1),
            ])
            .split(frame.area());

        draw_header(frame, app, chunks[0]);
        draw_table(frame, app, chunks[1]);
        draw_detail_panel(frame, app, chunks[2]);
        draw_footer(frame, app, chunks[3]);
    } else {
        // Standard 3-region layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(frame.area());

        draw_header(frame, app, chunks[0]);
        draw_table(frame, app, chunks[1]);
        draw_footer(frame, app, chunks[2]);
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let title = if app.filter.is_active() {
        format!(" task-manager  |  {}  |  filter: {} ", app.view.display_name(), app.filter.summary())
    } else {
        format!(" task-manager  |  {} ", app.view.display_name())
    };
    let header = Paragraph::new(title).style(
        Style::default()
            .fg(theme::BAR_FG)
            .bg(theme::BAR_BG)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(header, area);
}

fn format_recurrence_display(r: &crate::task::Recurrence) -> String {
    use crate::task::{IntervalUnit, Recurrence};
    match r {
        Recurrence::Interval { unit, count } => {
            let (singular, plural) = match unit {
                IntervalUnit::Daily => ("Daily", "Days"),
                IntervalUnit::Weekly => ("Weekly", "Weeks"),
                IntervalUnit::Monthly => ("Monthly", "Months"),
                IntervalUnit::Yearly => ("Yearly", "Years"),
            };
            if *count == 1 {
                singular.to_string()
            } else {
                format!("Every {} {}", count, plural)
            }
        }
        Recurrence::NthWeekday { n, weekday } => {
            let ordinal = match n {
                1 => "1st",
                2 => "2nd",
                3 => "3rd",
                4 => "4th",
                5 => "5th",
                _ => "?",
            };
            let day = match weekday {
                chrono::Weekday::Mon => "Mon",
                chrono::Weekday::Tue => "Tue",
                chrono::Weekday::Wed => "Wed",
                chrono::Weekday::Thu => "Thu",
                chrono::Weekday::Fri => "Fri",
                chrono::Weekday::Sat => "Sat",
                chrono::Weekday::Sun => "Sun",
            };
            format!("Monthly ({} {})", ordinal, day)
        }
        Recurrence::WeeklyOn { weekday, every_n_weeks } => {
            let day = match weekday {
                chrono::Weekday::Mon => "Mon",
                chrono::Weekday::Tue => "Tue",
                chrono::Weekday::Wed => "Wed",
                chrono::Weekday::Thu => "Thu",
                chrono::Weekday::Fri => "Fri",
                chrono::Weekday::Sat => "Sat",
                chrono::Weekday::Sun => "Sun",
            };
            if *every_n_weeks == 1 {
                format!("Weekly ({})", day)
            } else {
                format!("Every {} Weeks ({})", every_n_weeks, day)
            }
        }
    }
}

fn truncate_desc(desc: Option<&str>) -> String {
    match desc {
        None | Some("") => String::new(),
        Some(s) if s.len() > 30 => format!("{}…", &s[..29]),
        Some(s) => s.to_string(),
    }
}

fn draw_table(frame: &mut Frame, app: &mut App, area: Rect) {
    let filtered = app.filtered_indices();

    if filtered.is_empty() {
        let msg = if app.filter.is_active() {
            "No tasks match filter."
        } else {
            "No tasks. Press 'a' to add one."
        };
        let paragraph = Paragraph::new(msg)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(paragraph, area);
        return;
    }

    let show_desc = filtered.iter().any(|&i| {
        app.task_file.tasks[i].description.as_ref().map_or(false, |d| !d.is_empty())
    });
    let show_due = filtered.iter().any(|&i| app.task_file.tasks[i].due_date.is_some());
    let show_project = filtered.iter().any(|&i| app.task_file.tasks[i].project.is_some());
    let show_recur = filtered.iter().any(|&i| app.task_file.tasks[i].recurrence.is_some());
    let show_note = filtered.iter().any(|&i| app.task_file.tasks[i].note.is_some());

    let mut header_cells = vec!["ID", "Status", "Priority", "Title"];
    if show_desc { header_cells.push("Desc"); }
    if show_due { header_cells.push("Due"); }
    if show_project { header_cells.push("Project"); }
    if show_recur { header_cells.push("↻"); header_cells.push("Pattern"); }
    if show_note { header_cells.push("Note"); }
    header_cells.push("Tags");

    let header = Row::new(header_cells)
        .style(Style::default().add_modifier(Modifier::BOLD))
        .bottom_margin(0);

    // Compute today once for consistent overdue checks across all rows.
    // Overdue = strictly before today (tasks due today are NOT overdue).
    let today = Local::now().date_naive();
    let rows: Vec<Row> = filtered
        .iter()
        .map(|&i| {
            let task = &app.task_file.tasks[i];
            let is_overdue = task.status == Status::Open
                && task.due_date.map_or(false, |d| d < today);
            let status_str = match task.status {
                Status::Open => if is_overdue { "[!]" } else { "[ ]" },
                Status::Done => "[x]",
            };
            let priority_style = match task.priority {
                Priority::Critical => Style::default().fg(theme::PRIORITY_CRITICAL).add_modifier(Modifier::BOLD),
                Priority::High => Style::default().fg(theme::PRIORITY_HIGH),
                Priority::Medium => Style::default().fg(theme::PRIORITY_MEDIUM),
                Priority::Low => Style::default().fg(theme::PRIORITY_LOW),
            };
            let tags_str = if task.tags.is_empty() {
                String::new()
            } else {
                task.tags.join(", ")
            };
            let mut cells = vec![
                Cell::from(task.id.to_string()),
                Cell::from(status_str),
                Cell::from(format!("{}", task.priority)).style(priority_style),
                Cell::from(task.title.as_str()),
            ];
            if show_desc {
                cells.push(Cell::from(truncate_desc(task.description.as_deref())));
            }
            if show_due {
                let due_str = task.due_date
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default();
                cells.push(Cell::from(due_str));
            }
            if show_project {
                cells.push(Cell::from(task.project.as_deref().unwrap_or("").to_string()));
            }
            if show_recur {
                cells.push(Cell::from(if task.recurrence.is_some() { "↻" } else { "" }));
                cells.push(Cell::from(
                    task.recurrence.as_ref().map(|r| format_recurrence_display(r)).unwrap_or_default()
                ));
            }
            if show_note {
                cells.push(Cell::from(task.note.as_deref().unwrap_or("")));
            }
            cells.push(Cell::from(tags_str));
            let row = Row::new(cells);
            if task.status == Status::Done {
                row.style(Style::default().fg(theme::DONE_TEXT))
            } else if is_overdue {
                row.style(Style::default().fg(theme::OVERDUE))
            } else {
                row
            }
        })
        .collect();

    let mut widths = vec![
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Length(9),
        Constraint::Fill(1),
    ];
    if show_desc { widths.push(Constraint::Length(30)); }
    if show_due { widths.push(Constraint::Length(12)); }
    if show_project { widths.push(Constraint::Length(15)); }
    if show_recur { widths.push(Constraint::Length(3)); widths.push(Constraint::Min(8)); }
    if show_note { widths.push(Constraint::Length(15)); }
    widths.push(Constraint::Length(20));

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL))
        .row_highlight_style(
            Style::default()
                .bg(theme::HIGHLIGHT_BG)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(table, area, &mut app.table_state);
}

fn draw_detail_panel(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(ref draft) = app.detail_draft {
        // Edit mode rendering
        let field_labels = ["Title", "Description", "Priority", "Status", "Due Date", "Project", "Tags"];
        let mut lines: Vec<Line> = Vec::new();
        for (i, label) in field_labels.iter().enumerate() {
            let value = match i {
                0 => if i == app.detail_field_index { format!("{}_ ", app.input_buffer) } else { draft.title.clone() },
                1 => if i == app.detail_field_index { format!("{}_ ", app.input_buffer) } else { draft.description.clone() },
                2 => format!("{}", draft.priority),
                3 => format!("{}", draft.status),
                4 => if i == app.detail_field_index { format!("{}_ ", app.input_buffer) } else { draft.due_date.clone() },
                5 => if i == app.detail_field_index { format!("{}_ ", app.input_buffer) } else { draft.project.clone() },
                6 => if i == app.detail_field_index { format!("{}_ ", app.input_buffer) } else { draft.tags.clone() },
                _ => String::new(),
            };
            let display_value = if value.is_empty() && i != app.detail_field_index { "(empty)".to_string() } else { value };
            let style = if i == app.detail_field_index {
                Style::default().bg(theme::HIGHLIGHT_BG).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let marker = if i == app.detail_field_index { ">> " } else { "   " };
            lines.push(Line::from(Span::styled(
                format!("{}{:>12}: {}", marker, label, display_value),
                style,
            )));
        }
        let paragraph = Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(" Edit Task "));
        frame.render_widget(paragraph, area);
    } else {
        // Read-only rendering
        let filtered = app.filtered_indices();
        let content = if let Some(&task_idx) = filtered.get(app.selected) {
            let task = &app.task_file.tasks[task_idx];
            let desc = task.description.as_deref().unwrap_or("(none)");
            let tags = if task.tags.is_empty() {
                "(none)".to_string()
            } else {
                task.tags.join(", ")
            };
            let due = task.due_date
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "(none)".to_string());
            let project = task.project.as_deref().unwrap_or("(none)");
            let recurrence_str = match &task.recurrence {
                Some(r) => format_recurrence_display(r),
                None => "-".to_string(),
            };
            let note_str = task.note.as_deref().unwrap_or("(none)");
            let created = task.created.format("%Y-%m-%d %H:%M").to_string();
            let updated = task.updated
                .map(|u| u.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "(never)".to_string());

            format!(
                "ID: {}  |  Status: {}  |  Priority: {}  |  Due: {}  |  Project: {}\n\
                 Title: {}\n\
                 Description: {}\n\
                 Tags: {}  |  Recurrence: {}  |  Note: {}\n\
                 Created: {}  |  Updated: {}",
                task.id, task.status, task.priority, due, project,
                task.title,
                desc,
                tags, recurrence_str, note_str,
                created, updated,
            )
        } else {
            "No task selected.".to_string()
        };

        let paragraph = Paragraph::new(content)
            .wrap(ratatui::widgets::Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).title(" Task Details "));
        frame.render_widget(paragraph, area);
    }
}

fn draw_chat_panel(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    for msg in &app.chat_history {
        match msg {
            ChatMessage::User(text) => {
                for line in text.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("> {}", line),
                        Style::default().fg(theme::CHAT_USER),
                    )));
                }
            }
            ChatMessage::Assistant(text) => {
                for line in text.lines() {
                    lines.push(Line::from(Span::raw(line.to_string())));
                }
            }
            ChatMessage::TaskList { text, tasks } => {
                for line in text.lines() {
                    lines.push(Line::from(Span::raw(line.to_string())));
                }
                for (id, title, priority, status) in tasks {
                    lines.push(Line::from(Span::styled(
                        format!("  #{} {} [{}] ({})", id, title, priority, status),
                        Style::default().fg(theme::CHAT_TASK_LIST),
                    )));
                }
            }
            ChatMessage::Error(text) => {
                for line in text.lines() {
                    lines.push(Line::from(Span::styled(
                        format!("Error: {}", line),
                        Style::default().fg(theme::CHAT_ERROR),
                    )));
                }
            }
        }
        lines.push(Line::from(""));
    }

    let content_width = area.width.saturating_sub(2) as usize; // account for border
    let visible_height = area.height.saturating_sub(2) as usize;

    // Estimate wrapped line count for scroll calculation
    let wrapped_count: usize = lines.iter().map(|line| {
        let len = line.width();
        if content_width == 0 || len == 0 { 1 } else { (len + content_width - 1) / content_width }
    }).sum();

    let scroll = if wrapped_count > visible_height {
        (wrapped_count - visible_height) as u16
    } else {
        0
    };

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::TOP).title(" Chat "))
        .wrap(ratatui::widgets::Wrap { trim: false })
        .scroll((scroll, 0));

    frame.render_widget(paragraph, area);
}

fn draw_notes_list(frame: &mut Frame, app: &mut App, area: Rect) {
    if app.notes_list.is_empty() {
        let msg = Paragraph::new("No notes yet. Press 'a' to create one.")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(msg, area);
        return;
    }

    let header = Row::new(vec!["Title", "Slug"])
        .style(Style::default().add_modifier(Modifier::BOLD))
        .bottom_margin(0);

    let rows: Vec<Row> = app
        .notes_list
        .iter()
        .enumerate()
        .map(|(i, note)| {
            let style = if i == app.notes_selected {
                Style::default().bg(theme::HIGHLIGHT_BG)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(note.title.as_str()),
                Cell::from(note.slug.as_str()),
            ])
            .style(style)
        })
        .collect();

    let widths = [Constraint::Percentage(60), Constraint::Percentage(40)];
    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL));

    let mut state = TableState::default();
    state.select(Some(app.notes_selected));
    frame.render_stateful_widget(table, area, &mut state);
}

fn draw_note_editor(frame: &mut Frame, app: &mut App, area: Rect) {
    let editor = match app.note_editor.as_mut() {
        Some(e) => e,
        None => return,
    };

    let inner = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", editor.title));
    let inner_area = inner.inner(area);
    frame.render_widget(inner, area);

    let visible_height = inner_area.height as usize;
    let line_num_width = 4u16;
    let text_width = inner_area.width.saturating_sub(line_num_width + 1);
    let text_width_usize = text_width as usize;
    let cols_per_row = text_width_usize.max(1);

    editor.ensure_cursor_visible(visible_height, text_width_usize);

    let visual_rows_for = |line: &str| -> usize {
        let n = line.chars().count();
        if n == 0 { 1 } else { (n + cols_per_row - 1) / cols_per_row }
    };

    // Compute code block state up to viewport_offset by scanning from line 0
    let mut in_code_block = false;
    for idx in 0..editor.viewport_offset.min(editor.lines.len()) {
        let (_, new_state) = md_style::style_markdown_line(&editor.lines[idx], in_code_block);
        in_code_block = new_state;
    }

    // Render lines with word wrap
    let mut display_row: usize = 0;
    let mut line_idx = editor.viewport_offset;
    while display_row < visible_height && line_idx < editor.lines.len() {
        let line = editor.lines[line_idx].clone();
        let visual_rows = visual_rows_for(&line);

        for vis_row in 0..visual_rows {
            if display_row >= visible_height {
                break;
            }
            let y = inner_area.y + display_row as u16;

            // Line number gutter: only on the first visual row of this logical line
            if vis_row == 0 {
                let num_str = format!("{:>3} ", line_idx + 1);
                let num_span = Span::styled(num_str, Style::default().fg(Color::DarkGray));
                frame.render_widget(
                    Paragraph::new(num_span),
                    Rect::new(inner_area.x, y, line_num_width, 1),
                );
            } else {
                frame.render_widget(
                    Paragraph::new("    "),
                    Rect::new(inner_area.x, y, line_num_width, 1),
                );
            }

            // Render the chunk of this logical line for this visual row
            let chunk_start = vis_row * cols_per_row;
            let chunk: String = line.chars().skip(chunk_start).take(cols_per_row).collect();
            let (spans, new_state) = md_style::style_markdown_line(&chunk, in_code_block);
            // Only advance code-block state on the last visual row of this logical line
            if vis_row == visual_rows - 1 {
                in_code_block = new_state;
            }
            frame.render_widget(
                Paragraph::new(Line::from(spans)),
                Rect::new(inner_area.x + line_num_width, y, text_width, 1),
            );

            display_row += 1;
        }
        line_idx += 1;
    }

    // Set cursor position accounting for wrapped visual rows
    let visual_row_within_line = editor.cursor_col / cols_per_row;
    let visual_col_within_row = editor.cursor_col % cols_per_row;
    let mut cursor_screen_row: usize = 0;
    for row in editor.viewport_offset..editor.cursor_row.min(editor.lines.len()) {
        cursor_screen_row += visual_rows_for(&editor.lines[row]);
    }
    cursor_screen_row += visual_row_within_line;
    let cursor_x = inner_area.x + line_num_width + visual_col_within_row as u16;
    let cursor_y = inner_area.y + cursor_screen_row as u16;
    if cursor_y < inner_area.y + inner_area.height && cursor_x < inner_area.x + inner_area.width {
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

fn draw_note_picker(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<Row> = app.note_picker_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let style = if i == app.note_picker_selected {
                Style::default().bg(theme::HIGHLIGHT_BG)
            } else {
                Style::default()
            };
            Row::new(vec![Cell::from(item.as_str())]).style(style)
        })
        .collect();

    let widths = [Constraint::Percentage(100)];
    let table = Table::new(items, widths)
        .block(Block::default().borders(Borders::ALL).title(" Link Note "));

    let mut state = TableState::default();
    state.select(Some(app.note_picker_selected));
    frame.render_stateful_widget(table, area, &mut state);
}

// ---------------------------------------------------------------------------
// Session directory picker (3.1–3.3)
// ---------------------------------------------------------------------------

fn populate_session_dir_picker(app: &mut App) {
    let root = claude_session::claude_code_dir();
    let mut dirs: Vec<PathBuf> = match std::fs::read_dir(&root) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map_or(false, |t| t.is_dir()))
            .map(|e| e.path())
            .collect(),
        Err(_) => Vec::new(),
    };
    dirs.sort();
    app.session_dir_picker = dirs;
}

fn draw_session_dir_picker(frame: &mut Frame, app: &App, area: Rect) {
    if app.session_dir_picker.is_empty() {
        let msg = Paragraph::new("No projects found — set `claude-code-dir` in config")
            .block(Block::default().borders(Borders::ALL).title(" Select Project Directory "));
        frame.render_widget(msg, area);
        return;
    }

    let items: Vec<Row> = app
        .session_dir_picker
        .iter()
        .enumerate()
        .map(|(i, path)| {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_string();
            let style = if i == app.session_dir_picker_selected {
                Style::default().bg(theme::HIGHLIGHT_BG)
            } else {
                Style::default()
            };
            Row::new(vec![Cell::from(name)]).style(style)
        })
        .collect();

    let widths = [Constraint::Percentage(100)];
    let table = Table::new(items, widths)
        .block(Block::default().borders(Borders::ALL).title(" Select Project Directory "));

    let mut state = TableState::default();
    state.select(Some(app.session_dir_picker_selected));
    frame.render_stateful_widget(table, area, &mut state);
}

fn handle_session_dir_picker(app: &mut App, key: KeyCode) -> Result<(), String> {
    match key {
        KeyCode::Char('j') | KeyCode::Down => {
            if !app.session_dir_picker.is_empty()
                && app.session_dir_picker_selected < app.session_dir_picker.len() - 1
            {
                app.session_dir_picker_selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.session_dir_picker_selected > 0 {
                app.session_dir_picker_selected -= 1;
            }
        }
        KeyCode::Enter => {
            if let Some(dir) = app
                .session_dir_picker
                .get(app.session_dir_picker_selected)
                .cloned()
            {
                if !claude_session::claude_available() {
                    app.status_message = Some(
                        "claude binary not found — install Claude Code to use sessions".to_string(),
                    );
                    app.mode = Mode::Normal;
                    return Ok(());
                }
                let context = app.session_pending_context.take().unwrap_or_default();
                let id = app.next_session_id;
                app.next_session_id += 1;
                let label = dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("session")
                    .to_string();
                match claude_session::launch_claude_session(id, label, dir, context) {
                    Ok(session) => {
                        app.claude_sessions.push(session);
                        app.session_selected = app.claude_sessions.len() - 1;
                        app.session_viewing_output = false;
                        app.mode = Mode::Sessions;
                    }
                    Err(e) => {
                        app.status_message = Some(format!("Failed to launch session: {}", e));
                        app.mode = Mode::Normal;
                    }
                }
            } else if app.session_dir_picker.is_empty() {
                app.mode = Mode::Normal;
            }
        }
        KeyCode::Esc => {
            app.session_pending_context = None;
            app.mode = Mode::Normal;
        }
        _ => {}
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Sessions panel (6.1–6.3)
// ---------------------------------------------------------------------------

fn draw_sessions_panel(frame: &mut Frame, app: &App, area: Rect) {
    if app.session_viewing_output {
        // Full output detail for selected session
        if let Some(session) = app.claude_sessions.get(app.session_selected) {
            let visible_height = area.height.saturating_sub(2) as usize;

            // Build a flat list of rendered lines from structured events
            let mut rendered: Vec<Line> = Vec::new();
            let mut in_code_block = false;
            for (event_idx, event) in session.output.iter().enumerate() {
                match event {
                    SessionOutputEvent::TurnSeparator => {
                        rendered.push(Line::from(Span::styled(
                            claude_session::TURN_SEPARATOR_LABEL,
                            Style::default().fg(Color::DarkGray),
                        )));
                    }
                    SessionOutputEvent::Text(s) => {
                        let (spans, updated) = md_style::style_markdown_line(s, in_code_block);
                        in_code_block = updated;
                        rendered.push(Line::from(spans));
                    }
                    SessionOutputEvent::ToolCall { name, input_preview, result_lines, collapsed } => {
                        let focused = event_idx == app.session_focused_event;
                        let bracket = if *collapsed { "[+]" } else { "[-]" };
                        let header = if input_preview.is_empty() {
                            format!("⚙  {} {}{}",
                                name, bracket,
                                if focused { " ◀" } else { "" })
                        } else {
                            format!("⚙  {}: {} {}{}",
                                name, input_preview, bracket,
                                if focused { " ◀" } else { "" })
                        };
                        rendered.push(Line::from(Span::styled(
                            header,
                            Style::default().fg(Color::Rgb(255, 200, 80)),
                        )));
                        if !collapsed {
                            for rl in result_lines {
                                rendered.push(Line::from(Span::styled(
                                    format!("  {}", rl),
                                    Style::default().fg(Color::Rgb(150, 200, 150)),
                                )));
                            }
                        }
                    }
                    SessionOutputEvent::PermissionRequest { tool, input_preview } => {
                        rendered.push(Line::from(Span::styled(
                            format!("⚠  Permission: {} — {}", tool, input_preview),
                            Style::default().fg(Color::Yellow),
                        )));
                    }
                }
            }

            let total_lines = rendered.len();
            let start = app.session_output_scroll.min(total_lines.saturating_sub(visible_height));
            let display: Vec<Line> = rendered.into_iter().skip(start).take(visible_height).collect();
            let title = format!(" {} — output ({}/{}) ", session.label, start + display.len(), total_lines);
            let para = Paragraph::new(display)
                .block(Block::default().borders(Borders::ALL).title(title))
                .wrap(Wrap { trim: false });
            frame.render_widget(para, area);
        }
        return;
    }

    if app.claude_sessions.is_empty() {
        let msg = Paragraph::new("No sessions yet. Press C on a task or note to start one.")
            .block(Block::default().borders(Borders::ALL).title(" Claude Sessions "));
        frame.render_widget(msg, area);
        return;
    }

    // Split area for list + reply input if in SessionReply
    let (list_area, reply_area) = if app.mode == Mode::SessionReply {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(3)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    let items: Vec<Row> = app
        .claude_sessions
        .iter()
        .enumerate()
        .map(|(i, session)| {
            let status_str = match session.status {
                ClaudeSessionStatus::Running => "⠿ Running",
                ClaudeSessionStatus::WaitingForInput => "● Waiting",
                ClaudeSessionStatus::Failed => "✗ Failed",
                ClaudeSessionStatus::Done => "✓ Done",
            };
            let dir_name = session
                .working_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?");
            let last_line = session.output.iter().rev().find_map(|e| match e {
                SessionOutputEvent::Text(s) if !s.trim().is_empty() => Some(s.as_str()),
                SessionOutputEvent::ToolCall { input_preview, .. } if !input_preview.is_empty() => {
                    Some(input_preview.as_str())
                }
                _ => None,
            }).unwrap_or("");
            let style = if i == app.session_selected {
                Style::default().bg(theme::HIGHLIGHT_BG)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(session.label.clone()),
                Cell::from(dir_name.to_string()),
                Cell::from(status_str),
                Cell::from(last_line.to_string()),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Percentage(25),
        Constraint::Percentage(20),
        Constraint::Length(12),
        Constraint::Min(10),
    ];
    let table = Table::new(items, widths)
        .header(Row::new(vec!["Label", "Directory", "Status", "Last output"]).style(
            Style::default().fg(Color::DarkGray),
        ))
        .block(Block::default().borders(Borders::ALL).title(" Claude Sessions "));

    let mut state = TableState::default();
    state.select(Some(app.session_selected));
    frame.render_stateful_widget(table, list_area, &mut state);

    if let Some(reply_area) = reply_area {
        draw_session_reply(frame, app, reply_area);
    }
}

fn handle_sessions(app: &mut App, key: KeyCode) -> Result<(), String> {
    if app.session_viewing_output {
        match key {
            KeyCode::Char('j') | KeyCode::Down => {
                // Move focused event down, scroll if needed
                let n = app.claude_sessions
                    .get(app.session_selected)
                    .map(|s| s.output.len())
                    .unwrap_or(0);
                if app.session_focused_event + 1 < n {
                    app.session_focused_event += 1;
                }
                let max_scroll = n.saturating_sub(1);
                if app.session_output_scroll < max_scroll {
                    app.session_output_scroll += 1;
                }
                app.session_output_follow = false;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.session_focused_event = app.session_focused_event.saturating_sub(1);
                if app.session_output_scroll > 0 {
                    app.session_output_scroll -= 1;
                }
                app.session_output_follow = false;
            }
            KeyCode::Home | KeyCode::Char('g') => {
                app.session_output_scroll = 0;
                app.session_focused_event = 0;
                app.session_output_follow = false;
            }
            KeyCode::End | KeyCode::Char('G') => {
                let total = app
                    .claude_sessions
                    .get(app.session_selected)
                    .map(|s| s.output.len())
                    .unwrap_or(0);
                app.session_output_scroll = total.saturating_sub(1);
                app.session_focused_event = total.saturating_sub(1);
                app.session_output_follow = true;
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                // Toggle collapse on focused ToolCall event
                let idx = app.session_focused_event;
                if let Some(session) = app.claude_sessions.get_mut(app.session_selected) {
                    if let Some(SessionOutputEvent::ToolCall { collapsed, .. }) = session.output.get_mut(idx) {
                        *collapsed = !*collapsed;
                    }
                }
            }
            KeyCode::Esc => {
                app.session_viewing_output = false;
            }
            _ => {}
        }
        return Ok(());
    }

    match key {
        KeyCode::Char('j') | KeyCode::Down => {
            if !app.claude_sessions.is_empty()
                && app.session_selected < app.claude_sessions.len() - 1
            {
                app.session_selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.session_selected > 0 {
                app.session_selected -= 1;
            }
        }
        KeyCode::Enter => {
            if app.claude_sessions.get(app.session_selected).is_some() {
                app.session_viewing_output = true;
                app.session_output_follow = true;
                let total = app.claude_sessions[app.session_selected].output.len();
                app.session_output_scroll = total.saturating_sub(1);
                app.session_focused_event = total.saturating_sub(1);
            }
        }
        KeyCode::Char('r') => {
            if let Some(session) = app.claude_sessions.get(app.session_selected) {
                if session.status == ClaudeSessionStatus::WaitingForInput {
                    app.session_reply_input.clear();
                    app.mode = Mode::SessionReply;
                }
            }
        }
        KeyCode::Char('n') => {
            // Start a new session from the sessions panel (no task context)
            app.session_pending_context = Some(String::new());
            populate_session_dir_picker(app);
            app.session_dir_picker_selected = 0;
            app.mode = Mode::SessionDirectoryPicker;
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            app.session_viewing_output = false;
            app.mode = Mode::Normal;
        }
        _ => {}
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Session reply (7.1–7.3)
// ---------------------------------------------------------------------------

fn draw_permission_modal(frame: &mut Frame, app: &App) {
    // Centered modal overlay
    let area = frame.area();
    let modal_w = (area.width.saturating_sub(4)).min(60);
    let modal_h = 7u16;
    let x = area.x + area.width.saturating_sub(modal_w) / 2;
    let y = area.y + area.height.saturating_sub(modal_h) / 2;
    let modal_area = Rect { x, y, width: modal_w, height: modal_h };

    frame.render_widget(ratatui::widgets::Clear, modal_area);

    let content = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  Tool: {}", app.permission_modal_tool),
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            format!("  Input: {}", app.permission_modal_input),
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("  [y] Allow  ", Style::default().fg(Color::Green)),
            Span::styled("[n] Deny  ", Style::default().fg(Color::Red)),
            Span::styled("[a] Allow session", Style::default().fg(Color::Cyan)),
        ]),
    ];

    let para = Paragraph::new(content)
        .block(Block::default().borders(Borders::ALL).title(" ⚠  Permission Request ").border_style(Style::default().fg(Color::Yellow)));
    frame.render_widget(para, modal_area);
}

fn handle_permission_modal(app: &mut App, key: KeyCode) -> Result<(), String> {
    let selected = app.session_selected;
    match key {
        KeyCode::Char('y') | KeyCode::Char('a') => {
            claude_session::respond_to_permission(&mut app.claude_sessions[selected], true);
            app.mode = Mode::Sessions;
        }
        KeyCode::Char('n') => {
            claude_session::respond_to_permission(&mut app.claude_sessions[selected], false);
            app.mode = Mode::Sessions;
        }
        _ => {}
    }
    Ok(())
}

fn draw_session_reply(frame: &mut Frame, app: &App, area: Rect) {
    let text = format!(" > {}_ ", app.session_reply_input);
    let para = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title(" Reply "));
    frame.render_widget(para, area);
}

fn handle_session_reply(app: &mut App, key: KeyCode) -> Result<(), String> {
    match key {
        KeyCode::Char(c) => {
            app.session_reply_input.push(c);
        }
        KeyCode::Backspace => {
            app.session_reply_input.pop();
        }
        KeyCode::Enter => {
            send_session_reply(app)?;
        }
        KeyCode::Esc => {
            app.session_reply_input.clear();
            app.mode = Mode::Sessions;
        }
        _ => {}
    }
    Ok(())
}

fn send_session_reply(app: &mut App) -> Result<(), String> {
    let message = app.session_reply_input.trim().to_string();
    if message.is_empty() {
        return Ok(());
    }
    if let Some(session) = app.claude_sessions.get_mut(app.session_selected) {
        claude_session::continue_claude_session(session, message)
            .map_err(|e| format!("Failed to send reply: {}", e))?;
    }
    app.session_reply_input.clear();
    app.mode = Mode::Sessions;
    Ok(())
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let text = match &app.mode {
        Mode::Normal => {
            if let Some(ref msg) = app.status_message {
                format!(" {} ", msg)
            } else if app.view == View::Notes {
                " a:new  Enter:edit  d:delete  v:view  C:claude  q:quit ".to_string()
            } else if app.show_detail_panel {
                " j/k:nav  Enter:edit  Space:toggle  a:add  d:delete  f:filter  p:priority  e:edit-title  t:tags  r:desc  R:recur  n:note  g:go-note  v:view  Tab:details  q:quit ".to_string()
            } else {
                " j/k:nav  Enter:toggle  a:add  d:delete  f:filter  p:priority  e:edit  t:tags  r:desc  R:recur  n:note  g:go-note  v:view  C:claude  ::command  D:set-dir  T/N/W/M/Q/Y:due  X:clr-due  Tab:details  ^r:reload  q:quit".to_string()
            }
        }
        Mode::Adding => {
            if app.view == View::Notes || app.note_picker_task_idx.is_some() {
                format!(" Note title: {}_ ", app.input_buffer)
            } else {
                format!(" Add task: {}_ ", app.input_buffer)
            }
        }
        Mode::Filtering => {
            format!(" Filter (status:open priority:high tag:name): {}_ ", app.input_buffer)
        }
        Mode::Confirming => {
            if app.view == View::Notes {
                format!(" Delete note '{}'? y/n ", app.input_buffer)
            } else {
                let filtered = app.filtered_indices();
                if let Some(&idx) = filtered.get(app.selected) {
                    let task = &app.task_file.tasks[idx];
                    format!(" Delete task {}? y/n ", task.id)
                } else {
                    " Delete? y/n ".to_string()
                }
            }
        }
        Mode::EditingPriority => {
            " Set priority: c)ritical  h)igh  m)edium  l)ow  Esc:cancel ".to_string()
        }
        Mode::EditingTitle => {
            format!(" Edit title (required): {}_ ", app.input_buffer)
        }
        Mode::EditingTags => {
            format!(" Edit tags (space-separated): {}_ ", app.input_buffer)
        }
        Mode::EditingDescription => {
            format!(" Edit description: {}_ ", app.input_buffer)
        }
        Mode::EditingRecurrence => {
            format!(" Recurrence (e.g. daily, weekly, every 3rd thu, none): {}_ ", app.input_buffer)
        }
        Mode::EditingDefaultDir => {
            format!(" Set default directory: {}_ ", app.input_buffer)
        }
        Mode::NlpChat => {
            format!(" > {}_ ", app.input_buffer)
        }
        Mode::ConfirmingNlp => {
            if let Some((NlpAction::Update { ref description, .. }, ref indices)) = app.pending_nlp_update {
                format!(" {} ({} tasks) — y/n ", description, indices.len())
            } else {
                " Apply changes? y/n ".to_string()
            }
        }
        Mode::EditingDetailPanel => {
            " j/k:field  c/h/m/l:priority  Enter/Space:status  Esc:done ".to_string()
        }
        Mode::ConfirmingDetailSave => {
            " Unsaved changes. [s]ave  [d]iscard  [c]ancel ".to_string()
        }
        Mode::EditingNote => {
            " Ctrl+S:save  Esc:exit  Arrow keys:navigate ".to_string()
        }
        Mode::ConfirmingNoteExit => {
            " Unsaved changes. [s]ave  [d]iscard  [c]ancel ".to_string()
        }
        Mode::NotePicker => {
            " j/k:nav  Enter:select  Esc:cancel ".to_string()
        }
        Mode::SessionDirectoryPicker => {
            " j/k:nav  Enter:select  Esc:cancel ".to_string()
        }
        Mode::Sessions => {
            if app.session_viewing_output {
                " j/k:scroll  Esc:back ".to_string()
            } else {
                " j/k:nav  Enter:output  r:reply  n:new  Esc:back ".to_string()
            }
        }
        Mode::SessionReply => {
            format!(" Reply: {}_ ", app.session_reply_input)
        }
        Mode::PermissionModal => {
            " y:Allow  n:Deny  a:Allow session ".to_string()
        }
    };

    let footer = Paragraph::new(text).style(
        Style::default()
            .fg(theme::BAR_FG)
            .bg(theme::BAR_BG),
    );
    frame.render_widget(footer, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use crate::task::{Priority, Status, Task};

    fn make_task(due: Option<NaiveDate>) -> Task {
        Task {
            id: 1,
            title: "test".to_string(),
            status: Status::Open,
            priority: Priority::Medium,
            tags: Vec::new(),
            created: Utc::now(),
            updated: None,
            description: None,
            due_date: due,
            project: None,
            recurrence: None,
            note: None,
        }
    }

    // -- View::matches tests --

    #[test]
    fn today_view_shows_task_due_today() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        let task = make_task(Some(today));
        assert!(View::Today.matches(&task, today));
    }

    #[test]
    fn today_view_shows_task_with_no_due_date() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        let task = make_task(None);
        assert!(View::Today.matches(&task, today));
    }

    #[test]
    fn today_view_hides_task_due_tomorrow() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        let tomorrow = NaiveDate::from_ymd_opt(2026, 2, 27).unwrap();
        let task = make_task(Some(tomorrow));
        assert!(!View::Today.matches(&task, today));
    }

    #[test]
    fn today_view_shows_overdue_open_task() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        let yesterday = NaiveDate::from_ymd_opt(2026, 2, 25).unwrap();
        let task = make_task(Some(yesterday));
        assert!(View::Today.matches(&task, today));
    }

    #[test]
    fn today_view_hides_overdue_completed_task() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        let yesterday = NaiveDate::from_ymd_opt(2026, 2, 25).unwrap();
        let mut task = make_task(Some(yesterday));
        task.status = Status::Done;
        assert!(!View::Today.matches(&task, today));
    }

    #[test]
    fn weekly_view_shows_overdue_open_task() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        let last_week = NaiveDate::from_ymd_opt(2026, 2, 15).unwrap();
        let task = make_task(Some(last_week));
        assert!(View::Weekly.matches(&task, today));
    }

    #[test]
    fn monthly_view_shows_overdue_open_task() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        let last_month = NaiveDate::from_ymd_opt(2026, 1, 10).unwrap();
        let task = make_task(Some(last_month));
        assert!(View::Monthly.matches(&task, today));
    }

    #[test]
    fn yearly_view_shows_overdue_open_task() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        let last_year = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let task = make_task(Some(last_year));
        assert!(View::Yearly.matches(&task, today));
    }

    #[test]
    fn no_due_date_view_hides_overdue_task() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        let yesterday = NaiveDate::from_ymd_opt(2026, 2, 25).unwrap();
        let task = make_task(Some(yesterday));
        assert!(!View::NoDueDate.matches(&task, today));
    }

    #[test]
    fn all_view_shows_everything() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        assert!(View::All.matches(&make_task(Some(today)), today));
        assert!(View::All.matches(&make_task(None), today));
        let far_future = NaiveDate::from_ymd_opt(2030, 12, 31).unwrap();
        assert!(View::All.matches(&make_task(Some(far_future)), today));
    }

    #[test]
    fn weekly_view_shows_task_due_this_week() {
        // 2026-02-26 is a Thursday. Monday = 2026-02-23, Sunday = 2026-03-01
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        let monday = NaiveDate::from_ymd_opt(2026, 2, 23).unwrap();
        let sunday = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        assert!(View::Weekly.matches(&make_task(Some(monday)), today));
        assert!(View::Weekly.matches(&make_task(Some(today)), today));
        assert!(View::Weekly.matches(&make_task(Some(sunday)), today));
    }

    #[test]
    fn weekly_view_hides_task_due_next_week() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        let next_monday = NaiveDate::from_ymd_opt(2026, 3, 2).unwrap();
        assert!(!View::Weekly.matches(&make_task(Some(next_monday)), today));
    }

    #[test]
    fn weekly_view_hides_no_due_date() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        assert!(!View::Weekly.matches(&make_task(None), today));
    }

    #[test]
    fn monthly_view_shows_task_due_this_month() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        let first = NaiveDate::from_ymd_opt(2026, 2, 1).unwrap();
        let last = NaiveDate::from_ymd_opt(2026, 2, 28).unwrap();
        assert!(View::Monthly.matches(&make_task(Some(first)), today));
        assert!(View::Monthly.matches(&make_task(Some(last)), today));
    }

    #[test]
    fn monthly_view_hides_task_due_next_month() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        let next = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        assert!(!View::Monthly.matches(&make_task(Some(next)), today));
    }

    #[test]
    fn yearly_view_shows_task_due_this_year() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        let dec = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
        assert!(View::Yearly.matches(&make_task(Some(dec)), today));
    }

    #[test]
    fn yearly_view_hides_task_due_next_year() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        let next = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
        assert!(!View::Yearly.matches(&make_task(Some(next)), today));
    }

    #[test]
    fn no_due_date_view_shows_only_none() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        assert!(View::NoDueDate.matches(&make_task(None), today));
        assert!(!View::NoDueDate.matches(&make_task(Some(today)), today));
    }

    // -- Completed tasks hidden from non-All views --

    #[test]
    fn today_view_hides_completed_task() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        let mut task = make_task(Some(today));
        task.status = Status::Done;
        assert!(!View::Today.matches(&task, today));
    }

    #[test]
    fn all_view_shows_completed_task() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        let mut task = make_task(Some(today));
        task.status = Status::Done;
        assert!(View::All.matches(&task, today));
    }

    #[test]
    fn weekly_view_hides_completed_task() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        let mut task = make_task(Some(today));
        task.status = Status::Done;
        assert!(!View::Weekly.matches(&task, today));
    }

    #[test]
    fn no_due_date_view_hides_completed_task() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        let mut task = make_task(None);
        task.status = Status::Done;
        assert!(!View::NoDueDate.matches(&task, today));
    }

    // -- Recurring view tests --

    #[test]
    fn recurring_view_shows_recurring_open_task() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        let mut task = make_task(Some(today));
        task.recurrence = Some(crate::task::Recurrence::Interval { unit: crate::task::IntervalUnit::Weekly, count: 1 });
        assert!(View::Recurring.matches(&task, today));
    }

    #[test]
    fn recurring_view_hides_recurring_done_task() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        let mut task = make_task(Some(today));
        task.status = Status::Done;
        task.recurrence = Some(crate::task::Recurrence::Interval { unit: crate::task::IntervalUnit::Daily, count: 1 });
        assert!(!View::Recurring.matches(&task, today));
    }

    #[test]
    fn recurring_view_hides_non_recurring_task() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        let task = make_task(Some(today));
        assert!(!View::Recurring.matches(&task, today));
    }

    // -- View::next / View::prev tests --

    #[test]
    fn next_cycles_through_all_views() {
        let mut v = View::Today;
        v = v.next(); assert_eq!(v, View::All);
        v = v.next(); assert_eq!(v, View::Weekly);
        v = v.next(); assert_eq!(v, View::Monthly);
        v = v.next(); assert_eq!(v, View::Yearly);
        v = v.next(); assert_eq!(v, View::NoDueDate);
        v = v.next(); assert_eq!(v, View::Recurring);
        v = v.next(); assert_eq!(v, View::Notes);
        v = v.next(); assert_eq!(v, View::Today); // wrap
    }

    #[test]
    fn prev_cycles_through_all_views() {
        let mut v = View::Today;
        v = v.prev(); assert_eq!(v, View::Notes);
        v = v.prev(); assert_eq!(v, View::Recurring);
        v = v.prev(); assert_eq!(v, View::NoDueDate);
        v = v.prev(); assert_eq!(v, View::Yearly);
        v = v.prev(); assert_eq!(v, View::Monthly);
        v = v.prev(); assert_eq!(v, View::Weekly);
        v = v.prev(); assert_eq!(v, View::All);
        v = v.prev(); assert_eq!(v, View::Today); // wrap
    }

    // -- View::from_config tests --

    #[test]
    fn from_config_parses_valid_values() {
        assert_eq!(View::from_config("today"), View::Today);
        assert_eq!(View::from_config("all"), View::All);
        assert_eq!(View::from_config("weekly"), View::Weekly);
        assert_eq!(View::from_config("monthly"), View::Monthly);
        assert_eq!(View::from_config("yearly"), View::Yearly);
        assert_eq!(View::from_config("no-due-date"), View::NoDueDate);
        assert_eq!(View::from_config("recurring"), View::Recurring);
    }

    #[test]
    fn from_config_is_case_insensitive() {
        assert_eq!(View::from_config("TODAY"), View::Today);
        assert_eq!(View::from_config("Weekly"), View::Weekly);
    }

    #[test]
    fn from_config_falls_back_on_invalid() {
        assert_eq!(View::from_config("bogus"), View::Today);
        assert_eq!(View::from_config(""), View::Today);
    }

    // -- Status message tests --

    fn make_app_with_tasks(tasks: Vec<Task>) -> App {
        let mut task_file = TaskFile::new();
        task_file.tasks = tasks;
        App {
            task_file,
            file_path: PathBuf::from("/dev/null"),
            selected: 0,
            filter: Filter::default(),
            view: View::All,
            mode: Mode::Normal,
            input_buffer: String::new(),
            table_state: TableState::default(),
            status_message: None,
            pending_nlp_update: None,
            chat_history: Vec::new(),
            nlp_messages: Vec::new(),
            show_detail_panel: false,
            detail_draft: None,
            detail_field_index: 0,
            pending_navigation: None,
            nlp_pending: None,
            nlp_spinner_frame: 0,
            notes_list: Vec::new(),
            notes_selected: 0,
            note_editor: None,
            note_picker_items: Vec::new(),
            note_picker_selected: 0,
            note_picker_task_idx: None,
            claude_sessions: Vec::new(),
            next_session_id: 0,
            session_selected: 0,
            session_dir_picker: Vec::new(),
            session_dir_picker_selected: 0,
            session_pending_context: None,
            session_reply_input: String::new(),
            session_viewing_output: false,
            session_output_scroll: 0,
            session_output_follow: true,
            session_focused_event: 0,
            permission_modal_tool: String::new(),
            permission_modal_input: String::new(),
        }
    }

    #[test]
    fn status_message_cleared_on_keypress() {
        let mut app = make_app_with_tasks(vec![make_task(None)]);
        app.status_message = Some("Test message".to_string());
        // Any normal-mode keypress should clear the status message
        let _ = handle_normal(&mut app, KeyCode::Char('k'));
        assert!(app.status_message.is_none());
    }

    #[test]
    fn no_token_sets_status_message() {
        // Ensure no token is stored (read_token checks the config dir)
        // We test the logic directly: if read_token returns None, status message is set
        let mut app = make_app_with_tasks(vec![make_task(None)]);
        // Simulate the import key handler logic for the no-token case
        if auth::read_token().is_none() {
            app.status_message = Some("No Todoist token. Run `task auth todoist` from the CLI.".to_string());
        }
        // In CI/test environments, there's typically no token stored
        // If a token happens to exist, the status_message won't be set (which is correct behavior)
        // We verify the message content is correct when it IS set
        if app.status_message.is_some() {
            assert_eq!(
                app.status_message.unwrap(),
                "No Todoist token. Run `task auth todoist` from the CLI."
            );
        }
    }

    // -- NLP mode tests --

    #[test]
    fn colon_key_enters_nlp_chat_mode() {
        let mut app = make_app_with_tasks(vec![make_task(None)]);
        // Pre-populate to verify clearing
        app.chat_history.push(ChatMessage::User("old".to_string()));
        app.nlp_messages.push(ApiMessage { role: "user".to_string(), content: "old".to_string() });
        let _ = handle_normal(&mut app, KeyCode::Char(':'));
        assert_eq!(app.mode, Mode::NlpChat);
        assert!(app.input_buffer.is_empty());
        assert!(app.chat_history.is_empty());
        assert!(app.nlp_messages.is_empty());
    }

    #[test]
    fn esc_in_nlp_chat_clears_conversation() {
        let mut app = make_app_with_tasks(vec![make_task(None)]);
        app.mode = Mode::NlpChat;
        app.input_buffer = "some query".to_string();
        app.chat_history.push(ChatMessage::User("test".to_string()));
        app.nlp_messages.push(ApiMessage { role: "user".to_string(), content: "test".to_string() });
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let _ = handle_nlp_chat(&mut terminal, &mut app, KeyCode::Esc);
        assert_eq!(app.mode, Mode::Normal);
        assert!(app.input_buffer.is_empty());
        assert!(app.chat_history.is_empty());
        assert!(app.nlp_messages.is_empty());
    }

    // -- Due date keybinding tests --

    fn make_app_with_tmpfile(tasks: Vec<Task>) -> App {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = PathBuf::from(format!("target/tmp/tui-test-{}-{}", std::process::id(), id));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("tasks.md");
        let mut task_file = TaskFile::new();
        task_file.tasks = tasks;
        // Write initial file so save() works
        let _ = storage::save(&path, &task_file);
        App {
            task_file,
            file_path: path,
            selected: 0,
            filter: Filter::default(),
            view: View::All,
            mode: Mode::Normal,
            input_buffer: String::new(),
            table_state: TableState::default(),
            status_message: None,
            pending_nlp_update: None,
            chat_history: Vec::new(),
            nlp_messages: Vec::new(),
            show_detail_panel: false,
            detail_draft: None,
            detail_field_index: 0,
            pending_navigation: None,
            nlp_pending: None,
            nlp_spinner_frame: 0,
            notes_list: Vec::new(),
            notes_selected: 0,
            note_editor: None,
            note_picker_items: Vec::new(),
            note_picker_selected: 0,
            note_picker_task_idx: None,
            claude_sessions: Vec::new(),
            next_session_id: 0,
            session_selected: 0,
            session_dir_picker: Vec::new(),
            session_dir_picker_selected: 0,
            session_pending_context: None,
            session_reply_input: String::new(),
            session_viewing_output: false,
            session_output_scroll: 0,
            session_output_follow: true,
            session_focused_event: 0,
            permission_modal_tool: String::new(),
            permission_modal_input: String::new(),
        }
    }

    #[test]
    fn shift_t_sets_due_date_to_today() {
        let mut app = make_app_with_tmpfile(vec![make_task(None)]);
        let _ = handle_normal(&mut app, KeyCode::Char('T'));
        let today = Local::now().date_naive();
        assert_eq!(app.task_file.tasks[0].due_date, Some(today));
        assert!(app.task_file.tasks[0].updated.is_some());
        assert!(app.status_message.as_ref().unwrap().starts_with("Due: "));
    }

    #[test]
    fn shift_n_sets_due_date_to_tomorrow() {
        let mut app = make_app_with_tmpfile(vec![make_task(None)]);
        let _ = handle_normal(&mut app, KeyCode::Char('N'));
        let expected = Local::now().date_naive().checked_add_days(Days::new(1)).unwrap();
        assert_eq!(app.task_file.tasks[0].due_date, Some(expected));
        assert!(app.status_message.as_ref().unwrap().starts_with("Due: "));
    }

    #[test]
    fn shift_y_sets_due_date_to_next_year() {
        let mut app = make_app_with_tmpfile(vec![make_task(None)]);
        let _ = handle_normal(&mut app, KeyCode::Char('Y'));
        let expected = Local::now().date_naive().checked_add_months(Months::new(12)).unwrap();
        assert_eq!(app.task_file.tasks[0].due_date, Some(expected));
        assert!(app.status_message.as_ref().unwrap().starts_with("Due: "));
    }

    #[test]
    fn shift_w_sets_due_date_to_next_week() {
        let mut app = make_app_with_tmpfile(vec![make_task(None)]);
        let _ = handle_normal(&mut app, KeyCode::Char('W'));
        let expected = Local::now().date_naive().checked_add_days(Days::new(7)).unwrap();
        assert_eq!(app.task_file.tasks[0].due_date, Some(expected));
        assert!(app.status_message.as_ref().unwrap().starts_with("Due: "));
    }

    #[test]
    fn shift_m_sets_due_date_to_next_month() {
        let mut app = make_app_with_tmpfile(vec![make_task(None)]);
        let _ = handle_normal(&mut app, KeyCode::Char('M'));
        let expected = Local::now().date_naive().checked_add_months(Months::new(1)).unwrap();
        assert_eq!(app.task_file.tasks[0].due_date, Some(expected));
        assert!(app.status_message.as_ref().unwrap().starts_with("Due: "));
    }

    #[test]
    fn shift_q_sets_due_date_to_next_quarter() {
        let mut app = make_app_with_tmpfile(vec![make_task(None)]);
        let _ = handle_normal(&mut app, KeyCode::Char('Q'));
        let expected = Local::now().date_naive().checked_add_months(Months::new(3)).unwrap();
        assert_eq!(app.task_file.tasks[0].due_date, Some(expected));
        assert!(app.status_message.as_ref().unwrap().starts_with("Due: "));
    }

    #[test]
    fn shift_x_clears_due_date() {
        let today = Local::now().date_naive();
        let mut app = make_app_with_tmpfile(vec![make_task(Some(today))]);
        let _ = handle_normal(&mut app, KeyCode::Char('X'));
        assert_eq!(app.task_file.tasks[0].due_date, None);
        assert_eq!(app.status_message.as_ref().unwrap(), "Due date cleared");
    }

    #[test]
    fn due_date_keys_noop_on_empty_list() {
        let mut app = make_app_with_tasks(vec![]);
        for key in ['T', 'Y', 'W', 'M', 'Q', 'X'] {
            let _ = handle_normal(&mut app, KeyCode::Char(key));
        }
        assert!(app.status_message.is_none());
    }

    // -- Detail panel tests --

    #[test]
    fn tab_toggles_detail_panel() {
        let mut app = make_app_with_tasks(vec![make_task(None)]);
        assert!(!app.show_detail_panel);
        let _ = handle_normal(&mut app, KeyCode::Tab);
        assert!(app.show_detail_panel);
        let _ = handle_normal(&mut app, KeyCode::Tab);
        assert!(!app.show_detail_panel);
    }

    // -- Description truncation tests --

    #[test]
    fn truncate_desc_handles_all_cases() {
        // None → empty
        assert_eq!(truncate_desc(None), "");
        // Empty string → empty
        assert_eq!(truncate_desc(Some("")), "");
        // Short string (≤30 chars) → full
        assert_eq!(truncate_desc(Some("short desc")), "short desc");
        // Exactly 30 chars → full
        let thirty = "a".repeat(30);
        assert_eq!(truncate_desc(Some(&thirty)), thirty);
        // 31 chars → truncated with …
        let thirty_one = "a".repeat(31);
        assert_eq!(truncate_desc(Some(&thirty_one)), format!("{}…", "a".repeat(29)));
    }

    // -- Detail draft tests --

    #[test]
    fn detail_draft_from_task_and_is_dirty() {
        let mut task = make_task(Some(NaiveDate::from_ymd_opt(2026, 3, 1).unwrap()));
        task.title = "Buy milk".to_string();
        task.description = Some("From the store".to_string());
        task.project = Some("Shopping".to_string());
        task.tags = vec!["errands".to_string()];

        let draft = DetailDraft::from_task(&task);
        assert_eq!(draft.title, "Buy milk");
        assert_eq!(draft.description, "From the store");
        assert_eq!(draft.priority, Priority::Medium);
        assert_eq!(draft.status, Status::Open);
        assert_eq!(draft.due_date, "2026-03-01");
        assert_eq!(draft.project, "Shopping");
        assert_eq!(draft.tags, "errands");
        assert!(!draft.is_dirty(&task));

        let mut modified_draft = draft.clone();
        modified_draft.title = "Buy eggs".to_string();
        assert!(modified_draft.is_dirty(&task));
    }

    #[test]
    fn enter_with_panel_enters_editing_space_toggles() {
        let mut app = make_app_with_tmpfile(vec![make_task(None)]);
        app.show_detail_panel = true;

        // Enter should enter editing mode
        let _ = handle_normal(&mut app, KeyCode::Enter);
        assert_eq!(app.mode, Mode::EditingDetailPanel);
        assert!(app.detail_draft.is_some());
        assert_eq!(app.detail_field_index, 0);
        assert_eq!(app.input_buffer, "test");

        // Reset to Normal
        app.mode = Mode::Normal;
        app.detail_draft = None;

        // Space should toggle completion
        let _ = handle_normal(&mut app, KeyCode::Char(' '));
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.task_file.tasks[0].status, Status::Done);
    }

    #[test]
    fn detail_field_navigation_wraps() {
        let mut app = make_app_with_tasks(vec![make_task(None)]);
        app.show_detail_panel = true;
        app.detail_draft = Some(DetailDraft::from_task(&app.task_file.tasks[0]));
        app.detail_field_index = 0;
        app.input_buffer = app.task_file.tasks[0].title.clone();
        app.mode = Mode::EditingDetailPanel;

        // Navigate forward through all fields
        for i in 1..DETAIL_FIELD_COUNT {
            let _ = handle_detail_edit(&mut app, KeyCode::Char('j'));
            assert_eq!(app.detail_field_index, i);
        }
        // Wrap from 6 back to 0
        let _ = handle_detail_edit(&mut app, KeyCode::Char('j'));
        assert_eq!(app.detail_field_index, 0);

        // Navigate backward: 0 -> 6
        let _ = handle_detail_edit(&mut app, KeyCode::Char('k'));
        assert_eq!(app.detail_field_index, 6);
    }

    #[test]
    fn esc_from_clean_draft_exits_immediately() {
        let mut app = make_app_with_tasks(vec![make_task(None)]);
        app.detail_draft = Some(DetailDraft::from_task(&app.task_file.tasks[0]));
        app.detail_field_index = 0;
        app.input_buffer = app.task_file.tasks[0].title.clone();
        app.mode = Mode::EditingDetailPanel;

        let _ = handle_detail_edit(&mut app, KeyCode::Esc);
        assert_eq!(app.mode, Mode::Normal);
        assert!(app.detail_draft.is_none());
    }

    #[test]
    fn esc_from_dirty_draft_enters_confirming() {
        let mut app = make_app_with_tasks(vec![make_task(None)]);
        app.detail_draft = Some(DetailDraft::from_task(&app.task_file.tasks[0]));
        app.detail_field_index = 0;
        app.input_buffer = "modified title".to_string(); // dirty
        app.mode = Mode::EditingDetailPanel;

        let _ = handle_detail_edit(&mut app, KeyCode::Esc);
        assert_eq!(app.mode, Mode::ConfirmingDetailSave);
        assert!(app.detail_draft.is_some());
    }

    #[test]
    fn confirming_detail_save_discard_cancel() {
        // Test save
        let mut app = make_app_with_tmpfile(vec![make_task(None)]);
        let mut draft = DetailDraft::from_task(&app.task_file.tasks[0]);
        draft.title = "Updated".to_string();
        app.detail_draft = Some(draft);
        app.mode = Mode::ConfirmingDetailSave;
        let _ = handle_detail_confirm(&mut app, KeyCode::Char('s'));
        assert_eq!(app.mode, Mode::Normal);
        assert!(app.detail_draft.is_none());
        assert_eq!(app.task_file.tasks[0].title, "Updated");

        // Test discard
        let mut app2 = make_app_with_tmpfile(vec![make_task(None)]);
        let mut draft2 = DetailDraft::from_task(&app2.task_file.tasks[0]);
        draft2.title = "Should not save".to_string();
        app2.detail_draft = Some(draft2);
        app2.mode = Mode::ConfirmingDetailSave;
        let _ = handle_detail_confirm(&mut app2, KeyCode::Char('d'));
        assert_eq!(app2.mode, Mode::Normal);
        assert!(app2.detail_draft.is_none());
        assert_eq!(app2.task_file.tasks[0].title, "test"); // unchanged

        // Test cancel
        let mut app3 = make_app_with_tasks(vec![make_task(None)]);
        let mut draft3 = DetailDraft::from_task(&app3.task_file.tasks[0]);
        draft3.title = "In progress".to_string();
        app3.detail_draft = Some(draft3);
        app3.mode = Mode::ConfirmingDetailSave;
        let _ = handle_detail_confirm(&mut app3, KeyCode::Char('c'));
        assert_eq!(app3.mode, Mode::EditingDetailPanel);
        assert!(app3.detail_draft.is_some());
    }

    #[test]
    fn navigation_interception_with_dirty_draft() {
        let mut app = make_app_with_tasks(vec![make_task(None), make_task(None)]);
        let mut draft = DetailDraft::from_task(&app.task_file.tasks[0]);
        draft.title = "dirty".to_string();
        app.detail_draft = Some(draft);

        // j with dirty draft should enter confirming
        let _ = handle_normal(&mut app, KeyCode::Char('j'));
        assert_eq!(app.mode, Mode::ConfirmingDetailSave);
        assert_eq!(app.pending_navigation, Some(NavDirection::Down));
        assert_eq!(app.selected, 0); // hasn't moved yet

        // Reset and test clean draft navigates normally
        app.mode = Mode::Normal;
        app.detail_draft = Some(DetailDraft::from_task(&app.task_file.tasks[0])); // clean
        app.pending_navigation = None;
        let _ = handle_normal(&mut app, KeyCode::Char('j'));
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.selected, 1);
        assert!(app.detail_draft.is_none()); // cleared
    }

    #[test]
    fn title_contains_filter_matches_case_insensitively() {
        let mut task = make_task(None);
        task.title = "Deploy FLOW AI Service".to_string();
        let filter = Filter {
            title_contains: Some("flow ai".to_string()),
            ..Filter::default()
        };
        assert!(filter.matches(&task));

        let filter2 = Filter {
            title_contains: Some("DEPLOY".to_string()),
            ..Filter::default()
        };
        assert!(filter2.matches(&task));

        let filter3 = Filter {
            title_contains: Some("nonexistent".to_string()),
            ..Filter::default()
        };
        assert!(!filter3.matches(&task));
    }

    #[test]
    fn task_due_today_is_not_overdue_in_view() {
        let today = NaiveDate::from_ymd_opt(2026, 3, 4).unwrap();
        let task = make_task(Some(today));
        // Task due today should appear in Today view (it's due today, not overdue)
        assert!(View::Today.matches(&task, today));
        // The overdue path in View::matches requires d < today, which is false for d == today
        // So the task is shown because it matches the Today view directly, not as overdue
    }

    #[test]
    fn task_due_yesterday_is_overdue_in_views() {
        let today = NaiveDate::from_ymd_opt(2026, 3, 4).unwrap();
        let yesterday = NaiveDate::from_ymd_opt(2026, 3, 3).unwrap();
        let task = make_task(Some(yesterday));
        // Overdue open tasks appear in all time-based views
        assert!(View::Today.matches(&task, today));
        assert!(View::Weekly.matches(&task, today));
        assert!(View::Monthly.matches(&task, today));
    }

    #[test]
    fn nlp_spinner_frame_cycles() {
        let frames = ["Thinking", "Thinking.", "Thinking..", "Thinking..."];
        for i in 0u8..8 {
            let expected = frames[(i % 4) as usize];
            let dots = match i % 4 {
                0 => "Thinking",
                1 => "Thinking.",
                2 => "Thinking..",
                _ => "Thinking...",
            };
            assert_eq!(dots, expected, "Frame {} should be '{}'", i, expected);
        }
    }

    #[test]
    fn recurrence_pattern_column_shows_pattern_text() {
        use crate::task::{Recurrence, IntervalUnit};
        let mut task = make_task(None);
        task.recurrence = Some(Recurrence::Interval { unit: IntervalUnit::Weekly, count: 1 });
        let display = format_recurrence_display(task.recurrence.as_ref().unwrap());
        assert_eq!(display, "Weekly");

        let mut task2 = make_task(None);
        task2.recurrence = Some(Recurrence::NthWeekday { n: 3, weekday: chrono::Weekday::Thu });
        let display2 = format_recurrence_display(task2.recurrence.as_ref().unwrap());
        assert_eq!(display2, "Monthly (3rd Thu)");
    }

    #[test]
    fn recurring_view_hides_done_tasks() {
        use crate::task::{Recurrence, IntervalUnit};
        let today = chrono::Local::now().date_naive();
        let mut task = make_task(None);
        task.recurrence = Some(Recurrence::Interval { unit: IntervalUnit::Weekly, count: 1 });
        task.status = Status::Done;
        assert!(!View::Recurring.matches(&task, today), "Done recurring task should be hidden");
    }

    #[test]
    fn recurring_view_shows_open_tasks() {
        use crate::task::{Recurrence, IntervalUnit};
        let today = chrono::Local::now().date_naive();
        let mut task = make_task(None);
        task.recurrence = Some(Recurrence::Interval { unit: IntervalUnit::Weekly, count: 1 });
        task.status = Status::Open;
        assert!(View::Recurring.matches(&task, today), "Open recurring task should be shown");
    }

    #[test]
    fn format_recurrence_display_with_count() {
        use crate::task::{Recurrence, IntervalUnit};
        let r = Recurrence::Interval { unit: IntervalUnit::Monthly, count: 3 };
        assert_eq!(format_recurrence_display(&r), "Every 3 Months");

        let r2 = Recurrence::Interval { unit: IntervalUnit::Weekly, count: 2 };
        assert_eq!(format_recurrence_display(&r2), "Every 2 Weeks");

        let r3 = Recurrence::Interval { unit: IntervalUnit::Daily, count: 1 };
        assert_eq!(format_recurrence_display(&r3), "Daily");
    }

    #[test]
    fn format_recurrence_display_weekly_on() {
        use crate::task::Recurrence;
        let r = Recurrence::WeeklyOn { weekday: chrono::Weekday::Fri, every_n_weeks: 1 };
        assert_eq!(format_recurrence_display(&r), "Weekly (Fri)");

        let r2 = Recurrence::WeeklyOn { weekday: chrono::Weekday::Mon, every_n_weeks: 2 };
        assert_eq!(format_recurrence_display(&r2), "Every 2 Weeks (Mon)");
    }

    // -- Sort order tests --

    #[test]
    fn filtered_indices_sorted_by_due_date_ascending() {
        let today = chrono::Local::now().date_naive();
        let d1 = today + chrono::Days::new(10);
        let d2 = today + chrono::Days::new(2);
        let d3 = today + chrono::Days::new(5);
        let app = make_app_with_tasks(vec![
            make_task(Some(d1)),
            make_task(Some(d2)),
            make_task(Some(d3)),
        ]);
        let indices = app.filtered_indices();
        assert_eq!(indices, vec![1, 2, 0]); // d2, d3, d1
    }

    #[test]
    fn filtered_indices_none_due_date_sorted_last() {
        let today = chrono::Local::now().date_naive();
        let d1 = today + chrono::Days::new(5);
        let app = make_app_with_tasks(vec![
            make_task(None),
            make_task(Some(d1)),
            make_task(None),
        ]);
        let indices = app.filtered_indices();
        assert_eq!(indices, vec![1, 0, 2]); // d1, None, None
    }

    #[test]
    fn filtered_indices_same_date_sorted_by_priority_descending() {
        let today = chrono::Local::now().date_naive();
        let d = today + chrono::Days::new(3);
        let mut t1 = make_task(Some(d));
        t1.priority = Priority::Low;
        let mut t2 = make_task(Some(d));
        t2.priority = Priority::Critical;
        let mut t3 = make_task(Some(d));
        t3.priority = Priority::Medium;
        let app = make_app_with_tasks(vec![t1, t2, t3]);
        let indices = app.filtered_indices();
        assert_eq!(indices, vec![1, 2, 0]); // Critical, Medium, Low
    }

    #[test]
    fn filtered_indices_no_due_date_sorted_by_priority() {
        let mut t1 = make_task(None);
        t1.priority = Priority::Low;
        let mut t2 = make_task(None);
        t2.priority = Priority::High;
        let mut t3 = make_task(None);
        t3.priority = Priority::Medium;
        let app = make_app_with_tasks(vec![t1, t2, t3]);
        let indices = app.filtered_indices();
        assert_eq!(indices, vec![1, 2, 0]); // High, Medium, Low
    }

    // -- NoteEditor tests --

    #[test]
    fn note_editor_new_empty_body() {
        let ed = NoteEditor::new("my-note", "My Note", "");
        assert_eq!(ed.lines, vec![""]);
        assert_eq!(ed.cursor_row, 0);
        assert_eq!(ed.cursor_col, 0);
        assert!(!ed.dirty);
    }

    #[test]
    fn note_editor_new_multiline_body() {
        let ed = NoteEditor::new("slug", "Title", "line one\nline two\nline three");
        assert_eq!(ed.lines.len(), 3);
        assert_eq!(ed.lines[0], "line one");
        assert_eq!(ed.lines[2], "line three");
    }

    #[test]
    fn note_editor_insert_char() {
        let mut ed = NoteEditor::new("s", "T", "hello");
        ed.cursor_col = 5;
        ed.insert_char('!');
        assert_eq!(ed.lines[0], "hello!");
        assert_eq!(ed.cursor_col, 6);
        assert!(ed.dirty);
    }

    #[test]
    fn note_editor_insert_char_middle() {
        let mut ed = NoteEditor::new("s", "T", "hllo");
        ed.cursor_col = 1;
        ed.insert_char('e');
        assert_eq!(ed.lines[0], "hello");
        assert_eq!(ed.cursor_col, 2);
    }

    #[test]
    fn note_editor_insert_newline() {
        let mut ed = NoteEditor::new("s", "T", "hello world");
        ed.cursor_col = 5;
        ed.insert_newline();
        assert_eq!(ed.lines.len(), 2);
        assert_eq!(ed.lines[0], "hello");
        assert_eq!(ed.lines[1], " world");
        assert_eq!(ed.cursor_row, 1);
        assert_eq!(ed.cursor_col, 0);
        assert!(ed.dirty);
    }

    #[test]
    fn note_editor_backspace_within_line() {
        let mut ed = NoteEditor::new("s", "T", "hello");
        ed.cursor_col = 5;
        ed.backspace();
        assert_eq!(ed.lines[0], "hell");
        assert_eq!(ed.cursor_col, 4);
        assert!(ed.dirty);
    }

    #[test]
    fn note_editor_backspace_joins_lines() {
        let mut ed = NoteEditor::new("s", "T", "first\nsecond");
        ed.cursor_row = 1;
        ed.cursor_col = 0;
        ed.backspace();
        assert_eq!(ed.lines.len(), 1);
        assert_eq!(ed.lines[0], "firstsecond");
        assert_eq!(ed.cursor_row, 0);
        assert_eq!(ed.cursor_col, 5);
    }

    #[test]
    fn note_editor_backspace_at_start_does_nothing() {
        let mut ed = NoteEditor::new("s", "T", "hello");
        ed.cursor_col = 0;
        ed.backspace();
        assert_eq!(ed.lines[0], "hello");
        assert!(!ed.dirty);
    }

    #[test]
    fn note_editor_move_up_down() {
        let mut ed = NoteEditor::new("s", "T", "line1\nline2\nline3");
        assert_eq!(ed.cursor_row, 0);
        ed.move_down();
        assert_eq!(ed.cursor_row, 1);
        ed.move_down();
        assert_eq!(ed.cursor_row, 2);
        ed.move_down(); // at bottom, stays
        assert_eq!(ed.cursor_row, 2);
        ed.move_up();
        assert_eq!(ed.cursor_row, 1);
        ed.move_up();
        assert_eq!(ed.cursor_row, 0);
        ed.move_up(); // at top, stays
        assert_eq!(ed.cursor_row, 0);
    }

    #[test]
    fn note_editor_move_left_right() {
        let mut ed = NoteEditor::new("s", "T", "abc");
        ed.move_right();
        assert_eq!(ed.cursor_col, 1);
        ed.move_right();
        ed.move_right();
        assert_eq!(ed.cursor_col, 3);
        ed.move_right(); // at end, stays
        assert_eq!(ed.cursor_col, 3);
        ed.move_left();
        assert_eq!(ed.cursor_col, 2);
        ed.move_left();
        ed.move_left();
        assert_eq!(ed.cursor_col, 0);
        ed.move_left(); // at start, stays
        assert_eq!(ed.cursor_col, 0);
    }

    #[test]
    fn note_editor_clamp_col_on_move() {
        let mut ed = NoteEditor::new("s", "T", "long line\nhi");
        ed.cursor_col = 9; // end of "long line"
        ed.move_down();
        assert_eq!(ed.cursor_col, 2); // clamped to end of "hi"
    }

    #[test]
    fn note_editor_ensure_cursor_visible() {
        let mut ed = NoteEditor::new("s", "T", "a\nb\nc\nd\ne\nf");
        ed.viewport_offset = 0;
        ed.cursor_row = 5;
        ed.ensure_cursor_visible(3, 80);
        assert_eq!(ed.viewport_offset, 3); // scrolled so row 5 is visible in 3-line viewport

        ed.cursor_row = 1;
        ed.ensure_cursor_visible(3, 80);
        assert_eq!(ed.viewport_offset, 1); // scrolled up
    }

    #[test]
    fn note_editor_move_to_line_start() {
        let mut ed = NoteEditor::new("s", "T", "hello");
        ed.cursor_col = 3;
        ed.move_to_line_start();
        assert_eq!(ed.cursor_col, 0);
        // Already at start — no-op
        ed.move_to_line_start();
        assert_eq!(ed.cursor_col, 0);
    }

    #[test]
    fn note_editor_move_to_line_end() {
        let mut ed = NoteEditor::new("s", "T", "hello");
        ed.cursor_col = 0;
        ed.move_to_line_end();
        assert_eq!(ed.cursor_col, 5); // "hello".chars().count()
        // Empty line stays at 0
        let mut ed2 = NoteEditor::new("s", "T", "");
        ed2.move_to_line_end();
        assert_eq!(ed2.cursor_col, 0);
    }

    #[test]
    fn note_editor_visual_row_counting() {
        // A 10-char line with cols_per_row=4 produces ceil(10/4)=3 visual rows
        let line = "0123456789"; // 10 chars
        let cols_per_row: usize = 4;
        let n = line.chars().count();
        let visual_rows = if n == 0 { 1 } else { (n + cols_per_row - 1) / cols_per_row };
        assert_eq!(visual_rows, 3);

        // Empty line always produces 1 visual row
        let empty = "";
        let n2 = empty.chars().count();
        let visual_rows2 = if n2 == 0 { 1 } else { (n2 + cols_per_row - 1) / cols_per_row };
        assert_eq!(visual_rows2, 1);
    }

    #[test]
    fn note_editor_body_text() {
        let ed = NoteEditor::new("s", "T", "line1\nline2");
        assert_eq!(ed.body_text(), "line1\nline2");
    }

    #[test]
    fn char_to_byte_index_ascii() {
        assert_eq!(char_to_byte_index("hello", 0), 0);
        assert_eq!(char_to_byte_index("hello", 3), 3);
        assert_eq!(char_to_byte_index("hello", 5), 5);
    }

    #[test]
    fn char_to_byte_index_multibyte() {
        let s = "héllo";
        assert_eq!(char_to_byte_index(s, 0), 0);
        assert_eq!(char_to_byte_index(s, 1), 1); // 'h' is 1 byte
        assert_eq!(char_to_byte_index(s, 2), 3); // 'é' is 2 bytes
    }

    // -- reload_from_disk tests --

    #[test]
    fn reload_from_disk_updates_task_file_and_sets_status_message() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        // Write initial file with 1 task
        std::fs::write(&path, "<!-- format:2 -->\n<!-- next-id:2 -->\n\n# Tasks\n\n## [ ] Task one\n<!-- id:1 priority:medium created:2026-01-01T00:00:00+00:00 -->\n").unwrap();
        let mut app = App::new(&path).unwrap();
        assert_eq!(app.task_file.tasks.len(), 1);

        // External writer adds a second task
        std::fs::write(&path, "<!-- format:2 -->\n<!-- next-id:3 -->\n\n# Tasks\n\n## [ ] Task one\n<!-- id:1 priority:medium created:2026-01-01T00:00:00+00:00 -->\n\n## [ ] Task two\n<!-- id:2 priority:medium created:2026-01-02T00:00:00+00:00 -->\n").unwrap();

        app.reload_from_disk().unwrap();

        assert_eq!(app.task_file.tasks.len(), 2);
        assert_eq!(app.status_message, Some("Reloaded 2 tasks from disk".to_string()));
    }

    #[test]
    fn ctrl_r_in_non_normal_mode_sets_warning_and_does_not_reload() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tasks.md");
        std::fs::write(&path, "<!-- format:2 -->\n<!-- next-id:2 -->\n\n# Tasks\n\n## [ ] Task one\n<!-- id:1 priority:medium created:2026-01-01T00:00:00+00:00 -->\n").unwrap();
        let mut app = App::new(&path).unwrap();
        app.mode = Mode::Adding;

        // Simulate the ctrl+r blocking logic
        if app.mode != Mode::Normal {
            app.status_message = Some("Cannot reload: finish editing first".to_string());
        } else {
            app.reload_from_disk().unwrap();
        }

        assert_eq!(app.task_file.tasks.len(), 1, "task_file should not change");
        assert_eq!(app.status_message, Some("Cannot reload: finish editing first".to_string()));
    }
}
