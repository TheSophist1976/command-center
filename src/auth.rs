use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

pub fn token_path() -> PathBuf {
    let base = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"));
    base.join("task-manager").join("todoist_token")
}

pub fn read_token() -> Option<String> {
    let path = token_path();
    fs::read_to_string(&path).ok().map(|s| s.trim().to_string())
}

pub fn write_token(token: &str) -> Result<(), String> {
    let path = token_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    fs::write(&path, token)
        .map_err(|e| format!("Failed to write token: {}", e))?;

    // Set 0600 permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("Failed to set token file permissions: {}", e))?;
    }

    Ok(())
}

// -- Claude API key --

pub fn claude_key_path() -> PathBuf {
    let base = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"));
    base.join("task-manager").join("claude_api_key")
}

pub fn read_claude_key() -> Option<String> {
    // Env var takes precedence
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        let trimmed = key.trim().to_string();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }
    let path = claude_key_path();
    fs::read_to_string(&path).ok().map(|s| s.trim().to_string())
}

pub fn read_claude_key_source() -> Option<(&'static str, String)> {
    if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
        let trimmed = key.trim().to_string();
        if !trimmed.is_empty() {
            return Some(("env", trimmed));
        }
    }
    let path = claude_key_path();
    fs::read_to_string(&path).ok().map(|s| ("file", s.trim().to_string()))
}

