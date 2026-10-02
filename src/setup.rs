use std::path::Path;

use crate::config::{expand_tilde, read_config_value_from, write_config_value_to};
use crate::managed::{self, EditedPolicy, InstallReport};
use crate::prompt::Prompter;

/// Asks for one config value. An existing value is only replaced if the user says so.
/// Returns the value now in effect, if any.
fn configure_key(
    config_file: &Path,
    key: &str,
    question: &str,
    suggested: Option<&str>,
    optional: bool,
    prompter: &mut dyn Prompter,
) -> Result<Option<String>, String> {
    let existing = read_config_value_from(config_file, key).filter(|v| !v.is_empty());
    if let Some(current) = &existing {
        if !prompter.confirm(&format!("{} is currently {}. Change it?", key, current), false) {
            return Ok(existing);
        }
    }
    let answer = prompter.ask(question, suggested.or(existing.as_deref()));
    if answer.is_empty() {
        if optional {
            return Ok(existing);
        }
        return Err(format!("{} is required", key));
    }
    write_config_value_to(config_file, key, &answer)?;
    Ok(Some(answer))
}

pub fn run_setup(config_file: &Path, home: &Path, prompter: &mut dyn Prompter) -> Result<InstallReport, String> {
    let default_dir = configure_key(config_file, "default-dir", "Directory for tasks.db", Some("~/tasks"), false, prompter)?;
    let notes_suggestion = default_dir.as_ref().map(|d| format!("{}/Notes", d.trim_end_matches('/')));
    configure_key(config_file, "notes-dir", "Directory for notes", notes_suggestion.as_deref(), false, prompter)?;
    configure_key(config_file, "obsidian-vault", "Obsidian vault name (blank to skip)", None, true, prompter)?;

    for key in ["default-dir", "notes-dir"] {
        if let Some(v) = read_config_value_from(config_file, key).filter(|v| !v.is_empty()) {
            let dir = expand_tilde(&v);
            std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create {}: {}", dir.display(), e))?;
        }
    }

    managed::refresh(config_file, home, &mut EditedPolicy::Ask(prompter))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::read_config_value_from;
    use crate::prompt::testing::ScriptedPrompter;
    use tempfile::tempdir;

    #[test]
    fn fresh_setup_writes_config_creates_dirs_and_installs_files() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("cfg").join("config.md");
        let home = dir.path().join("home");
        let tasks = dir.path().join("my tasks");
        let mut p = ScriptedPrompter::new(&[tasks.to_str().unwrap(), "", "MyVault"]);
        run_setup(&config, &home, &mut p).unwrap();

        assert_eq!(read_config_value_from(&config, "default-dir").unwrap(), tasks.to_str().unwrap());
        assert_eq!(read_config_value_from(&config, "notes-dir").unwrap(), format!("{}/Notes", tasks.display()));
        assert_eq!(read_config_value_from(&config, "obsidian-vault").unwrap(), "MyVault");
        assert!(tasks.join("Notes").is_dir());
        assert!(tasks.join("AGENTS.md").is_file());
        assert!(home.join(".claude/skills/task-manager/SKILL.md").is_file());
    }

    #[test]
    fn rerun_keeps_existing_values_unless_user_says_change() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.md");
        let home = dir.path().join("home");
        let tasks = dir.path().join("tasks");
        let mut first = ScriptedPrompter::new(&[tasks.to_str().unwrap(), "", ""]);
        run_setup(&config, &home, &mut first).unwrap();

        // "Change?" for default-dir and notes-dir (both no); the vault was left blank so it is asked fresh.
        let mut second = ScriptedPrompter::new(&["n", "n", ""]);
        run_setup(&config, &home, &mut second).unwrap();
        assert_eq!(read_config_value_from(&config, "default-dir").unwrap(), tasks.to_str().unwrap());
    }

    #[test]
    fn changing_an_existing_value_requires_confirmation() {
        let dir = tempdir().unwrap();
        let config = dir.path().join("config.md");
        let home = dir.path().join("home");
        let old = dir.path().join("old");
        let new = dir.path().join("new");
        let mut first = ScriptedPrompter::new(&[old.to_str().unwrap(), "", ""]);
        run_setup(&config, &home, &mut first).unwrap();

        let mut second = ScriptedPrompter::new(&["y", new.to_str().unwrap(), "n", ""]);
        run_setup(&config, &home, &mut second).unwrap();
        assert_eq!(read_config_value_from(&config, "default-dir").unwrap(), new.to_str().unwrap());
    }
}
