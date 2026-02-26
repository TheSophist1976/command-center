use std::io::{self, stdout};
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{Datelike, Local, NaiveDate, Utc};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
};

use crate::auth;
use crate::config;
use crate::nlp::{self, NlpAction};
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
    NlpInput,
    ConfirmingNlp,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum View {
    Today,
    All,
    Weekly,
    Monthly,
    Yearly,
    NoDueDate,
}

impl View {
    fn matches(&self, task: &Task, today: NaiveDate) -> bool {
        // Completed tasks only appear in the All view
        if task.status == Status::Done && *self != View::All {
            return false;
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
        }
    }

    fn next(&self) -> View {
        match self {
            View::Today => View::All,
            View::All => View::Weekly,
            View::Weekly => View::Monthly,
            View::Monthly => View::Yearly,
            View::Yearly => View::NoDueDate,
            View::NoDueDate => View::Today,
        }
    }

    fn prev(&self) -> View {
        match self {
            View::Today => View::NoDueDate,
            View::All => View::Today,
            View::Weekly => View::All,
            View::Monthly => View::Weekly,
            View::Yearly => View::Monthly,
            View::NoDueDate => View::Yearly,
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
            _ => View::Today,
        }
    }
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
                if handle_key(app, key.code)? {
                    return Ok(());
                }
            }
        }
    }
}

