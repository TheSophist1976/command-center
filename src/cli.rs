use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "task", about = "A fast CLI task manager with an interactive TUI")]
pub struct Cli {
    /// Path to the task file (default: tasks.md)
    #[arg(long, global = true)]
    pub file: Option<String>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Launch interactive terminal UI
    Tui,

    /// Authenticate with external services
    Auth {
        #[command(subcommand)]
        subcommand: AuthCommand,
    },

    /// Manage application configuration
    Config {
        #[command(subcommand)]
        subcommand: ConfigCommand,
    },
}

#[derive(Subcommand)]
pub enum AuthCommand {
    /// Authenticate with Todoist using a personal API token
    Todoist {
        /// Personal API token (skips interactive prompt)
        #[arg(long)]
        token: Option<String>,
    },

    /// Authenticate with Claude using an API key
    Claude {
        /// Claude API key (skips interactive prompt)
        #[arg(long)]
        key: Option<String>,
    },

    /// Show authentication status
    Status,

    /// Revoke stored authentication tokens
    Revoke,
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// Set a configuration value
    Set {
        /// Configuration key (e.g., default-dir)
        key: String,

        /// Configuration value
        value: String,
    },

    /// Get a configuration value
    Get {
        /// Configuration key (e.g., default-dir)
        key: String,
    },
}
