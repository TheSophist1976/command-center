//! Resolves where a session's tasks database and notes live.
//!
//! Historically both lived in one directory (`default-dir`): `tasks.db` next
//! to a `Notes/` folder. Profiles split them apart so the database can stay
//! on local disk while notes stay in a synced vault, and so separate
//! workspaces (e.g. `work` and `home`) never share tasks or notes:
//!
//! ```text
//! profile: work
//! profile-work-dir: ~/Documents/Mark-main/Tasks
//! profile-home-dir: ~/Documents/Mark-main/Home
//! profile-home-db: ~/somewhere/else/tasks.db   # optional override
//! ```
//!
//! A profile's database defaults to `<data_dir>/task-manager/<name>/tasks.db`
//! (`~/.local/share/...` on Linux, `~/Library/Application Support/...` on macOS).

use std::fs;
use std::path::{Path, PathBuf};

use crate::config;

#[derive(Debug, Clone, PartialEq)]
pub struct Workspace {
    /// Active profile name, or `None` for legacy single-directory mode.
    pub profile: Option<String>,
    /// SQLite task database — local, machine-specific state.
    pub db_path: PathBuf,
    /// Directory holding `Notes/` (and, for legacy setups, `tasks.md`).
    pub task_dir: PathBuf,
}

impl Workspace {
    /// A legacy workspace: notes live next to the database.
    pub fn from_db_path(db_path: PathBuf) -> Self {
        let task_dir = db_path.parent().unwrap_or(Path::new(".")).to_path_buf();
        Workspace { profile: None, db_path, task_dir }
    }

    pub fn notes_dir(&self) -> PathBuf {
        self.task_dir.join("Notes")
    }

    /// Directory for local, machine-specific state (backups, Claude sessions).
    pub fn state_dir(&self) -> PathBuf {
        self.db_path.parent().unwrap_or(Path::new(".")).to_path_buf()
    }
}

/// A profile as configured: `profile-<name>-dir` and optional `profile-<name>-db`.
#[derive(Debug, Clone, PartialEq)]
pub struct Profile {
    pub name: String,
    pub dir: String,
    pub db: Option<String>,
}

pub fn list_profiles() -> Vec<Profile> {
    match config::config_path() {
        Some(p) => list_profiles_from(&p),
        None => Vec::new(),
    }
}

pub fn list_profiles_from(path: &Path) -> Vec<Profile> {
    let content = fs::read_to_string(path).unwrap_or_default();
    let mut profiles: Vec<Profile> = Vec::new();
    for line in content.lines() {
        let Some(rest) = line.strip_prefix("profile-") else { continue };
        let Some((key, value)) = rest.split_once(':') else { continue };
        let value = value.trim().to_string();
        if value.is_empty() {
            continue;
        }
        let (name, is_db) = if let Some(n) = key.strip_suffix("-dir") {
            (n, false)
        } else if let Some(n) = key.strip_suffix("-db") {
            (n, true)
        } else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        let idx = match profiles.iter().position(|p| p.name == name) {
            Some(i) => i,
            None => {
                profiles.push(Profile { name: name.to_string(), dir: String::new(), db: None });
                profiles.len() - 1
            }
        };
        if is_db {
            profiles[idx].db = Some(value);
        } else {
            profiles[idx].dir = value;
        }
    }
    // A db override without a notes dir isn't a usable profile.
    profiles.retain(|p| !p.dir.is_empty());
    profiles
}

/// Default local database location for a profile.
pub fn default_db_path(profile: &str) -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("task-manager").join(profile).join("tasks.db")
}

/// Resolve the workspace for this invocation.
///
/// Precedence: `--file` / `TASK_FILE` (explicit database, legacy layout) →
/// `--profile` / `TASK_PROFILE` → `profile:` in config → `default-dir` in
/// config (legacy layout) → `./tasks.db`.
pub fn resolve(file_flag: Option<&str>, profile_flag: Option<&str>) -> Result<Workspace, String> {
    resolve_inner(file_flag, profile_flag, config::config_path().as_deref())
}