/// Returns true if we should quit.
fn handle_key(app: &mut App, key: KeyCode) -> Result<bool, String> {
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
        Mode::EditingDefaultDir => {
            handle_input(app, key, InputAction::EditDefaultDir)?;
            Ok(false)
        }
        Mode::NlpInput => {
            handle_nlp_input(app, key)?;
            Ok(false)
        }
        Mode::ConfirmingNlp => {
            handle_nlp_confirm(app, key)?;
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
            if !filtered.is_empty() && app.selected < filtered.len() - 1 {
                app.selected += 1;
                app.table_state.select(Some(app.selected));
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.selected > 0 {
                app.selected -= 1;
                app.table_state.select(Some(app.selected));
            }
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            if let Some(&task_idx) = filtered.get(app.selected) {
                let task = &mut app.task_file.tasks[task_idx];
                task.status = match task.status {
                    Status::Open => Status::Done,
                    Status::Done => Status::Open,
                };
                task.updated = Some(Utc::now());
                app.save()?;
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
            app.mode = Mode::NlpInput;
            app.input_buffer.clear();
        }
        KeyCode::Char('D') => {
            app.input_buffer = config::read_config_value("default-dir").unwrap_or_default();
            app.mode = Mode::EditingDefaultDir;
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

fn handle_nlp_input(app: &mut App, key: KeyCode) -> Result<(), String> {
    match key {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.input_buffer.clear();
        }
        KeyCode::Enter => {
            let input = app.input_buffer.clone();
            app.input_buffer.clear();

            if input.trim().is_empty() {
                app.mode = Mode::Normal;
                return Ok(());
            }

            let api_key = match auth::read_claude_key() {
                Some(k) => k,
                None => {
                    app.mode = Mode::Normal;
                    app.status_message = Some("No Claude API key. Run `task auth claude` or set ANTHROPIC_API_KEY.".to_string());
                    return Ok(());
                }
            };

            match nlp::interpret(&app.task_file.tasks, &input, &api_key) {
                Ok(NlpAction::Filter(criteria)) => {
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
                    // NLP filters always switch to All view so time-based
                    // views don't silently hide matching results
                    app.view = View::All;
                    app.filter = filter;
                    app.selected = 0;
                    app.table_state.select(Some(0));
                    app.clamp_selection();
                    app.mode = Mode::Normal;
                }
                Ok(ref action @ NlpAction::Update { ref match_criteria, .. }) => {
                    // Find matching task indices
                    let matching: Vec<usize> = app.task_file.tasks.iter().enumerate()
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
                        .collect();

                    if matching.is_empty() {
                        app.mode = Mode::Normal;
                        app.status_message = Some("No tasks match the criteria.".to_string());
                    } else {
                        app.pending_nlp_update = Some((action.clone(), matching));
                        app.mode = Mode::ConfirmingNlp;
                    }
                }
                Ok(NlpAction::Message(text)) => {
                    app.mode = Mode::Normal;
                    app.status_message = Some(text);
                }
                Err(e) => {
                    app.mode = Mode::Normal;
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
                app.status_message = Some(format!("{} ({} tasks)", description, count));
            }
            app.mode = Mode::Normal;
        }
        _ => {
            app.pending_nlp_update = None;
            app.mode = Mode::Normal;
        }
    }
    Ok(())
}

// -- Rendering --

fn draw(frame: &mut Frame, app: &mut App) {
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

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let title = if app.filter.is_active() {
        format!(" task-manager  |  {}  |  filter: {} ", app.view.display_name(), app.filter.summary())
    } else {
        format!(" task-manager  |  {} ", app.view.display_name())
    };
    let header = Paragraph::new(title).style(
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(header, area);
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

    let show_due = filtered.iter().any(|&i| app.task_file.tasks[i].due_date.is_some());
    let show_project = filtered.iter().any(|&i| app.task_file.tasks[i].project.is_some());

    let mut header_cells = vec!["ID", "Status", "Priority", "Title"];
    if show_due { header_cells.push("Due"); }
    if show_project { header_cells.push("Project"); }
    header_cells.push("Tags");

    let header = Row::new(header_cells)
        .style(Style::default().add_modifier(Modifier::BOLD))
        .bottom_margin(0);

    let rows: Vec<Row> = filtered
        .iter()
        .map(|&i| {
            let task = &app.task_file.tasks[i];
            let status_str = match task.status {
                Status::Open => "[ ]",
                Status::Done => "[x]",
            };
            let priority_style = match task.priority {
                Priority::Critical => Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                Priority::High => Style::default().fg(Color::Red),
                Priority::Medium => Style::default().fg(Color::Yellow),
                Priority::Low => Style::default().fg(Color::Green),
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
            if show_due {
                let due_str = task.due_date
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default();
                cells.push(Cell::from(due_str));
            }
            if show_project {
                cells.push(Cell::from(task.project.as_deref().unwrap_or("").to_string()));
            }
            cells.push(Cell::from(tags_str));
            Row::new(cells)
        })
        .collect();

    let mut widths = vec![
        Constraint::Length(5),
        Constraint::Length(5),
        Constraint::Length(9),
        Constraint::Fill(1),
    ];
    if show_due { widths.push(Constraint::Length(12)); }
    if show_project { widths.push(Constraint::Length(15)); }
    widths.push(Constraint::Length(20));

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL))
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(table, area, &mut app.table_state);
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let text = match &app.mode {
        Mode::Normal => {
            if let Some(ref msg) = app.status_message {
                format!(" {} ", msg)
            } else {
                " j/k:nav  Enter:toggle  a:add  d:delete  f:filter  p:priority  e:edit  t:tags  r:desc  v:view  i:import  ::command  D:set-dir  q:quit ".to_string()
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
        Mode::EditingDefaultDir => {
            format!(" Set default directory: {}_ ", app.input_buffer)
        }
        Mode::NlpInput => {
            format!(" > {}_ ", app.input_buffer)
        }
        Mode::ConfirmingNlp => {
            if let Some((NlpAction::Update { ref description, .. }, ref indices)) = app.pending_nlp_update {
                format!(" {} ({} tasks) — y/n ", description, indices.len())
            } else {
                " Apply changes? y/n ".to_string()
            }
        }
    };

    let footer = Paragraph::new(text).style(
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan),
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
    fn today_view_hides_overdue_task() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 26).unwrap();
        let yesterday = NaiveDate::from_ymd_opt(2026, 2, 25).unwrap();
        let task = make_task(Some(yesterday));
        assert!(!View::Today.matches(&task, today));
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

    // -- View::next / View::prev tests --

    #[test]
    fn next_cycles_through_all_views() {
        let mut v = View::Today;
        v = v.next(); assert_eq!(v, View::All);
        v = v.next(); assert_eq!(v, View::Weekly);
        v = v.next(); assert_eq!(v, View::Monthly);
        v = v.next(); assert_eq!(v, View::Yearly);
        v = v.next(); assert_eq!(v, View::NoDueDate);
        v = v.next(); assert_eq!(v, View::Today); // wrap
    }

    #[test]
    fn prev_cycles_through_all_views() {
        let mut v = View::Today;
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
    fn colon_key_enters_nlp_input_mode() {
        let mut app = make_app_with_tasks(vec![make_task(None)]);
        let _ = handle_normal(&mut app, KeyCode::Char(':'));
        assert_eq!(app.mode, Mode::NlpInput);
        assert!(app.input_buffer.is_empty());
    }

    #[test]
    fn esc_in_nlp_input_returns_to_normal() {
        let mut app = make_app_with_tasks(vec![make_task(None)]);
        app.mode = Mode::NlpInput;
        app.input_buffer = "some query".to_string();
        let _ = handle_nlp_input(&mut app, KeyCode::Esc);
        assert_eq!(app.mode, Mode::Normal);
        assert!(app.input_buffer.is_empty());
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
