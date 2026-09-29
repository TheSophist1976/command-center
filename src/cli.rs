use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "task", about = "A fast CLI task manager with an interactive TUI", version)]
pub struct Cli {
    /// Path to the task database (overrides any profile)
    #[arg(long, global = true)]
    pub file: Option<String>,

    /// Profile to use (overrides TASK_PROFILE and the `profile` config key)
    #[arg(long, global = true)]
    pub profile: Option<String>,

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

    /// Manage workspace profiles (separate task databases and notes)
    Profile {
        #[command(subcommand)]
        subcommand: ProfileCommand,
    },

    /// Manage agent profiles and instructions
    Agent {
        #[command(subcommand)]
        subcommand: AgentCommand,
    },

    /// Add a new task
    Add {
        /// Task title (wrap in quotes if it contains spaces)
        title: String,

        /// Priority: critical, high, medium (default), low
        #[arg(short, long, default_value = "medium")]
        priority: String,

        /// Due date: YYYY-MM-DD or weekday name (e.g. monday, fri)
        #[arg(short, long)]
        due: Option<String>,

        /// Project name
        #[arg(long)]
        project: Option<String>,

        /// Comma-separated tags (e.g. security,ci-cd)
        #[arg(long)]
        tags: Option<String>,

        /// Agent to assign the task to
        #[arg(long)]
        agent: Option<String>,

        /// Task description
        #[arg(long)]
        description: Option<String>,
    },

    /// List tasks
    List {
        /// Filter by status: open or done
        #[arg(long)]
        status: Option<String>,

        /// Filter by assigned agent
        #[arg(long)]
        agent: Option<String>,

        /// Filter by project
        #[arg(long)]
        project: Option<String>,

        /// Filter by a single tag
        #[arg(long)]
        tag: Option<String>,

        /// Only show tasks due on or before this date (YYYY-MM-DD)
        #[arg(long)]
        due_before: Option<String>,
    },

    /// Show a task's full detail
    Show {
        /// Task id
        id: u32,
    },

    /// Edit a task
    Edit {
        /// Task id
        id: u32,

        #[arg(long)]
        title: Option<String>,

        #[arg(short, long)]
        priority: Option<String>,

        #[arg(short, long)]
        due: Option<String>,

        #[arg(long)]
        project: Option<String>,

        #[arg(long)]
        tags: Option<String>,

        #[arg(long)]
        agent: Option<String>,

        #[arg(long)]
        description: Option<String>,

        #[arg(long)]
        effort: Option<String>,

        #[arg(long)]
        work_status: Option<String>,

        #[arg(long)]
        recur: Option<String>,
    },

    /// Mark a task done
    Done {
        /// Task id
        id: u32,
    },

    /// Reopen a completed task
    Reopen {
        /// Task id
        id: u32,
    },

    /// Delete a task
    Rm {
        /// Task id
        id: u32,
    },
}

#[derive(Subcommand)]
pub enum AgentCommand {
    /// Manage agent instruction notes
    Instructions {
        /// Agent name (must match an agent-<name> profile in config)
        name: String,

        #[command(subcommand)]
        action: AgentInstructionsCommand,
    },

    /// Manage agent memory notes
    Memory {
        /// Agent name (must match an agent-<name> profile in config)
        name: String,

        #[command(subcommand)]
        action: AgentMemoryCommand,
    },
}

#[derive(Subcommand)]
pub enum AgentMemoryCommand {
    /// Show the agent's memory note
    Show,

    /// Create or update the agent's memory note
    Edit {
        /// New title for the memory note
        #[arg(long)]
        title: Option<String>,

        /// New body (replaces entire body)
        #[arg(long)]
        body: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum AgentInstructionsCommand {
    /// Show the agent's instruction note
    Show,

    /// Create or update the agent's instruction note
    Edit {
        /// New title for the instruction note
        #[arg(long)]
        title: Option<String>,

        /// New body (replaces entire body)
        #[arg(long)]
        body: Option<String>,
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
pub enum ProfileCommand {
    /// List configured profiles (* marks the default)
    List,

    /// Show the resolved profile, database, and notes directory
    Show,

    /// Add or update a profile
    Add {
        /// Profile name (e.g. work, home)
        name: String,

        /// Directory holding this profile's Notes/ folder (e.g. inside a vault)
        #[arg(long)]
        dir: String,

        /// Database path (default: <data-dir>/task-manager/<name>/tasks.db, on local disk)
        #[arg(long)]
        db: Option<String>,
    },

    /// Set the default profile
    Use {
        /// Profile name
        name: String,
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

    /// Append a section to a note's existing body
    Append {
        /// Note slug
        slug: String,

        /// Markdown text to append (including its own header, if any)
        #[arg(long)]
        body: String,
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
