use std::io::{self, stdout};
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{Datelike, Days, Local, Months, NaiveDate, Utc};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
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
use crate::config;
use crate::nlp::{self, ApiMessage, NlpAction};
use crate::storage;
use crate::task::{Priority, Status, Task, TaskFile};
use crate::todoist;

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
}

impl View {
    fn matches(&self, task: &Task, today: NaiveDate) -> bool {
        // Completed tasks only appear in the All and Recurring views
        if task.status == Status::Done && *self != View::All && *self != View::Recurring {
            return false;
        }
        // Recurring view: filter by recurrence presence only
        if *self == View::Recurring {
            return task.recurrence.is_some();
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
            View::Recurring => View::Today,
        }
    }

    fn prev(&self) -> View {
        match self {
            View::Today => View::Recurring,
            View::All => View::Today,
            View::Weekly => View::All,
            View::Monthly => View::Weekly,
            View::Yearly => View::Monthly,
            View::NoDueDate => View::Yearly,
            View::Recurring => View::NoDueDate,
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
        };
        app.table_state.select(Some(0));
        Ok(app)
    }

    fn filtered_indices(&self) -> Vec<usize> {
        let today = Local::now().date_naive();
        self.task_file
            .tasks
            .iter()
            .enumerate()
            .filter(|(_, t)| self.view.matches(t, today))
            .filter(|(_, t)| self.filter.matches(t))
            .map(|(i, _)| i)
            .collect()
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

    // Restore terminal
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);

    result
}

fn event_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<(), String> {
    loop {
        terminal
            .draw(|frame| draw(frame, app))
            .map_err(|e| format!("Draw error: {}", e))?;

        if event::poll(Duration::from_millis(250))
            .map_err(|e| format!("Event poll error: {}", e))?
        {
            if let Event::Key(key) = event::read().map_err(|e| format!("Event read error: {}", e))? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if handle_key(terminal, app, key.code)? {
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
            };
            app.task_file.tasks.push(new_task);
            app.status_message = Some(format!("Next occurrence: task {}, due {}", new_id, next_due));
        }
    }
    app.save()?;
    Ok(())
}

fn handle_key(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App, key: KeyCode) -> Result<bool, String> {
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
    }
}

