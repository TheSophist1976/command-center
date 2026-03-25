use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "task", about = "A fast CLI task manager with an interactive TUI", version)]
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

    /// Manage notes
    Note {
        #[command(subcommand)]
        subcommand: NoteCommand,
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

#[derive(Subcommand)]
pub enum NoteCommand {
    /// List all notes
    List,

    /// Create a new note
    Add {
        /// Note title
        title: String,

        /// Link note to this task ID
        #[arg(long)]
        task: Option<u32>,
    },

    /// Show a note's content
    Show {
        /// Note slug
        slug: String,
    },

    /// Edit a note's title or body
    Edit {
        /// Note slug
        slug: String,

        /// New title
        #[arg(long)]
        title: Option<String>,

        /// New body (replaces entire body)
        #[arg(long)]
        body: Option<String>,
    },

    /// Delete a note
    Rm {
        /// Note slug
        slug: String,
    },

    /// Link a note to a task
    Link {
        /// Note slug
        slug: String,

        /// Task ID to link to
        task_id: u32,
    },

    /// Unlink the note from a task
    Unlink {
        /// Task ID to unlink
        task_id: u32,
    },
}
