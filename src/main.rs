mod auth;
mod claude_session;
mod cli;
mod config;
mod nlp;
mod note;
mod parser;
mod slack;
mod storage;
mod task;
mod todoist;
mod tui;

use std::process;

use clap::Parser;

use cli::{AuthCommand, Cli, Command, ConfigCommand, NoteCommand};

fn main() {
    let cli = Cli::parse();

    let code = match run(cli) {
        Ok(()) => 0,
        Err((code, msg)) => {
            eprintln!("Error: {}", msg);
            code
        }
    };
    process::exit(code);
}

fn run(cli: Cli) -> Result<(), (i32, String)> {
    let path = storage::resolve_file_path(cli.file.as_deref());
    storage::backup_daily(&path);

    match cli.command {
        None | Some(Command::Tui) => {
            tui::run(&path).map_err(|e| (1, e))?;
            Ok(())
        }

        Some(Command::Auth { subcommand }) => match subcommand {
            AuthCommand::Todoist { token } => {
                let token = auth::prompt_for_token(token).map_err(|e| (1, e))?;
                auth::write_token(&token).map_err(|e| (1, e))?;
                println!("Todoist token stored.");
                Ok(())
            }

            AuthCommand::Claude { key } => {
                let key = auth::prompt_for_claude_key(key).map_err(|e| (1, e))?;
                auth::write_claude_key(&key).map_err(|e| (1, e))?;
                println!("Claude API key stored.");
                Ok(())
            }

            AuthCommand::Slack { token } => {
                let token = auth::prompt_for_slack_token(token).map_err(|e| (1, e))?;
                auth::write_slack_token(&token).map_err(|e| (1, e))?;
                println!("Slack token stored.");
                Ok(())
            }

            AuthCommand::Status => {
                let todoist_status = if auth::read_token().is_some() {
                    "Todoist token: present"
                } else {
                    "Todoist token: not set"
                };
                let claude_status = match auth::read_claude_key_source() {
                    Some(("env", _)) => "Claude API key: present (env)".to_string(),
                    Some((_, _)) => "Claude API key: present".to_string(),
                    None => "Claude API key: not set".to_string(),
                };
                let slack_status = if auth::read_slack_token().is_some() {
                    "Slack token: present"
                } else {
                    "Slack token: not set"
                };
                println!("{}\n{}\n{}", todoist_status, claude_status, slack_status);
                Ok(())
            }

            AuthCommand::Revoke => {
                let todoist_deleted = auth::delete_token().map_err(|e| (1, e))?;
                let claude_deleted = auth::delete_claude_key().map_err(|e| (1, e))?;
                let slack_deleted = auth::delete_slack_token().map_err(|e| (1, e))?;
                let mut msgs = Vec::new();
                if todoist_deleted { msgs.push("Todoist token revoked."); }
                if claude_deleted { msgs.push("Claude API key revoked."); }
                if slack_deleted { msgs.push("Slack token revoked."); }
                if msgs.is_empty() { msgs.push("No tokens found."); }
                println!("{}", msgs.join(" "));
                Ok(())
            }
        },

        Some(Command::Config { subcommand }) => match subcommand {
            ConfigCommand::Set { key, value } => {
                config::write_config_value(&key, &value).map_err(|e| (1, e))?;
                println!("Set {} = {}", key, value);
                Ok(())
            }
            ConfigCommand::Get { key } => {
                match config::read_config_value(&key) {
                    Some(val) => println!("{}", val),
                    None => println!("{} is not set", key),
                }
                Ok(())
            }
        },

        Some(Command::Note { subcommand }) => {
            let dir = path.parent().unwrap_or(std::path::Path::new(".")).to_path_buf();
            let task_filename = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("tasks.md")
                .to_string();
            match subcommand {
                NoteCommand::List => {
                    let notes = note::discover_notes(&dir, &task_filename);
                    for n in &notes {
                        println!("{}  {}", n.slug, n.title);
                    }
                    Ok(())
                }

                NoteCommand::Add { title, task: task_id } => {
                    let base_slug = note::slugify(&title);
                    let slug = note::unique_slug(&dir, &base_slug);
                    let new_note = note::Note {
                        slug: slug.clone(),
                        title: title.clone(),
                        body: String::new(),
                    };
                    let file_path = note::write_note(&dir, &new_note).map_err(|e| (1, e))?;
                    println!("{}", file_path.display());

                    if let Some(id) = task_id {
                        let mut task_file = storage::load(&path, false).map_err(|e| (1, e))?;
                        match task_file.find_task_mut(id) {
                            Some(t) => {
                                t.note = Some(slug.clone());
                                storage::save(&path, &task_file).map_err(|e| (1, e))?;
                            }
                            None => {
                                eprintln!("Warning: task {} not found; note was created but not linked", id);
                                return Err((1, format!("task {} not found", id)));
                            }
                        }
                    }
                    Ok(())
                }

                NoteCommand::Show { slug } => {
                    let note_path = dir.join(format!("{}.md", slug));
                    let n = note::read_note(&note_path).map_err(|e| (1, e))?;
                    println!("# {}", n.title);
                    if !n.body.is_empty() {
                        println!();
                        print!("{}", n.body);
                        if !n.body.ends_with('\n') {
                            println!();
                        }
                    }
                    Ok(())
                }

                NoteCommand::Edit { slug, title, body } => {
                    if title.is_none() && body.is_none() {
                        return Err((1, "at least one of --title or --body must be provided".to_string()));
                    }
                    let note_path = dir.join(format!("{}.md", slug));
                    let mut n = note::read_note(&note_path).map_err(|e| (1, e))?;
                    if let Some(t) = title {
                        n.title = t;
                    }
                    if let Some(b) = body {
                        n.body = b;
                    }
                    let file_path = note::write_note(&dir, &n).map_err(|e| (1, e))?;
                    println!("{}", file_path.display());
                    Ok(())
                }

                NoteCommand::Rm { slug } => {
                    note::delete_note(&dir, &slug).map_err(|e| (1, e))?;
                    println!("Deleted note: {}", slug);
                    Ok(())
                }

                NoteCommand::Link { slug, task_id } => {
                    let mut task_file = storage::load(&path, false).map_err(|e| (1, e))?;
                    match task_file.find_task_mut(task_id) {
                        Some(t) => {
                            t.note = Some(slug.clone());
                            storage::save(&path, &task_file).map_err(|e| (1, e))?;
                            println!("Linked note '{}' to task {}", slug, task_id);
                            Ok(())
                        }
                        None => Err((1, format!("task {} not found", task_id))),
                    }
                }

                NoteCommand::Unlink { task_id } => {
                    let mut task_file = storage::load(&path, false).map_err(|e| (1, e))?;
                    match task_file.find_task_mut(task_id) {
                        Some(t) => {
                            t.note = None;
                            storage::save(&path, &task_file).map_err(|e| (1, e))?;
                            println!("Unlinked note from task {}", task_id);
                            Ok(())
                        }
                        None => Err((1, format!("task {} not found", task_id))),
                    }
                }
            }
        }
    }
}