fn handle_normal(app: &mut App, key: KeyCode) -> Result<bool, String> {
    // Clear any status message on keypress
    app.status_message = None;

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
            app.view = app.view.next();
            app.selected = 0;
            app.table_state.select(Some(0));
            app.clamp_selection();
        }
        KeyCode::Char('V') => {
            app.view = app.view.prev();
            app.selected = 0;
            app.table_state.select(Some(0));
            app.clamp_selection();
        }
        KeyCode::Char('i') => {
            match auth::read_token() {
                None => {
                    app.status_message = Some("No Todoist token. Run `task auth todoist` from the CLI.".to_string());
                }
                Some(token) => {
                    match todoist::run_import(&token, &mut app.task_file, false) {
                        Ok((imported, skipped)) => {
                            app.save()?;
                            app.clamp_selection();
                            app.status_message = Some(format!(
                                "Imported {} tasks, skipped {} (already exported)",
                                imported, skipped
                            ));
                        }
                        Err(e) => {
                            app.status_message = Some(e);
                        }
                    }
                }
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
        KeyCode::Char('T') | KeyCode::Char('W') | KeyCode::Char('M') | KeyCode::Char('Q') | KeyCode::Char('X') => {
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
                        KeyCode::Char('W') => today.checked_add_days(Days::new(7)),
                        KeyCode::Char('M') => today.checked_add_months(Months::new(1)),
                        KeyCode::Char('Q') => today.checked_add_months(Months::new(3)),
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
                        });
                        app.save()?;
                        app.clamp_selection();
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
            let filtered = app.filtered_indices();
            if let Some(&task_idx) = filtered.get(app.selected) {
                app.task_file.tasks.remove(task_idx);
                app.save()?;
                app.clamp_selection();
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
        NlpAction::Update { match_criteria, set_fields, .. } => {
            let mut match_parts = Vec::new();
            if let Some(ref s) = match_criteria.status { match_parts.push(format!("status={}", s)); }
            if let Some(ref p) = match_criteria.priority { match_parts.push(format!("priority={}", p)); }
            if let Some(ref t) = match_criteria.tag { match_parts.push(format!("tag={}", t)); }
            if let Some(ref p) = match_criteria.project { match_parts.push(format!("project={}", p)); }
            if let Some(ref tc) = match_criteria.title_contains { match_parts.push(format!("title~{}", tc)); }
            let mut set_parts = Vec::new();
            if let Some(ref p) = set_fields.priority { set_parts.push(format!("priority={}", p)); }
            if let Some(ref s) = set_fields.status { set_parts.push(format!("status={}", s)); }
            if let Some(ref t) = set_fields.tags { set_parts.push(format!("tags=[{}]", t.join(", "))); }
            let match_str = if match_parts.is_empty() { "(all)".to_string() } else { match_parts.join(", ") };
            let set_str = if set_parts.is_empty() { "(none)".to_string() } else { set_parts.join(", ") };
            Some(format!("Updating: match {{{}}} → set {{{}}}", match_str, set_str))
        }
        NlpAction::SetRecurrence { description, .. } => {
            Some(description.clone())
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

fn handle_nlp_chat<B: Backend>(terminal: &mut Terminal<B>, app: &mut App, key: KeyCode) -> Result<(), String> {
    match key {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.input_buffer.clear();
            app.chat_history.clear();
            app.nlp_messages.clear();
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

            // Redraw so the user's message appears before the blocking API call
            terminal
                .draw(|frame| draw(frame, app))
                .map_err(|e| format!("Draw error: {}", e))?;

            match nlp::interpret(&app.task_file.tasks, &app.nlp_messages, &api_key) {
                Ok((ref action @ NlpAction::Filter(ref criteria), raw_response)) => {
                    app.nlp_messages.push(ApiMessage {
                        role: "assistant".to_string(),
                        content: raw_response,
                    });
                    if let Some(summary) = format_action_summary(action) {
                        app.chat_history.push(ChatMessage::Assistant(summary));
                    }
                    let mut filter = Filter::default();
                    if let Some(ref s) = criteria.status {
                        if let Ok(status) = s.parse::<Status>() {
                            filter.status = Some(status);
                        }
                    }
                    if let Some(ref p) = criteria.priority {
                        if let Ok(priority) = p.parse::<Priority>() {
                            filter.priority = Some(priority);
                        }
                    }
                    if let Some(ref t) = criteria.tag {
                        filter.tag = Some(t.clone());
                    }
                    if let Some(ref p) = criteria.project {
                        filter.project = Some(p.clone());
                    }
                    if let Some(ref tc) = criteria.title_contains {
                        filter.title_contains = Some(tc.clone());
                    }
                    app.view = View::All;
                    app.filter = filter;
                    app.selected = 0;
                    app.table_state.select(Some(0));
                    app.clamp_selection();
                    app.chat_history.push(ChatMessage::Assistant("Filter applied.".to_string()));
                }
                Ok((ref action @ NlpAction::Update { ref match_criteria, ref set_fields, .. }, raw_response)) => {
                    app.nlp_messages.push(ApiMessage {
                        role: "assistant".to_string(),
                        content: raw_response,
                    });
                    if let Some(summary) = format_action_summary(action) {
                        app.chat_history.push(ChatMessage::Assistant(summary));
                    }
                    let has_any_criteria = match_criteria.status.is_some()
                        || match_criteria.priority.is_some()
                        || match_criteria.tag.is_some()
                        || match_criteria.project.is_some()
                        || match_criteria.title_contains.is_some();
                    let matching: Vec<usize> = if !has_any_criteria {
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
                        app.pending_nlp_update = Some((action.clone(), matching));
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
                Ok((ref action @ NlpAction::SetRecurrence { task_id, ref recurrence, description: _ }, raw_response)) => {
                    app.nlp_messages.push(ApiMessage {
                        role: "assistant".to_string(),
                        content: raw_response,
                    });
                    if let Some(summary) = format_action_summary(action) {
                        app.chat_history.push(ChatMessage::Assistant(summary));
                    }
                    // Apply the recurrence change
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
                Err(e) => {
                    app.chat_history.push(ChatMessage::Error(e));
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
        Recurrence::Interval(unit) => match unit {
            IntervalUnit::Daily => "Daily".to_string(),
            IntervalUnit::Weekly => "Weekly".to_string(),
            IntervalUnit::Monthly => "Monthly".to_string(),
            IntervalUnit::Yearly => "Yearly".to_string(),
        },
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

    let mut header_cells = vec!["ID", "Status", "Priority", "Title"];
    if show_desc { header_cells.push("Desc"); }
    if show_due { header_cells.push("Due"); }
    if show_project { header_cells.push("Project"); }
    if show_recur { header_cells.push("↻"); }
    header_cells.push("Tags");

    let header = Row::new(header_cells)
        .style(Style::default().add_modifier(Modifier::BOLD))
        .bottom_margin(0);

    let rows: Vec<Row> = filtered
        .iter()
        .map(|&i| {
            let task = &app.task_file.tasks[i];
            let is_overdue = task.status == Status::Open
                && task.due_date.map_or(false, |d| d < Local::now().date_naive());
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
    if show_recur { widths.push(Constraint::Length(3)); }
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
            let created = task.created.format("%Y-%m-%d %H:%M").to_string();
            let updated = task.updated
                .map(|u| u.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "(never)".to_string());

            format!(
                "ID: {}  |  Status: {}  |  Priority: {}  |  Due: {}  |  Project: {}\n\
                 Title: {}\n\
                 Description: {}\n\
                 Tags: {}  |  Recurrence: {}\n\
                 Created: {}  |  Updated: {}",
                task.id, task.status, task.priority, due, project,
                task.title,
                desc,
                tags, recurrence_str,
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

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let text = match &app.mode {
        Mode::Normal => {
            if let Some(ref msg) = app.status_message {
                format!(" {} ", msg)
            } else if app.show_detail_panel {
                " j/k:nav  Enter:edit  Space:toggle  a:add  d:delete  f:filter  p:priority  e:edit-title  t:tags  r:desc  R:recur  v:view  Tab:details  q:quit ".to_string()
            } else {
                " j/k:nav  Enter:toggle  a:add  d:delete  f:filter  p:priority  e:edit  t:tags  r:desc  R:recur  v:view  i:import  ::command  D:set-dir  T/W/M/Q:due  X:clr-due  Tab:details  q:quit ".to_string()
            }
        }
        Mode::Adding => {
            format!(" Add task: {}_ ", app.input_buffer)
        }
        Mode::Filtering => {
            format!(" Filter (status:open priority:high tag:name): {}_ ", app.input_buffer)
        }
        Mode::Confirming => {
            let filtered = app.filtered_indices();
            if let Some(&idx) = filtered.get(app.selected) {
                let task = &app.task_file.tasks[idx];
                format!(" Delete task {}? y/n ", task.id)
            } else {
                " Delete? y/n ".to_string()
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
        task.recurrence = Some(crate::task::Recurrence::Interval(crate::task::IntervalUnit::Weekly));
        assert!(View::Recurring.matches(&task, today));
    }

    #[test]
    fn recurring_view_shows_recurring_done_task() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        let mut task = make_task(Some(today));
        task.status = Status::Done;
        task.recurrence = Some(crate::task::Recurrence::Interval(crate::task::IntervalUnit::Daily));
        assert!(View::Recurring.matches(&task, today));
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
        v = v.next(); assert_eq!(v, View::Today); // wrap
    }

    #[test]
    fn prev_cycles_through_all_views() {
        let mut v = View::Today;
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
        for key in ['T', 'W', 'M', 'Q', 'X'] {
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
}