fn resolve_inner(
    file_flag: Option<&str>,
    profile_flag: Option<&str>,
    config_path: Option<&Path>,
) -> Result<Workspace, String> {
    if let Some(path) = file_flag {
        return Ok(Workspace::from_db_path(PathBuf::from(path)));
    }
    if let Some(env_path) = non_empty_env("TASK_FILE") {
        return Ok(Workspace::from_db_path(PathBuf::from(env_path)));
    }

    let read = |key: &str| config_path.and_then(|p| config::read_config_value_from(p, key)).filter(|v| !v.is_empty());

    let selected = profile_flag
        .map(str::to_string)
        .or_else(|| non_empty_env("TASK_PROFILE"))
        .or_else(|| read("profile"));

    if let Some(name) = selected {
        let profiles = config_path.map(list_profiles_from).unwrap_or_default();
        let profile = profiles.into_iter().find(|p| p.name == name).ok_or_else(|| {
            format!(
                "Profile '{}' is not configured. Add it with: task profile add {} --dir <notes-dir>",
                name, name
            )
        })?;
        let db_path = match &profile.db {
            Some(db) => config::expand_tilde(db),
            None => default_db_path(&profile.name),
        };
        return Ok(Workspace {
            profile: Some(profile.name),
            db_path,
            task_dir: config::expand_tilde(&profile.dir),
        });
    }

    if let Some(default_dir) = read("default-dir") {
        return Ok(Workspace::from_db_path(config::expand_tilde(&default_dir).join("tasks.db")));
    }
    Ok(Workspace::from_db_path(PathBuf::from("tasks.db")))
}

/// Handles `task profile ...`. Runs before workspace resolution so a
/// misconfigured profile can always be fixed from the CLI.
pub fn run_profile_command(
    cmd: crate::cli::ProfileCommand,
    file_flag: Option<&str>,
    profile_flag: Option<&str>,
) -> Result<String, String> {
    use crate::cli::ProfileCommand;
    match cmd {
        ProfileCommand::List => {
            let default = config::read_config_value("profile").unwrap_or_default();
            let profiles = list_profiles();
            if profiles.is_empty() {
                return Ok("No profiles configured. Add one with: task profile add <name> --dir <notes-dir>".to_string());
            }
            let lines: Vec<String> = profiles
                .iter()
                .map(|p| {
                    let marker = if p.name == default { "*" } else { " " };
                    let db = p.db.as_deref().map(config::expand_tilde).unwrap_or_else(|| default_db_path(&p.name));
                    format!("{} {}  notes: {}  db: {}", marker, p.name, p.dir, db.display())
                })
                .collect();
            Ok(lines.join("\n"))
        }
        ProfileCommand::Show => {
            let ws = resolve(file_flag, profile_flag)?;
            Ok(format!(
                "profile: {}\ndb: {}\nnotes: {}",
                ws.profile.as_deref().unwrap_or("(none — legacy default-dir)"),
                ws.db_path.display(),
                ws.notes_dir().display()
            ))
        }
        ProfileCommand::Add { name, dir, db } => {
            if name.is_empty() || name.contains(char::is_whitespace) || name.contains(':') {
                return Err(format!("Invalid profile name '{}'", name));
            }
            config::write_config_value(&format!("profile-{}-dir", name), &dir)?;
            if let Some(db) = &db {
                config::write_config_value(&format!("profile-{}-db", name), db)?;
            }
            let mut msg = format!("Profile '{}' saved (notes: {})", name, dir);
            if config::read_config_value("profile").filter(|v| !v.is_empty()).is_none() {
                config::write_config_value("profile", &name)?;
                msg.push_str(&format!("\nDefault profile set to '{}'", name));
            }
            Ok(msg)
        }
        ProfileCommand::Use { name } => {
            if !list_profiles().iter().any(|p| p.name == name) {
                return Err(format!(
                    "Profile '{}' is not configured. Add it with: task profile add {} --dir <notes-dir>",
                    name, name
                ));
            }
            config::write_config_value("profile", &name)?;
            Ok(format!("Default profile set to '{}'", name))
        }
    }
}

fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::tempdir;

    fn clear_env() {
        unsafe {
            env::remove_var("TASK_FILE");
            env::remove_var("TASK_PROFILE");
        }
    }

    #[test]
    fn test_list_profiles_parses_dir_and_db() {
        let dir = tempdir().unwrap();
        let cfg = dir.path().join("config.md");
        fs::write(
            &cfg,
            "profile: work\nprofile-work-dir: /vault/Work\nprofile-my-home-dir: /vault/Home\nprofile-my-home-db: /local/home.db\nprofile-orphan-db: /x.db\n",
        )
        .unwrap();
        let profiles = list_profiles_from(&cfg);
        assert_eq!(
            profiles,
            vec![
                Profile { name: "work".into(), dir: "/vault/Work".into(), db: None },
                Profile { name: "my-home".into(), dir: "/vault/Home".into(), db: Some("/local/home.db".into()) },
            ]
        );
    }

    #[test]
    fn test_file_flag_wins() {
        clear_env();
        let ws = resolve_inner(Some("/tmp/x/tasks.db"), Some("work"), None).unwrap();
        assert_eq!(ws.db_path, PathBuf::from("/tmp/x/tasks.db"));
        assert_eq!(ws.task_dir, PathBuf::from("/tmp/x"));
        assert_eq!(ws.profile, None);
    }

    #[test]
    fn test_profile_from_config_splits_db_and_notes() {
        clear_env();
        let dir = tempdir().unwrap();
        let cfg = dir.path().join("config.md");
        fs::write(&cfg, "default-dir: /legacy\nprofile: work\nprofile-work-dir: /vault/Work\n").unwrap();
        let ws = resolve_inner(None, None, Some(&cfg)).unwrap();
        assert_eq!(ws.profile.as_deref(), Some("work"));
        assert_eq!(ws.task_dir, PathBuf::from("/vault/Work"));
        assert_eq!(ws.notes_dir(), PathBuf::from("/vault/Work/Notes"));
        assert_eq!(ws.db_path, default_db_path("work"));
    }

    #[test]
    fn test_profile_flag_overrides_config_and_db_override() {
        clear_env();
        let dir = tempdir().unwrap();
        let cfg = dir.path().join("config.md");
        fs::write(
            &cfg,
            "profile: work\nprofile-work-dir: /vault/Work\nprofile-home-dir: /vault/Home\nprofile-home-db: /local/home.db\n",
        )
        .unwrap();
        let ws = resolve_inner(None, Some("home"), Some(&cfg)).unwrap();
        assert_eq!(ws.profile.as_deref(), Some("home"));
        assert_eq!(ws.db_path, PathBuf::from("/local/home.db"));
        assert_eq!(ws.task_dir, PathBuf::from("/vault/Home"));
        assert_eq!(ws.state_dir(), PathBuf::from("/local"));
    }

    #[test]
    fn test_unknown_profile_errors() {
        clear_env();
        let dir = tempdir().unwrap();
        let cfg = dir.path().join("config.md");
        fs::write(&cfg, "profile-work-dir: /vault/Work\n").unwrap();
        let err = resolve_inner(None, Some("nope"), Some(&cfg)).unwrap_err();
        assert!(err.contains("'nope' is not configured"));
    }

    #[test]
    fn test_legacy_default_dir_keeps_db_with_notes() {
        clear_env();
        let dir = tempdir().unwrap();
        let cfg = dir.path().join("config.md");
        fs::write(&cfg, "default-dir: /legacy\n").unwrap();
        let ws = resolve_inner(None, None, Some(&cfg)).unwrap();
        assert_eq!(ws.db_path, PathBuf::from("/legacy/tasks.db"));
        assert_eq!(ws.task_dir, PathBuf::from("/legacy"));
        assert_eq!(ws.profile, None);
    }
}
