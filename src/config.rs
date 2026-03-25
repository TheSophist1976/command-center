use std::fs;
use std::path::{Path, PathBuf};

/// Expand a leading `~` to the user's home directory.
/// Leaves paths unchanged if they don't start with `~`.
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    } else if path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }
    PathBuf::from(path)
}

pub fn config_path() -> Option<PathBuf> {
    if let Ok(env_path) = std::env::var("TASK_CONFIG_FILE") {
        if !env_path.is_empty() {
            return Some(PathBuf::from(env_path));
        }
    }
    let base = dirs::config_dir()?;
    Some(base.join("task-manager").join("config.md"))
}

pub fn read_config_value(key: &str) -> Option<String> {
    let path = config_path()?;
    read_config_value_from(&path, key)
}

pub fn write_config_value(key: &str, value: &str) -> Result<(), String> {
    let path = config_path().ok_or("Config directory unavailable on this platform".to_string())?;
    write_config_value_to(&path, key, value)
}

// Path-parameterised helpers for testing

pub fn read_config_value_from(path: &Path, key: &str) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let prefix = format!("{}:", key);
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix(&prefix) {
            return Some(rest.trim().to_string());
        }
    }
    None
}

pub fn write_config_value_to(path: &Path, key: &str, value: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let existing = fs::read_to_string(path).unwrap_or_default();
    let prefix = format!("{}:", key);
    let new_line = format!("{}: {}", key, value);

    let mut found = false;
    let mut lines: Vec<String> = existing
        .lines()
        .map(|l| {
            if l.starts_with(&prefix) {
                found = true;
                new_line.clone()
            } else {
                l.to_string()
            }
        })
        .collect();

    if !found {
        if lines.is_empty() {
            lines.push("# task-manager config".to_string());
            lines.push(String::new());
        }
        lines.push(new_line);
    }

    let mut content = lines.join("\n");
    content.push('\n');

    fs::write(path, content).map_err(|e| format!("Failed to write config file: {}", e))?;
    Ok(())
}

/// Returns all agent profiles from config as `(name, dir)` pairs.
/// Scans for keys with the `agent-` prefix and strips it to get the name.
pub fn list_agent_profiles() -> Vec<(String, String)> {
    let path = match config_path() {
        Some(p) => p,
        None => return Vec::new(),
    };
    list_agent_profiles_from(&path)
}

pub fn list_agent_profiles_from(path: &Path) -> Vec<(String, String)> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let mut profiles = Vec::new();
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("agent-") {
            if let Some((name, dir)) = rest.split_once(':') {
                let name = name.trim().to_string();
                let dir = dir.trim().to_string();
                if !name.is_empty() && !dir.is_empty() {
                    profiles.push((name, dir));
                }
            }
        }
    }
    profiles
}

/// Finds the agent profile whose expanded directory is a prefix of `cwd`.
/// Returns the profile name of the longest (most specific) match, or `None`.
pub fn find_agent_for_cwd(cwd: &Path) -> Option<String> {
    let path = config_path()?;
    find_agent_for_cwd_from(&path, cwd)
}

