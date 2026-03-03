use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "task", about = "A fast CLI task manager for developers and AI agents")]
pub struct Cli {
    /// Path to the task file (default: tasks.md)
    #[arg(long, global = true)]
    pub file: Option<String>,

    /// Output in JSON format
    #[arg(long, global = true)]
    pub json: bool,

    /// Strict parsing mode — report errors for malformed tasks
    #[arg(long, global = true)]
    pub strict: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize a new task file
    Init,

    /// Add a new task
    Add {
        /// Task title
        title: String,

        /// Priority: critical, high, medium, low (default: medium)
        #[arg(long, short)]
        priority: Option<String>,

        /// Comma-separated tags
        #[arg(long, short)]
        tags: Option<String>,

        /// Due date (YYYY-MM-DD)
        #[arg(long)]
        due: Option<String>,

        /// Project name
        #[arg(long)]
        project: Option<String>,

        /// Recurrence pattern: daily, weekly, monthly, yearly, or monthly:N:DAY (e.g., monthly:3:thu)
        #[arg(long)]
        recur: Option<String>,
    },

    /// List tasks with optional filters
    List {
        /// Filter by status: open, done
        #[arg(long, short)]
        status: Option<String>,

        /// Filter by priority: critical, high, medium, low
        #[arg(long, short)]
        priority: Option<String>,

        /// Filter by tag
        #[arg(long, short)]
        tag: Option<String>,

        /// Filter by project name
        #[arg(long)]
        project: Option<String>,
    },

    /// Show full details of a task
    Show {
        /// Task ID
        id: u32,
    },

    /// Edit a task's title, priority, tags, due date, or project
    Edit {
        /// Task ID
        id: u32,

        /// New title
        #[arg(long, short)]
        title: Option<String>,

        /// New priority: critical, high, medium, low
        #[arg(long, short)]
        priority: Option<String>,

        /// New comma-separated tags (replaces existing)
        #[arg(long)]
        tags: Option<String>,

        /// New due date (YYYY-MM-DD)
        #[arg(long)]
        due: Option<String>,

        /// New project name
        #[arg(long)]
        project: Option<String>,

        /// Recurrence pattern: daily, weekly, monthly, yearly, monthly:N:DAY, or "none" to clear
        #[arg(long)]
        recur: Option<String>,
    },

    /// Mark a task as done
    Done {
        /// Task ID
        id: u32,
    },

    /// Reopen a completed task
    Undo {
        /// Task ID
        id: u32,
    },

    /// Remove a task
    Rm {
        /// Task ID
        id: u32,
    },

    /// Launch interactive terminal UI
    Tui,

    /// Migrate task file to the latest format version
    Migrate,

    /// Authenticate with external services
    Auth {
        #[command(subcommand)]
        subcommand: AuthCommand,
    },

    /// Import tasks from external services
    Import {
        #[command(subcommand)]
        subcommand: ImportCommand,
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

#[derive(Subcommand)]
pub enum ImportCommand {
    /// Import open tasks from Todoist
    Todoist {
        /// Test mode: import only first 3 tasks without labeling in Todoist
        #[arg(long)]
        test: bool,
    },
}
