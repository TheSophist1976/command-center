mod auth;
mod cli;
mod config;
mod output;
mod parser;
mod storage;
mod task;
mod todoist;
mod tui;

use std::process;
use std::str::FromStr;

use chrono::{NaiveDate, Utc};
use clap::Parser;

use cli::{AuthCommand, Cli, Command, ConfigCommand, ImportCommand};
use task::{Priority, Status, Task};

fn main() {
    let cli = Cli::parse();
    let json = cli.json;

    let code = match run(cli) {
        Ok(()) => 0,
        Err((code, msg)) => {
            output::print_error(&msg, json);
            code
        }
    };
    process::exit(code);
}

fn run(cli: Cli) -> Result<(), (i32, String)> {
    let json = cli.json;
    let strict = cli.strict;
    let path = storage::resolve_file_path(cli.file.as_deref());

    match cli.command {
        Command::Init => {
            storage::init_file(&path).map_err(|e| (1, e))?;
            output::print_success(
                &format!("Initialized {}", path.display()),
                json,
            );
            Ok(())
        }

        Command::Add { title, priority, tags, due, project } => {
            let priority = match priority {
                Some(p) => Priority::from_str(&p).map_err(|e| (1, e))?,
                None => Priority::Medium,
            };
            let tags = validate_and_parse_tags(tags.as_deref())?;
            let due_date = parse_due_date(due.as_deref())?;

            let mut tf = storage::load(&path, strict).map_err(|e| (1, e))?;
            let id = tf.next_id;
            tf.next_id += 1;

            tf.tasks.push(Task {
                id,
                title: title.clone(),
                status: Status::Open,
                priority,
                tags,
                created: Utc::now(),
                updated: None,
                description: None,
                due_date,
                project,
            });

            storage::save(&path, &tf).map_err(|e| (1, e))?;
            output::print_success(&format!("Added task {}: {}", id, title), json);
            Ok(())
        }

        Command::List { status, priority, tag, project } => {
            let tf = storage::load(&path, strict).map_err(|e| (1, e))?;

            let status_filter = match status {
                Some(s) => Some(Status::from_str(&s).map_err(|e| (1, e))?),
                None => None,
            };
            let priority_filter = match priority {
                Some(p) => Some(Priority::from_str(&p).map_err(|e| (1, e))?),
                None => None,
            };

            let filtered: Vec<&Task> = tf.tasks.iter().filter(|t| {
                if let Some(s) = status_filter {
                    if t.status != s { return false; }
                }
                if let Some(p) = priority_filter {
                    if t.priority != p { return false; }
                }
                if let Some(ref tag_filter) = tag {
                    if !t.tags.iter().any(|tg| tg == tag_filter) { return false; }
                }
                if let Some(ref proj_filter) = project {
                    match &t.project {
                        Some(p) => if p != proj_filter { return false; },
                        None => return false,
                    }
                }
                true
            }).collect();

            let owned: Vec<Task> = filtered.into_iter().cloned().collect();
            output::print_task_table(&owned, json);
            Ok(())
        }

        Command::Show { id } => {
            let tf = storage::load(&path, strict).map_err(|e| (1, e))?;
            match tf.find_task(id) {
                Some(task) => {
                    output::print_task_detail(task, json);
                    Ok(())
                }
                None => Err((2, format!("Task {} not found", id))),
            }
        }

        Command::Edit { id, title, priority, tags, due, project } => {
            if title.is_none() && priority.is_none() && tags.is_none() && due.is_none() && project.is_none() {
                return Err((1, "Nothing to edit. Provide --title, --priority, --tags, --due, or --project.".to_string()));
            }

            let mut tf = storage::load(&path, strict).map_err(|e| (1, e))?;
            let task = tf.find_task_mut(id)
                .ok_or((2, format!("Task {} not found", id)))?;

            if let Some(new_title) = title {
                task.title = new_title;
            }
            if let Some(p) = priority {
                task.priority = Priority::from_str(&p).map_err(|e| (1, e))?;
            }
            if let Some(t) = tags {
                task.tags = validate_and_parse_tags(Some(&t))?;
            }
            if due.is_some() {
                task.due_date = parse_due_date(due.as_deref())?;
            }
            if let Some(proj) = project {
                task.project = Some(proj);
            }
            task.updated = Some(Utc::now());

            let task_title = task.title.clone();
            storage::save(&path, &tf).map_err(|e| (1, e))?;
            output::print_success(&format!("Updated task {}: {}", id, task_title), json);
            Ok(())
        }

        Command::Done { id } => {
            let mut tf = storage::load(&path, strict).map_err(|e| (1, e))?;
            let task = tf.find_task_mut(id)
                .ok_or((2, format!("Task {} not found", id)))?;

            if task.status == Status::Done {
                output::print_success(&format!("Task {} is already done", id), json);
                return Ok(());
            }

            task.status = Status::Done;
            task.updated = Some(Utc::now());

            let task_title = task.title.clone();
            storage::save(&path, &tf).map_err(|e| (1, e))?;
            output::print_success(&format!("Completed task {}: {}", id, task_title), json);
            Ok(())
        }

        Command::Undo { id } => {
            let mut tf = storage::load(&path, strict).map_err(|e| (1, e))?;
            let task = tf.find_task_mut(id)
                .ok_or((2, format!("Task {} not found", id)))?;

            if task.status == Status::Open {
                output::print_success(&format!("Task {} is already open", id), json);
                return Ok(());
            }

            task.status = Status::Open;
            task.updated = Some(Utc::now());

            let task_title = task.title.clone();
            storage::save(&path, &tf).map_err(|e| (1, e))?;
            output::print_success(&format!("Reopened task {}: {}", id, task_title), json);
            Ok(())
        }

        Command::Tui => {
            tui::run(&path).map_err(|e| (1, e))?;
            Ok(())
        }

        Command::Rm { id } => {
            let mut tf = storage::load(&path, strict).map_err(|e| (1, e))?;
            let removed = tf.remove_task(id)
                .ok_or((2, format!("Task {} not found", id)))?;

            storage::save(&path, &tf).map_err(|e| (1, e))?;
            output::print_success(&format!("Removed task {}: {}", id, removed.title), json);
            Ok(())
        }

        Command::Migrate => {
            let mut tf = storage::load(&path, strict).map_err(|e| (1, e))?;
            tf.format_version = 2;
            storage::save(&path, &tf).map_err(|e| (1, e))?;
            output::print_success(&format!("Migrated {} to format:2", path.display()), json);
            Ok(())
        }

        Command::Auth { subcommand } => match subcommand {
            AuthCommand::Todoist { token } => {
                let token = auth::prompt_for_token(token).map_err(|e| (1, e))?;
                auth::write_token(&token).map_err(|e| (1, e))?;
                output::print_success("Todoist token stored.", json);
                Ok(())
            }

            AuthCommand::Status => {
                if auth::read_token().is_some() {
                    output::print_success("Todoist token: present", json);
                } else {
                    output::print_success("Todoist token: not set", json);
                }
                Ok(())
            }

            AuthCommand::Revoke => {
                match auth::delete_token().map_err(|e| (1, e))? {
                    true => output::print_success("Todoist token revoked.", json),
                    false => output::print_success("No Todoist token found.", json),
                }
                Ok(())
            }
        },

        Command::Config { subcommand } => match subcommand {
            ConfigCommand::Set { key, value } => {
                config::write_config_value(&key, &value).map_err(|e| (1, e))?;
                output::print_success(&format!("Set {} = {}", key, value), json);
                Ok(())
            }
            ConfigCommand::Get { key } => {
                match config::read_config_value(&key) {
                    Some(val) => output::print_success(&val, json),
                    None => output::print_success(&format!("{} is not set", key), json),
                }
                Ok(())
            }
        },

        Command::Import { subcommand } => match subcommand {
            ImportCommand::Todoist { test } => {
                let token = auth::read_token()
                    .ok_or((1, "No Todoist token found. Run `task auth todoist` first.".to_string()))?;

                let mut tf = storage::load(&path, strict).map_err(|e| (1, e))?;
                let (imported, skipped) = todoist::run_import(&token, &mut tf, test)
                    .map_err(|e| (1, e))?;

                if imported > 0 {
                    storage::save(&path, &tf).map_err(|e| (1, e))?;
                }

                let msg = if test {
                    format!("[test mode] Imported {} tasks (Todoist tasks not labeled)", imported)
                } else {
                    format!("Imported {} tasks, skipped {} (already exported)", imported, skipped)
                };
                output::print_success(&msg, json);
                Ok(())
            }
        },
    }
}

fn parse_due_date(input: Option<&str>) -> Result<Option<NaiveDate>, (i32, String)> {
    match input {
        None => Ok(None),
        Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map(Some)
            .map_err(|_| (1, format!("Invalid date '{}'. Expected format: YYYY-MM-DD", s))),
    }
}

fn validate_and_parse_tags(input: Option<&str>) -> Result<Vec<String>, (i32, String)> {
    match input {
        None => Ok(Vec::new()),
        Some(s) if s.is_empty() => Ok(Vec::new()),
        Some(s) => {
            let tags: Vec<String> = s.split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect();

            for tag in &tags {
                if !tag.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
                    return Err((1, format!(
                        "Invalid tag: '{}'. Tags must be lowercase alphanumeric characters and hyphens.", tag
                    )));
                }
            }
            Ok(tags)
        }
    }
}