pub fn find_agent_for_cwd_from(config: &Path, cwd: &Path) -> Option<String> {
    let profiles = list_agent_profiles_from(config);
    let mut best: Option<(usize, String)> = None; // (match_len, name)
    for (name, dir) in profiles {
        let expanded = expand_tilde(&dir);
        if cwd.starts_with(&expanded) {
            let len = expanded.as_os_str().len();
            if best.as_ref().map_or(true, |(best_len, _)| len > *best_len) {
                best = Some((len, name));
            }
        }
    }
    best.map(|(_, name)| name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // -- read_config_value_from --

    #[test]
    fn test_read_key_present() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.md");
        fs::write(&path, "# config\n\ndefault-dir: /home/user/notes\n").unwrap();
        let val = read_config_value_from(&path, "default-dir");
        assert_eq!(val, Some("/home/user/notes".to_string()));
    }

    #[test]
    fn test_read_key_absent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.md");
        fs::write(&path, "# config\n\nother-key: value\n").unwrap();
        let val = read_config_value_from(&path, "default-dir");
        assert!(val.is_none());
    }

    #[test]
    fn test_read_ignores_comment_lines() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.md");
        fs::write(&path, "# task-manager config\n\n# default-dir: /ignored\ndefault-dir: /real\n").unwrap();
        let val = read_config_value_from(&path, "default-dir");
        assert_eq!(val, Some("/real".to_string()));
    }

    #[test]
    fn test_read_nonexistent_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.md");
        let val = read_config_value_from(&path, "default-dir");
        assert!(val.is_none());
    }

    #[test]
    fn test_read_trims_whitespace() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.md");
        fs::write(&path, "default-dir:   /trimmed   \n").unwrap();
        let val = read_config_value_from(&path, "default-dir");
        assert_eq!(val, Some("/trimmed".to_string()));
    }

    // -- write_config_value_to --

    #[test]
    fn test_write_creates_file_and_dir() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("subdir").join("config.md");
        write_config_value_to(&path, "default-dir", "/notes").unwrap();
        assert!(path.exists());
        let val = read_config_value_from(&path, "default-dir");
        assert_eq!(val, Some("/notes".to_string()));
    }

    #[test]
    fn test_write_appends_new_key() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.md");
        fs::write(&path, "# task-manager config\n\nother-key: foo\n").unwrap();
        write_config_value_to(&path, "default-dir", "/notes").unwrap();
        let val = read_config_value_from(&path, "default-dir");
        assert_eq!(val, Some("/notes".to_string()));
        // Other key should still be there
        let other = read_config_value_from(&path, "other-key");
        assert_eq!(other, Some("foo".to_string()));
    }

    #[test]
    fn test_write_overwrites_existing_key() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.md");
        fs::write(&path, "default-dir: /old\n").unwrap();
        write_config_value_to(&path, "default-dir", "/new").unwrap();
        let val = read_config_value_from(&path, "default-dir");
        assert_eq!(val, Some("/new".to_string()));
    }

    #[test]
    fn test_write_empty_file_adds_header() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.md");
        write_config_value_to(&path, "default-dir", "/fresh").unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("# task-manager config"));
        assert!(content.contains("default-dir: /fresh"));
    }

    // -- list_agent_profiles_from --

    #[test]
    fn test_list_agent_profiles_empty() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.md");
        fs::write(&path, "default-dir: /tasks\n").unwrap();
        let profiles = list_agent_profiles_from(&path);
        assert!(profiles.is_empty());
    }

    #[test]
    fn test_list_agent_profiles_one() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.md");
        fs::write(&path, "agent-command-center: /code/cc\n").unwrap();
        let profiles = list_agent_profiles_from(&path);
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].0, "command-center");
        assert_eq!(profiles[0].1, "/code/cc");
    }

    #[test]
    fn test_list_agent_profiles_multiple() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.md");
        fs::write(&path, "agent-alpha: /code/alpha\nagent-beta: /code/beta\n").unwrap();
        let mut profiles = list_agent_profiles_from(&path);
        profiles.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].0, "alpha");
        assert_eq!(profiles[1].0, "beta");
    }

    // -- find_agent_for_cwd_from --

    #[test]
    fn test_find_agent_exact_match() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.md");
        fs::write(&path, "agent-myapp: /code/myapp\n").unwrap();
        let result = find_agent_for_cwd_from(&path, std::path::Path::new("/code/myapp"));
        assert_eq!(result, Some("myapp".to_string()));
    }

    #[test]
    fn test_find_agent_subdirectory_match() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.md");
        fs::write(&path, "agent-myapp: /code/myapp\n").unwrap();
        let result = find_agent_for_cwd_from(&path, std::path::Path::new("/code/myapp/src/bin"));
        assert_eq!(result, Some("myapp".to_string()));
    }

    #[test]
    fn test_find_agent_no_match() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.md");
        fs::write(&path, "agent-myapp: /code/myapp\n").unwrap();
        let result = find_agent_for_cwd_from(&path, std::path::Path::new("/code/other"));
        assert!(result.is_none());
    }

    #[test]
    fn test_find_agent_longest_match_wins() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.md");
        fs::write(&path, "agent-root: /code\nagent-specific: /code/myapp\n").unwrap();
        let result = find_agent_for_cwd_from(&path, std::path::Path::new("/code/myapp/src"));
        assert_eq!(result, Some("specific".to_string()));
    }
}