pub fn write_claude_key(key: &str) -> Result<(), String> {
    let path = claude_key_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    fs::write(&path, key)
        .map_err(|e| format!("Failed to write Claude API key: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("Failed to set key file permissions: {}", e))?;
    }

    Ok(())
}

pub fn delete_claude_key() -> Result<bool, String> {
    let path = claude_key_path();
    if path.exists() {
        fs::remove_file(&path)
            .map_err(|e| format!("Failed to delete Claude API key: {}", e))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn prompt_for_claude_key(key_flag: Option<String>) -> Result<String, String> {
    if let Some(k) = key_flag {
        let trimmed = k.trim().to_string();
        if trimmed.is_empty() {
            return Err("API key cannot be empty.".to_string());
        }
        return Ok(trimmed);
    }

    println!("Get your API key at:");
    println!("  https://console.anthropic.com/settings/keys");
    println!();
    print!("Paste your Claude API key: ");
    io::stdout().flush().map_err(|e| format!("Failed to flush stdout: {}", e))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("Failed to read key: {}", e))?;

    let trimmed = input.trim().to_string();
    if trimmed.is_empty() {
        return Err("API key cannot be empty.".to_string());
    }
    Ok(trimmed)
}

// -- Todoist token --

pub fn delete_token() -> Result<bool, String> {
    let path = token_path();
    if path.exists() {
        fs::remove_file(&path)
            .map_err(|e| format!("Failed to delete token: {}", e))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn prompt_for_token(token_flag: Option<String>) -> Result<String, String> {
    if let Some(t) = token_flag {
        let trimmed = t.trim().to_string();
        if trimmed.is_empty() {
            return Err("Token cannot be empty.".to_string());
        }
        return Ok(trimmed);
    }

    println!("Find your personal API token at:");
    println!("  https://app.todoist.com/app/settings/integrations/developer");
    println!();
    print!("Paste your Todoist API token: ");
    io::stdout().flush().map_err(|e| format!("Failed to flush stdout: {}", e))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("Failed to read token: {}", e))?;

    let trimmed = input.trim().to_string();
    if trimmed.is_empty() {
        return Err("Token cannot be empty.".to_string());
    }
    Ok(trimmed)
}

// Token operations using a custom path (for testing)
#[cfg(test)]
pub fn read_token_from(path: &std::path::Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

pub fn write_token_to(path: &std::path::Path, token: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    fs::write(path, token)
        .map_err(|e| format!("Failed to write token: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("Failed to set token file permissions: {}", e))?;
    }

    Ok(())
}

pub fn delete_token_at(path: &std::path::Path) -> Result<bool, String> {
    if path.exists() {
        fs::remove_file(path)
            .map_err(|e| format!("Failed to delete token: {}", e))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // -- token_path --

    #[test]
    fn test_token_path_returns_path() {
        let path = token_path();
        // Should end with task-manager/todoist_token
        assert!(path.to_string_lossy().contains("task-manager"));
        assert!(path.to_string_lossy().contains("todoist_token"));
    }

    // -- prompt_for_token with flag --

    #[test]
    fn test_prompt_for_token_with_valid_flag() {
        let result = prompt_for_token(Some("mytoken123".to_string()));
        assert_eq!(result.unwrap(), "mytoken123");
    }

    #[test]
    fn test_prompt_for_token_with_whitespace_trimmed() {
        let result = prompt_for_token(Some("  mytoken  ".to_string()));
        assert_eq!(result.unwrap(), "mytoken");
    }

    #[test]
    fn test_prompt_for_token_empty_flag_rejected() {
        let result = prompt_for_token(Some("".to_string()));
        assert!(result.is_err());
        let err = result.unwrap_err();
        // Error message is "Token cannot be empty."
        assert!(err.contains("empty"));
    }

    #[test]
    fn test_prompt_for_token_whitespace_only_rejected() {
        let result = prompt_for_token(Some("   ".to_string()));
        assert!(result.is_err());
        let err = result.unwrap_err();
        // Error message is "Token cannot be empty."
        assert!(err.contains("empty"));
    }

    // -- write_token_to / read_token_from --

    #[test]
    fn test_write_and_read_token() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("subdir").join("todoist_token");
        write_token_to(&path, "testtoken").unwrap();
        assert!(path.exists());
        let read = read_token_from(&path);
        assert_eq!(read, Some("testtoken".to_string()));
    }

    #[test]
    fn test_write_token_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("a").join("b").join("c").join("token");
        write_token_to(&path, "abc").unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_read_token_from_nonexistent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent_token");
        let result = read_token_from(&path);
        assert!(result.is_none());
    }

    #[test]
    fn test_read_token_trims_whitespace() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("token");
        fs::write(&path, "  mytoken  \n").unwrap();
        let result = read_token_from(&path);
        assert_eq!(result, Some("mytoken".to_string()));
    }

    // -- delete_token_at --

    #[test]
    fn test_delete_existing_token() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("token");
        fs::write(&path, "sometoken").unwrap();
        let result = delete_token_at(&path).unwrap();
        assert!(result);
        assert!(!path.exists());
    }

    #[test]
    fn test_delete_nonexistent_token() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent");
        let result = delete_token_at(&path).unwrap();
        assert!(!result);
    }

    // -- write_token / read_token / delete_token (public API using token_path) --

    #[test]
    fn test_write_token_sets_permissions() {
        // We can test write_token creates valid content (permissions are a side effect on unix)
        let dir = tempdir().unwrap();
        let path = dir.path().join("token");
        write_token_to(&path, "secure_token").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let meta = std::fs::metadata(&path).unwrap();
            let mode = meta.permissions().mode();
            // 0600 means owner rw only
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    // Tests for the public API (write_token/read_token/delete_token) that use token_path()
    // These use the real config dir. We call write_token first, then read and delete.
    // Note: these tests affect the system config dir, but are idempotent.

    #[test]
    fn test_write_read_delete_token_public_api() {
        // Write token via public API
        let result = write_token("test_coverage_token_12345");
        assert!(result.is_ok());

        // Read back
        let token = read_token();
        // Token should be present (we just wrote it)
        assert!(token.is_some());
        assert_eq!(token.unwrap(), "test_coverage_token_12345");

        // Delete it
        let deleted = delete_token().unwrap();
        assert!(deleted);

        // Now token should be gone
        // (subsequent delete should return false)
        let deleted_again = delete_token().unwrap();
        assert!(!deleted_again);
    }

    // -- write_token_to with no parent path (covers the None branch of if let Some(parent)) --

    #[test]
    fn test_write_token_to_no_parent_path() {
        // Path::new("") has no parent (parent() returns None)
        // This covers the `if let Some(parent)` None branch (line 21/83)
        // The write itself will fail because "" is not a valid file path
        let result = write_token_to(std::path::Path::new(""), "token");
        assert!(result.is_err());
    }

    // -- Error path tests for write_token_to --

    #[test]
    #[cfg(unix)]
    fn test_write_token_to_readonly_parent() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let readonly_dir = dir.path().join("readonly");
        fs::create_dir(&readonly_dir).unwrap();
        // Make the directory readonly
        fs::set_permissions(&readonly_dir, fs::Permissions::from_mode(0o444)).unwrap();

        let path = readonly_dir.join("token");
        let result = write_token_to(&path, "token");
        // Should fail due to permission denied when creating parent or writing
        assert!(result.is_err());

        // Restore permissions for cleanup
        let _ = fs::set_permissions(&readonly_dir, fs::Permissions::from_mode(0o755));
    }

    // -- write_token_to create_dir_all error path (line 82) --

    #[test]
    #[cfg(unix)]
    fn test_write_token_to_create_dir_all_fails() {
        // Covers line 82: create_dir_all error when parent needs to be created under a readonly dir
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let readonly_root = dir.path().join("readonly_root");
        fs::create_dir(&readonly_root).unwrap();
        // Make readonly_root itself read-only, so sub-dirs can't be created
        fs::set_permissions(&readonly_root, fs::Permissions::from_mode(0o555)).unwrap();

        // Path requires creating a subdirectory under readonly_root
        let path = readonly_root.join("newsubdir").join("token");
        let result = write_token_to(&path, "token");

        // Restore permissions for cleanup
        let _ = fs::set_permissions(&readonly_root, fs::Permissions::from_mode(0o755));

        // create_dir_all should fail because readonly_root is not writable
        assert!(result.is_err(), "Expected write_token_to to fail on readonly dir");
    }

    // -- delete_token_at error path (line 100) --

    #[test]
    #[cfg(unix)]
    fn test_delete_token_at_readonly_parent() {
        // Covers line 100: remove_file fails when parent dir is readonly
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let readonly_dir = dir.path().join("readonly");
        fs::create_dir(&readonly_dir).unwrap();
        // Create the token file
        let token_path = readonly_dir.join("token");
        fs::write(&token_path, "mytoken").unwrap();
        // Make parent dir read-only so deletion fails
        fs::set_permissions(&readonly_dir, fs::Permissions::from_mode(0o555)).unwrap();

        let result = delete_token_at(&token_path);

        // Restore permissions for cleanup
        let _ = fs::set_permissions(&readonly_dir, fs::Permissions::from_mode(0o755));

        // Should fail because the token file can't be removed from readonly dir
        assert!(result.is_err(), "Expected delete_token_at to fail on readonly parent dir");
    }

    // -- Claude key tests --

    #[test]
    fn test_claude_key_path_returns_path() {
        let path = claude_key_path();
        assert!(path.to_string_lossy().contains("task-manager"));
        assert!(path.to_string_lossy().contains("claude_api_key"));
    }

    #[test]
    fn test_write_read_delete_claude_key() {
        let result = write_claude_key("test_claude_key_12345");
        assert!(result.is_ok());

        // Temporarily unset the env var to test file-based read
        let original = std::env::var("ANTHROPIC_API_KEY").ok();
        unsafe { std::env::remove_var("ANTHROPIC_API_KEY"); }

        let key = read_claude_key();
        assert!(key.is_some());
        assert_eq!(key.unwrap(), "test_claude_key_12345");

        let deleted = delete_claude_key().unwrap();
        assert!(deleted);

        let deleted_again = delete_claude_key().unwrap();
        assert!(!deleted_again);

        // Restore env var if it was set
        if let Some(val) = original {
            unsafe { std::env::set_var("ANTHROPIC_API_KEY", val); }
        }
    }

    #[test]
    fn test_prompt_for_claude_key_with_valid_flag() {
        let result = prompt_for_claude_key(Some("sk-ant-test123".to_string()));
        assert_eq!(result.unwrap(), "sk-ant-test123");
    }

    #[test]
    fn test_prompt_for_claude_key_empty_flag_rejected() {
        let result = prompt_for_claude_key(Some("".to_string()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_prompt_for_claude_key_whitespace_trimmed() {
        let result = prompt_for_claude_key(Some("  sk-ant-test  ".to_string()));
        assert_eq!(result.unwrap(), "sk-ant-test");
    }

    #[test]
    fn test_read_claude_key_source_from_file() {
        let result = write_claude_key("test_source_key_99");
        assert!(result.is_ok());

        let original = std::env::var("ANTHROPIC_API_KEY").ok();
        unsafe { std::env::remove_var("ANTHROPIC_API_KEY"); }

        let source = read_claude_key_source();
        assert!(source.is_some());
        let (src, key) = source.unwrap();
        assert_eq!(src, "file");
        assert_eq!(key, "test_source_key_99");

        let _ = delete_claude_key();
        if let Some(val) = original {
            unsafe { std::env::set_var("ANTHROPIC_API_KEY", val); }
        }
    }
}
