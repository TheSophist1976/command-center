use std::process;

use clap::Parser;

use task::cli::{AgentCommand, AgentInstructionsCommand, AuthCommand, Cli, Command, ConfigCommand, NoteCommand};

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
    let path = task::storage::resolve_file_path(cli.file.as_deref());
    task::storage::backup_daily(&path);

    match cli.command {
        None | Some(Command::Tui) => {
            task::tui::run(&path).map_err(|e| (1, e))?;
            Ok(())
        }

        Some(Command::Auth { subcommand }) => match subcommand {
            AuthCommand::Todoist { token } => {
                let token = task::auth::prompt_for_token(token).map_err(|e| (1, e))?;
                task::auth::write_token(&token).map_err(|e| (1, e))?;
                println!("Todoist token stored.");
                Ok(())
            }

            AuthCommand::Status => {
                let todoist_status = if task::auth::read_token().is_some() {
                    "Todoist token: present"
                } else {
                    "Todoist token: not set"
                };
                println!("{}", todoist_status);
                Ok(())
            }

            AuthCommand::Revoke => {
                let todoist_deleted = task::auth::delete_token().map_err(|e| (1, e))?;
                let mut msgs = Vec::new();
                if todoist_deleted { msgs.push("Todoist token revoked."); }
                if msgs.is_empty() { msgs.push("No tokens found."); }
                println!("{}", msgs.join(" "));
                Ok(())
            }
        },

        Some(Command::Config { subcommand }) => match subcommand {
            ConfigCommand::Set { key, value } => {
                task::config::write_config_value(&key, &value).map_err(|e| (1, e))?;
                println!("Set {} = {}", key, value);
                Ok(())
            }
            ConfigCommand::Get { key } => {
                match task::config::read_config_value(&key) {
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
                    let notes = task::note::discover_notes(&dir, &task_filename);
                    for n in &notes {
                        println!("{}  {}", n.slug, n.title);
                    }
                    Ok(())
                }

                NoteCommand::Add { title, task: task_id } => {
                    let base_slug = task::note::slugify(&title);
                    let slug = task::note::unique_slug(&dir, &base_slug);
                    let new_note = task::note::Note {
                        slug: slug.clone(),
                        title: title.clone(),
                        body: String::new(),
                    };
                    let file_path = task::note::write_note(&dir, &new_note).map_err(|e| (1, e))?;
                    println!("{}", file_path.display());

                    if let Some(id) = task_id {
                        let mut task_file = task::storage::load(&path, false).map_err(|e| (1, e))?;
                        match task_file.find_task_mut(id) {
                            Some(t) => {
                                t.note = Some(slug.clone());
                                task::storage::save(&path, &task_file).map_err(|e| (1, e))?;
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
                    let n = task::note::read_note(&note_path).map_err(|e| (1, e))?;
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
                    let mut n = task::note::read_note(&note_path).map_err(|e| (1, e))?;
                    if let Some(t) = title {
                        n.title = t;
                    }
                    if let Some(b) = body {
                        n.body = b;
                    }
                    let file_path = task::note::write_note(&dir, &n).map_err(|e| (1, e))?;
                    println!("{}", file_path.display());
                    Ok(())
                }

                NoteCommand::Rm { slug } => {
                    task::note::delete_note(&dir, &slug).map_err(|e| (1, e))?;
                    println!("Deleted note: {}", slug);
                    Ok(())
                }

                NoteCommand::Link { slug, task_id } => {
                    let mut task_file = task::storage::load(&path, false).map_err(|e| (1, e))?;
                    match task_file.find_task_mut(task_id) {
                        Some(t) => {
                            t.note = Some(slug.clone());
                            task::storage::save(&path, &task_file).map_err(|e| (1, e))?;
                            println!("Linked note '{}' to task {}", slug, task_id);
                            Ok(())
                        }
                        None => Err((1, format!("task {} not found", task_id))),
                    }
                }

                NoteCommand::Unlink { task_id } => {
                    let mut task_file = task::storage::load(&path, false).map_err(|e| (1, e))?;
                    match task_file.find_task_mut(task_id) {
                        Some(t) => {
                            t.note = None;
                            task::storage::save(&path, &task_file).map_err(|e| (1, e))?;
                            println!("Unlinked note from task {}", task_id);
                            Ok(())
                        }
                        None => Err((1, format!("task {} not found", task_id))),
                    }
                }
            }
        }

        Some(Command::Agent { subcommand }) => {
            let task_dir = path.parent().unwrap_or(std::path::Path::new(".")).to_path_buf();
            let instructions_dir = task_dir.join("Notes").join("Instructions");

            match subcommand {
                AgentCommand::Instructions { name, action } => {
                    let slug = task::config::read_config_value(
                        &format!("agent-{}-instructions", name)
                    )
                    .unwrap_or_else(|| task::note::slugify(&name));

                    let note_path = instructions_dir.join(format!("{}.md", slug));

                    match action {
                        AgentInstructionsCommand::Show => {
                            if note_path.exists() {
                                match task::note::read_note(&note_path) {
                                    Ok(note) => {
                                        println!("# {}\n\n{}", note.title, note.body);
                                        Ok(())
                                    }
                                    Err(e) => Err((1, e)),
                                }
                            } else {
                                println!("No instructions found for agent '{}'.", name);
                                Ok(())
                            }
                        }

                        AgentInstructionsCommand::Edit { title, body } => {
                            if title.is_none() && body.is_none() {
                                return Err((1, "Provide at least --title or --body.".to_string()));
                            }
                            let existing = if note_path.exists() {
                                task::note::read_note(&note_path).ok()
                            } else {
                                None
                            };
                            let new_title = title
                                .unwrap_or_else(|| existing.as_ref().map(|n| n.title.clone()).unwrap_or_else(|| format!("{} Instructions", name)));
                            let new_body = body
                                .unwrap_or_else(|| existing.as_ref().map(|n| n.body.clone()).unwrap_or_default());
                            let note = task::note::Note {
                                slug: slug.clone(),
                                title: new_title,
                                body: new_body,
                            };
                            task::note::write_note(&instructions_dir, &note)
                                .map(|p| println!("{}", p.display()))
                                .map_err(|e| (1, e))
                        }
                    }
                }
            }
        }
    }
}
