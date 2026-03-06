mod auth;
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

use cli::{AuthCommand, Cli, Command, ConfigCommand};

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
    }
}
