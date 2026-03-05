use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub slug: String,
    pub title: String,
    pub body: String,
}

pub fn slugify(title: &str) -> String {
    let lower = title.to_lowercase();
    let slug: String = lower
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    // Collapse consecutive hyphens and trim leading/trailing hyphens
    let mut result = String::new();
    let mut prev_hyphen = true; // treat start as hyphen to trim leading
    for c in slug.chars() {
        if c == '-' {
            if !prev_hyphen {
                result.push('-');
            }
            prev_hyphen = true;
        } else {
            result.push(c);
            prev_hyphen = false;
        }
    }
    // Trim trailing hyphen
    if result.ends_with('-') {
        result.pop();
    }
    if result.is_empty() {
        "note".to_string()
    } else {
        result
    }
}

pub fn unique_slug(dir: &Path, base_slug: &str) -> String {
    if !dir.join(format!("{}.md", base_slug)).exists() {
        return base_slug.to_string();
    }
    let mut n = 2;
    loop {
        let candidate = format!("{}-{}", base_slug, n);
        if !dir.join(format!("{}.md", candidate)).exists() {
            return candidate;
        }
        n += 1;
    }
}

pub fn read_note(path: &Path) -> Result<Note, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read note {}: {}", path.display(), e))?;

    let slug = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut lines = content.lines();
    let title = match lines.next() {
        Some(line) => line.strip_prefix("# ").unwrap_or(line).to_string(),
        None => slug.clone(),
    };

    // Skip blank line after title
    let rest: Vec<&str> = lines.collect();
    let body = rest.join("\n");
    // Trim leading blank line
    let body = body.strip_prefix('\n').unwrap_or(&body).to_string();

    Ok(Note { slug, title, body })
}

pub fn write_note(dir: &Path, note: &Note) -> Result<PathBuf, String> {
    let file_path = dir.join(format!("{}.md", note.slug));
    let tmp_path = dir.join(format!(".note-tmp-{}", std::process::id()));

    let mut content = format!("# {}\n", note.title);
    if !note.body.is_empty() {
        content.push('\n');
        content.push_str(&note.body);
        if !note.body.ends_with('\n') {
            content.push('\n');
        }
    }

    let file = fs::File::create(&tmp_path)
        .map_err(|e| format!("Failed to create temp note file: {}", e))?;
    let mut writer = std::io::BufWriter::new(file);
    writer
        .write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write note: {}", e))?;
    writer
        .flush()
        .map_err(|e| format!("Failed to flush note: {}", e))?;
    drop(writer);

    fs::rename(&tmp_path, &file_path)
        .map_err(|e| format!("Failed to rename note temp file: {}", e))?;

    Ok(file_path)
}

pub fn discover_notes(dir: &Path, task_filename: &str) -> Vec<Note> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut notes: Vec<Note> = Vec::new();
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        if !name.ends_with(".md") {
            continue;
        }
        // Exclude the task file and backup files
        if name == task_filename || name.starts_with(".") || name.starts_with("tasks-") {
            continue;
        }
        if let Ok(note) = read_note(&path) {
            notes.push(note);
        }
    }
    notes.sort_by(|a, b| a.slug.cmp(&b.slug));
    notes
}

pub fn delete_note(dir: &Path, slug: &str) -> Result<(), String> {
    let path = dir.join(format!("{}.md", slug));
    fs::remove_file(&path).map_err(|e| format!("Failed to delete note {}: {}", slug, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // -- slugify tests --

    #[test]
    fn test_slugify_basic() {
        assert_eq!(slugify("Meeting Notes"), "meeting-notes");
    }

    #[test]
    fn test_slugify_special_chars() {
        assert_eq!(slugify("My Project Plan!"), "my-project-plan");
    }

    #[test]
    fn test_slugify_multiple_spaces() {
        assert_eq!(slugify("Hello   World"), "hello-world");
    }

    #[test]
    fn test_slugify_leading_trailing() {
        assert_eq!(slugify("  Hello  "), "hello");
    }

    #[test]
    fn test_slugify_empty() {
        assert_eq!(slugify(""), "note");
    }

    #[test]
    fn test_slugify_only_special() {
        assert_eq!(slugify("!!!"), "note");
    }

    // -- unique_slug tests --

    #[test]
    fn test_unique_slug_no_collision() {
        let dir = tempdir().unwrap();
        assert_eq!(unique_slug(dir.path(), "my-note"), "my-note");
    }

    #[test]
    fn test_unique_slug_collision() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("my-note.md"), "# My Note\n").unwrap();
        assert_eq!(unique_slug(dir.path(), "my-note"), "my-note-2");
    }

    #[test]
    fn test_unique_slug_multiple_collisions() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("my-note.md"), "# My Note\n").unwrap();
        fs::write(dir.path().join("my-note-2.md"), "# My Note\n").unwrap();
        assert_eq!(unique_slug(dir.path(), "my-note"), "my-note-3");
    }

    // -- read/write tests --

    #[test]
    fn test_write_and_read_note() {
        let dir = tempdir().unwrap();
        let note = Note {
            slug: "test-note".to_string(),
            title: "Test Note".to_string(),
            body: "Some content here.".to_string(),
        };
        write_note(dir.path(), &note).unwrap();

        let read = read_note(&dir.path().join("test-note.md")).unwrap();
        assert_eq!(read.slug, "test-note");
        assert_eq!(read.title, "Test Note");
        assert_eq!(read.body, "Some content here.");
    }

    #[test]
    fn test_write_and_read_empty_body() {
        let dir = tempdir().unwrap();
        let note = Note {
            slug: "empty".to_string(),
            title: "Empty Note".to_string(),
            body: String::new(),
        };
        write_note(dir.path(), &note).unwrap();

        let read = read_note(&dir.path().join("empty.md")).unwrap();
        assert_eq!(read.title, "Empty Note");
        assert_eq!(read.body, "");
    }

    #[test]
    fn test_write_multiline_body() {
        let dir = tempdir().unwrap();
        let note = Note {
            slug: "multi".to_string(),
            title: "Multi".to_string(),
            body: "Line 1\nLine 2\nLine 3".to_string(),
        };
        write_note(dir.path(), &note).unwrap();

        let read = read_note(&dir.path().join("multi.md")).unwrap();
        assert_eq!(read.body, "Line 1\nLine 2\nLine 3");
    }

    // -- discover tests --

    #[test]
    fn test_discover_notes_excludes_task_file() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("tasks.md"), "# Tasks\n").unwrap();
        fs::write(dir.path().join("my-note.md"), "# My Note\n\nContent.\n").unwrap();
        fs::write(dir.path().join("other.md"), "# Other\n\nStuff.\n").unwrap();

        let notes = discover_notes(dir.path(), "tasks.md");
        assert_eq!(notes.len(), 2);
        assert!(notes.iter().any(|n| n.slug == "my-note"));
        assert!(notes.iter().any(|n| n.slug == "other"));
    }

    #[test]
    fn test_discover_notes_empty_dir() {
        let dir = tempdir().unwrap();
        let notes = discover_notes(dir.path(), "tasks.md");
        assert!(notes.is_empty());
    }

    #[test]
    fn test_discover_notes_only_task_file() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("tasks.md"), "# Tasks\n").unwrap();
        let notes = discover_notes(dir.path(), "tasks.md");
        assert!(notes.is_empty());
    }

    // -- delete tests --

    #[test]
    fn test_delete_note() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("doomed.md"), "# Doomed\n").unwrap();
        assert!(dir.path().join("doomed.md").exists());
        delete_note(dir.path(), "doomed").unwrap();
        assert!(!dir.path().join("doomed.md").exists());
    }

    #[test]
    fn test_delete_nonexistent_note() {
        let dir = tempdir().unwrap();
        assert!(delete_note(dir.path(), "nope").is_err());
    }
}
