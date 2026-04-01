use chrono::{Datelike, DateTime, Duration, NaiveDate, Utc, Weekday};
use std::str::FromStr;

use crate::task::{Priority, Recurrence, Status, Task, TaskFile};

#[derive(Debug)]
pub struct ParseError {
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

pub fn parse(content: &str, strict: bool) -> Result<TaskFile, Vec<ParseError>> {
    let mut task_file = TaskFile::new();
    let mut errors: Vec<ParseError> = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;

    // Parse header comments
    while i < lines.len() {
        let line = lines[i].trim();
        if line.starts_with("<!-- format:") && line.ends_with("-->") {
            if let Some(val) = extract_header_value(line, "format") {
                if let Ok(v) = val.parse::<u32>() {
                    if v != 1 && v != 2 {
                        let err = ParseError {
                            line: i + 1,
                            message: format!("Unsupported format version: {}", v),
                        };
                        if strict {
                            errors.push(err);
                            return Err(errors);
                        }
                    }
                    task_file.format_version = v;
                }
            }
        } else if line.starts_with("<!-- next-id:") && line.ends_with("-->") {
            if let Some(val) = extract_header_value(line, "next-id") {
                if let Ok(v) = val.parse::<u32>() {
                    task_file.next_id = v;
                }
            }
        } else if line.starts_with("## ") {
            // We've hit the first task heading, stop parsing headers
            break;
        }
        i += 1;
    }

    // Parse tasks
    while i < lines.len() {
        let line = lines[i];
        if let Some((status, title)) = parse_task_heading(line) {
            let heading_line = i + 1;
            i += 1;

            // Look for metadata comment — skip any blank lines between heading and comment
            while i < lines.len() && lines[i].trim().is_empty() {
                i += 1;
            }
            if i < lines.len() {
                if let Some(metadata) = parse_metadata_comment(lines[i]) {
                    i += 1;

                    // Capture description body
                    let mut desc_lines: Vec<&str> = Vec::new();
                    while i < lines.len() && !lines[i].starts_with("## ") {
                        desc_lines.push(lines[i]);
                        i += 1;
                    }
                    let description = {
                        let text = desc_lines.join("\n").trim().to_string();
                        if text.is_empty() { None } else { Some(text) }
                    };

                    task_file.tasks.push(Task {
                        id: metadata.id,
                        title,
                        status,
                        priority: metadata.priority,
                        tags: metadata.tags,
                        created: metadata.created,
                        updated: metadata.updated,
                        description,
                        due_date: metadata.due_date,
                        project: metadata.project,
                        recurrence: metadata.recurrence,
                        note: metadata.note,
                        agent: metadata.agent,
                    });
                } else if strict {
                    errors.push(ParseError {
                        line: heading_line,
                        message: format!("Task '{}' has no metadata comment", title),
                    });
                    // Skip to next heading
                    while i < lines.len() && !lines[i].starts_with("## ") {
                        i += 1;
                    }
                } else {
                    // Tolerant mode: skip task without metadata
                    while i < lines.len() && !lines[i].starts_with("## ") {
                        i += 1;
                    }
                }
            }
        } else {
            // Not a task heading
            if strict && line.starts_with("## ") {
                errors.push(ParseError {
                    line: i + 1,
                    message: format!("H2 heading does not match task format: {}", line),
                });
            }
            i += 1;
        }
    }

    // If no next-id header was found, derive from max task id
    if task_file.next_id == 1 && !task_file.tasks.is_empty() {
        let max_id = task_file.tasks.iter().map(|t| t.id).max().unwrap_or(0);
        if max_id >= task_file.next_id {
            task_file.next_id = max_id + 1;
        }
    }

    if strict && !errors.is_empty() {
        Err(errors)
    } else {
        Ok(task_file)
    }
}

fn extract_header_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let prefix = format!("<!-- {}:", key);
    let stripped = line.strip_prefix(&prefix)?;
    let val = stripped.strip_suffix("-->")?;
    Some(val.trim())
}

fn parse_task_heading(line: &str) -> Option<(Status, String)> {
    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix("## [x] ") {
        Some((Status::Done, rest.trim().to_string()))
    } else if let Some(rest) = trimmed.strip_prefix("## [X] ") {
        Some((Status::Done, rest.trim().to_string()))
    } else if let Some(rest) = trimmed.strip_prefix("## [ ] ") {
        Some((Status::Open, rest.trim().to_string()))
    } else {
        None
    }
}

struct Metadata {
    id: u32,
    priority: Priority,
    tags: Vec<String>,
    created: DateTime<Utc>,
    updated: Option<DateTime<Utc>>,
    due_date: Option<NaiveDate>,
    project: Option<String>,
    recurrence: Option<Recurrence>,
    note: Option<String>,
    agent: Option<String>,
}

fn parse_metadata_comment(line: &str) -> Option<Metadata> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix("<!--")?.strip_suffix("-->")?.trim();

    let mut id: Option<u32> = None;
    let mut priority = Priority::Medium;
    let mut tags: Vec<String> = Vec::new();
    let mut created: Option<DateTime<Utc>> = None;
    let mut updated: Option<DateTime<Utc>> = None;
    let mut due_date: Option<NaiveDate> = None;
    let mut project: Option<String> = None;
    let mut recurrence: Option<Recurrence> = None;
    let mut note: Option<String> = None;
    let mut agent: Option<String> = None;

    for pair in inner.split_whitespace() {
        if let Some((key, rest)) = pair.split_once(':') {
            match key {
                "id" => id = rest.parse().ok(),
                "priority" => priority = Priority::from_str(rest).unwrap_or(Priority::Medium),
                "tags" => {
                    tags = rest.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
                "created" => created = DateTime::parse_from_rfc3339(rest).ok().map(|dt| dt.with_timezone(&Utc)),
                "updated" => updated = DateTime::parse_from_rfc3339(rest).ok().map(|dt| dt.with_timezone(&Utc)),
                "due" => due_date = NaiveDate::parse_from_str(rest, "%Y-%m-%d").ok(),
                "project" => project = Some(rest.replace("%20", " ").replace("%3A", ":")),
                "recur" => {
                    // The recur value may contain colons (e.g., "monthly:3:thu"),
                    // but split_once on the first ':' only gives us "recur" and the rest.
                    // However, the outer split_whitespace splits by whitespace, so
                    // "recur:monthly:3:thu" is one token. split_once(':') gives key="recur", rest="monthly:3:thu".
                    recurrence = Recurrence::from_str(rest).ok();
                }
                "note" => {
                    if !rest.is_empty() {
                        note = Some(rest.to_string());
                    }
                }
                "agent" => {
                    if !rest.is_empty() {
                        agent = Some(rest.to_string());
                    }
                }
                _ => {}
            }
        }
    }

    Some(Metadata {
        id: id?,
        priority,
        tags,
        created: created.unwrap_or_else(Utc::now),
        updated,
        due_date,
        project,
        recurrence,
        note,
        agent,
    })
}

pub fn serialize(task_file: &TaskFile) -> String {
    let mut out = String::new();

    // Header — always write format:2
    out.push_str("<!-- format:2 -->\n");
    out.push_str(&format!("<!-- next-id:{} -->\n", task_file.next_id));
    out.push_str("\n# Tasks\n");

    for task in &task_file.tasks {
        out.push('\n');
        let checkbox = match task.status {
            Status::Open => "[ ]",
            Status::Done => "[x]",
        };
        out.push_str(&format!("## {} {}\n", checkbox, task.title));

        // Metadata comment
        let mut meta_parts = vec![
            format!("id:{}", task.id),
            format!("priority:{}", task.priority),
        ];
        if !task.tags.is_empty() {
            meta_parts.push(format!("tags:{}", task.tags.join(",")));
        }
        if let Some(due) = task.due_date {
            meta_parts.push(format!("due:{}", due.format("%Y-%m-%d")));
        }
        if let Some(ref proj) = task.project {
            let encoded = proj.replace('%', "%25").replace(' ', "%20").replace(':', "%3A");
            meta_parts.push(format!("project:{}", encoded));
        }
        if let Some(ref recur) = task.recurrence {
            meta_parts.push(format!("recur:{}", recur));
        }
        if let Some(ref note_slug) = task.note {
            meta_parts.push(format!("note:{}", note_slug));
        }
        if let Some(ref agent) = task.agent {
            meta_parts.push(format!("agent:{}", agent));
        }
        meta_parts.push(format!("created:{}", task.created.to_rfc3339()));
        if let Some(updated) = task.updated {
            meta_parts.push(format!("updated:{}", updated.to_rfc3339()));
        }
        out.push_str(&format!("<!-- {} -->\n", meta_parts.join(" ")));

        if let Some(ref desc) = task.description {
            out.push('\n');
            out.push_str(desc);
            out.push('\n');
        }
    }

    out
}

/// Parse a due date input string.
///
/// Accepts:
/// - ISO dates: `"2026-04-15"` → returns that date
/// - Full weekday names: `"monday"` … `"sunday"` (case-insensitive)
/// - Three-letter abbreviations: `"mon"` … `"sun"` (case-insensitive)
///
/// Weekday names resolve to the **next future occurrence** after `today` — never
/// today itself (same-day input advances by 7 days to the following week).
///
/// Returns `None` for empty or unrecognized input.
pub fn parse_due_date_input(s: &str, today: NaiveDate) -> Option<NaiveDate> {
    if s.is_empty() {
        return None;
    }

    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(d);
    }

    let weekday: Weekday = match s.to_lowercase().as_str() {
        "monday" | "mon" => Weekday::Mon,
        "tuesday" | "tue" => Weekday::Tue,
        "wednesday" | "wed" => Weekday::Wed,
        "thursday" | "thu" => Weekday::Thu,
        "friday" | "fri" => Weekday::Fri,
        "saturday" | "sat" => Weekday::Sat,
        "sunday" | "sun" => Weekday::Sun,
        _ => return None,
    };

    let today_num = today.weekday().number_from_monday() as i64;
    let target_num = weekday.number_from_monday() as i64;
    let diff = (target_num - today_num + 7) % 7;
    let days_ahead = if diff == 0 { 7 } else { diff };

    today.checked_add_signed(Duration::days(days_ahead))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_string() {
        let tf = parse("", false).unwrap();
        assert_eq!(tf.tasks.len(), 0);
    }

    #[test]
    fn test_parse_well_formed() {
        let content = r#"<!-- format:1 -->
<!-- next-id:3 -->

# Tasks

## [ ] Build the login page
<!-- id:1 priority:high tags:frontend,auth created:2025-01-15T10:00:00+00:00 -->

Some description here.

## [x] Set up CI pipeline
<!-- id:2 priority:medium tags:infra created:2025-01-10T08:00:00+00:00 -->
"#;
        let tf = parse(content, false).unwrap();
        assert_eq!(tf.format_version, 1);
        assert_eq!(tf.next_id, 3);
        assert_eq!(tf.tasks.len(), 2);
        assert_eq!(tf.tasks[0].title, "Build the login page");
        assert_eq!(tf.tasks[0].status, Status::Open);
        assert_eq!(tf.tasks[0].priority, Priority::High);
        assert_eq!(tf.tasks[0].tags, vec!["frontend", "auth"]);
        assert_eq!(tf.tasks[0].description, Some("Some description here.".to_string()));
        assert_eq!(tf.tasks[1].title, "Set up CI pipeline");
        assert_eq!(tf.tasks[1].status, Status::Done);
    }

    #[test]
    fn test_parse_malformed_tolerant() {
        let content = r#"<!-- format:1 -->
<!-- next-id:2 -->

# Tasks

## Some random heading

## [ ] Valid task
<!-- id:1 priority:low created:2025-01-15T10:00:00+00:00 -->
"#;
        let tf = parse(content, false).unwrap();
        assert_eq!(tf.tasks.len(), 1);
        assert_eq!(tf.tasks[0].title, "Valid task");
    }

    #[test]
    fn test_parse_strict_errors() {
        let content = r#"<!-- format:1 -->
<!-- next-id:2 -->

# Tasks

## Some random heading

## [ ] Valid task
<!-- id:1 priority:low created:2025-01-15T10:00:00+00:00 -->
"#;
        let result = parse(content, true);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_parse_missing_header() {
        let content = r#"# Tasks

## [ ] Only task
<!-- id:5 priority:medium created:2025-01-15T10:00:00+00:00 -->
"#;
        let tf = parse(content, false).unwrap();
        assert_eq!(tf.tasks.len(), 1);
        assert_eq!(tf.next_id, 6); // derived from max id
    }

    #[test]
    fn test_round_trip() {
        let content = r#"<!-- format:1 -->
<!-- next-id:3 -->

# Tasks

## [ ] Build the login page
<!-- id:1 priority:high tags:frontend,auth created:2025-01-15T10:00:00+00:00 -->

Some description here.

## [x] Set up CI pipeline
<!-- id:2 priority:medium tags:infra created:2025-01-10T08:00:00+00:00 -->
"#;
        let tf = parse(content, false).unwrap();
        let serialized = serialize(&tf);
        let tf2 = parse(&serialized, false).unwrap();
        assert_eq!(tf.tasks.len(), tf2.tasks.len());
        for (a, b) in tf.tasks.iter().zip(tf2.tasks.iter()) {
            assert_eq!(a.id, b.id);
            assert_eq!(a.title, b.title);
            assert_eq!(a.status, b.status);
            assert_eq!(a.priority, b.priority);
            assert_eq!(a.tags, b.tags);
            assert_eq!(a.description, b.description);
        }
    }

    #[test]
    fn test_parse_uppercase_x_done() {
        // [X] (uppercase) should also be parsed as done
        let content = "<!-- format:1 -->\n<!-- next-id:2 -->\n\n## [X] Done task\n<!-- id:1 priority:medium created:2025-01-15T10:00:00+00:00 -->\n";
        let tf = parse(content, false).unwrap();
        assert_eq!(tf.tasks.len(), 1);
        assert_eq!(tf.tasks[0].status, Status::Done);
    }

    #[test]
    fn test_parse_task_without_metadata_tolerant() {
        let content = "<!-- format:1 -->\n<!-- next-id:2 -->\n\n## [ ] No metadata task\nsome body text\n";
        let tf = parse(content, false).unwrap();
        // Tolerant mode skips tasks without metadata
        assert_eq!(tf.tasks.len(), 0);
    }

    #[test]
    fn test_parse_task_without_metadata_strict() {
        let content = "<!-- format:1 -->\n<!-- next-id:2 -->\n\n## [ ] No metadata task\nsome body text\n\n## [ ] Valid task\n<!-- id:1 priority:medium created:2025-01-15T10:00:00+00:00 -->\n";
        let result = parse(content, true);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
        assert!(errors[0].to_string().contains("no metadata"));
    }

    #[test]
    fn test_parse_error_display() {
        let err = ParseError { line: 5, message: "test error".to_string() };
        assert_eq!(format!("{}", err), "line 5: test error");
    }

    #[test]
    fn test_parse_unsupported_format_version_non_strict() {
        // Unsupported version should be tolerated in non-strict mode
        let content = "<!-- format:99 -->\n<!-- next-id:1 -->\n\n";
        let tf = parse(content, false).unwrap();
        assert_eq!(tf.format_version, 99);
    }

    #[test]
    fn test_parse_unsupported_format_version_strict() {
        let content = "<!-- format:99 -->\n<!-- next-id:1 -->\n\n";
        let result = parse(content, true);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("Unsupported format version"));
    }

    #[test]
    fn test_parse_format_version_2() {
        // format:2 is explicitly supported
        let content = "<!-- format:2 -->\n<!-- next-id:1 -->\n\n";
        let tf = parse(content, false).unwrap();
        assert_eq!(tf.format_version, 2);
    }

    #[test]
    fn test_parse_missing_next_id_with_no_tasks() {
        // No next-id header and no tasks: next_id stays 1
        let content = "<!-- format:1 -->\n\n# Tasks\n";
        let tf = parse(content, false).unwrap();
        assert_eq!(tf.next_id, 1);
    }

    #[test]
    fn test_parse_missing_optional_fields() {
        let content = "<!-- format:1 -->\n<!-- next-id:2 -->\n\n## [ ] Minimal task\n<!-- id:1 created:2025-01-15T10:00:00+00:00 -->\n";
        let tf = parse(content, false).unwrap();
        assert_eq!(tf.tasks.len(), 1);
        assert!(tf.tasks[0].tags.is_empty());
        assert!(tf.tasks[0].updated.is_none());
        assert_eq!(tf.tasks[0].priority, Priority::Medium); // default
    }

    #[test]
    fn test_parse_with_updated_timestamp() {
        let content = "<!-- format:1 -->\n<!-- next-id:2 -->\n\n## [ ] Task\n<!-- id:1 priority:high created:2025-01-15T10:00:00+00:00 updated:2025-02-01T12:00:00+00:00 -->\n";
        let tf = parse(content, false).unwrap();
        assert!(tf.tasks[0].updated.is_some());
    }

    #[test]
    fn test_parse_with_due_date() {
        let content = "<!-- format:1 -->\n<!-- next-id:2 -->\n\n## [ ] Task\n<!-- id:1 priority:medium created:2025-01-15T10:00:00+00:00 due:2025-12-31 -->\n";
        let tf = parse(content, false).unwrap();
        assert!(tf.tasks[0].due_date.is_some());
        let due = tf.tasks[0].due_date.unwrap();
        assert_eq!(due.to_string(), "2025-12-31");
    }

    #[test]
    fn test_parse_with_project() {
        let content = "<!-- format:2 -->\n<!-- next-id:2 -->\n\n## [ ] Task\n<!-- id:1 priority:medium created:2025-01-15T10:00:00+00:00 project:myproject -->\n";
        let tf = parse(content, false).unwrap();
        assert_eq!(tf.tasks[0].project, Some("myproject".to_string()));
    }

    #[test]
    fn test_parse_project_with_encoding() {
        // Projects with spaces use %20 encoding
        let content = "<!-- format:2 -->\n<!-- next-id:2 -->\n\n## [ ] Task\n<!-- id:1 priority:medium created:2025-01-15T10:00:00+00:00 project:My%20Project -->\n";
        let tf = parse(content, false).unwrap();
        assert_eq!(tf.tasks[0].project, Some("My Project".to_string()));
    }

    #[test]
    fn test_serialize_includes_format_version_2() {
        let tf = TaskFile::new();
        let out = serialize(&tf);
        assert!(out.contains("<!-- format:2 -->"));
    }

    #[test]
    fn test_serialize_includes_next_id() {
        let mut tf = TaskFile::new();
        tf.next_id = 5;
        let out = serialize(&tf);
        assert!(out.contains("<!-- next-id:5 -->"));
    }

    #[test]
    fn test_serialize_open_task() {
        use chrono::Utc;
        use crate::task::{Priority, Status, Task};
        let mut tf = TaskFile::new();
        tf.tasks.push(Task {
            id: 1,
            title: "Open task".to_string(),
            status: Status::Open,
            priority: Priority::Medium,
            tags: Vec::new(),
            created: Utc::now(),
            updated: None,
            description: None,
            due_date: None,
            project: None,
            recurrence: None,
            note: None,
            agent: None,
        });
        let out = serialize(&tf);
        assert!(out.contains("## [ ] Open task"));
    }

    #[test]
    fn test_serialize_done_task() {
        use chrono::Utc;
        use crate::task::{Priority, Status, Task};
        let mut tf = TaskFile::new();
        tf.tasks.push(Task {
            id: 1,
            title: "Done task".to_string(),
            status: Status::Done,
            priority: Priority::High,
            tags: Vec::new(),
            created: Utc::now(),
            updated: None,
            description: None,
            due_date: None,
            project: None,
            recurrence: None,
            note: None,
            agent: None,
        });
        let out = serialize(&tf);
        assert!(out.contains("## [x] Done task"));
    }

    #[test]
    fn test_serialize_with_all_optional_fields() {
        use chrono::{NaiveDate, TimeZone, Utc};
        use crate::task::{Priority, Status, Task};
        let mut tf = TaskFile::new();
        tf.tasks.push(Task {
            id: 1,
            title: "Full task".to_string(),
            status: Status::Open,
            priority: Priority::Critical,
            tags: vec!["alpha".to_string(), "beta".to_string()],
            created: Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
            updated: Some(Utc.with_ymd_and_hms(2025, 6, 1, 0, 0, 0).unwrap()),
            description: Some("My description".to_string()),
            due_date: Some(NaiveDate::from_ymd_opt(2025, 12, 31).unwrap()),
            project: Some("My Project".to_string()),
            recurrence: None,
            note: None,
            agent: None,
        });
        let out = serialize(&tf);
        assert!(out.contains("tags:alpha,beta"));
        assert!(out.contains("due:2025-12-31"));
        // Project encoded with %20 for space
        assert!(out.contains("project:My%20Project"));
        assert!(out.contains("updated:"));
        assert!(out.contains("My description"));
    }

    #[test]
    fn test_serialize_project_with_colon() {
        use chrono::Utc;
        use crate::task::{Priority, Status, Task};
        let mut tf = TaskFile::new();
        tf.tasks.push(Task {
            id: 1,
            title: "Task".to_string(),
            status: Status::Open,
            priority: Priority::Medium,
            tags: Vec::new(),
            created: Utc::now(),
            updated: None,
            description: None,
            due_date: None,
            project: Some("Work:Project".to_string()),
            recurrence: None,
            note: None,
            agent: None,
        });
        let out = serialize(&tf);
        assert!(out.contains("project:Work%3AProject"));
    }

    #[test]
    fn test_serialize_project_roundtrip_with_special_chars() {
        use chrono::Utc;
        use crate::task::{Priority, Status, Task};
        let mut tf = TaskFile::new();
        tf.tasks.push(Task {
            id: 1,
            title: "Task".to_string(),
            status: Status::Open,
            priority: Priority::Medium,
            tags: Vec::new(),
            created: Utc::now(),
            updated: None,
            description: None,
            due_date: None,
            project: Some("My Project".to_string()),
            recurrence: None,
            note: None,
            agent: None,
        });
        let serialized = serialize(&tf);
        let parsed = parse(&serialized, false).unwrap();
        assert_eq!(parsed.tasks[0].project, Some("My Project".to_string()));
    }

    #[test]
    fn test_parse_task_with_no_description() {
        let content = "<!-- format:2 -->\n<!-- next-id:3 -->\n\n## [ ] Task A\n<!-- id:1 priority:medium created:2025-01-15T10:00:00+00:00 -->\n\n## [ ] Task B\n<!-- id:2 priority:medium created:2025-01-16T10:00:00+00:00 -->\n";
        let tf = parse(content, false).unwrap();
        assert_eq!(tf.tasks.len(), 2);
        assert!(tf.tasks[0].description.is_none());
        assert!(tf.tasks[1].description.is_none());
    }

    #[test]
    fn test_parse_no_tasks_no_next_id_derived() {
        // When there ARE tasks but no next-id, derive next-id
        let content = "# Tasks\n\n## [ ] Task\n<!-- id:10 priority:medium created:2025-01-15T10:00:00+00:00 -->\n";
        let tf = parse(content, false).unwrap();
        assert_eq!(tf.next_id, 11); // max id 10 + 1
    }

    #[test]
    fn test_parse_critical_priority() {
        let content = "<!-- format:1 -->\n<!-- next-id:2 -->\n\n## [ ] Critical task\n<!-- id:1 priority:critical created:2025-01-15T10:00:00+00:00 -->\n";
        let tf = parse(content, false).unwrap();
        assert_eq!(tf.tasks[0].priority, Priority::Critical);
    }

    #[test]
    fn test_parse_invalid_priority_defaults_medium() {
        let content = "<!-- format:1 -->\n<!-- next-id:2 -->\n\n## [ ] Task\n<!-- id:1 priority:badvalue created:2025-01-15T10:00:00+00:00 -->\n";
        let tf = parse(content, false).unwrap();
        // Invalid priority falls back to Medium
        assert_eq!(tf.tasks[0].priority, Priority::Medium);
    }

    #[test]
    fn test_parse_malformed_format_line_no_closing() {
        // format line without --> suffix - should not parse the format version
        let content = "<!-- format:1\n<!-- next-id:2 -->\n\n## [ ] Task\n<!-- id:1 priority:medium created:2025-01-15T10:00:00+00:00 -->\n";
        let tf = parse(content, false).unwrap();
        // The format line is malformed, format_version stays at default 1
        assert_eq!(tf.tasks.len(), 1);
    }

    #[test]
    fn test_parse_malformed_next_id_no_closing() {
        // next-id line without --> suffix
        let content = "<!-- format:1 -->\n<!-- next-id:5\n\n## [ ] Task\n<!-- id:1 priority:medium created:2025-01-15T10:00:00+00:00 -->\n";
        let tf = parse(content, false).unwrap();
        // next-id not parsed, derived from max task id
        assert_eq!(tf.tasks.len(), 1);
        assert_eq!(tf.next_id, 2); // derived from id:1
    }

    #[test]
    fn test_parse_metadata_with_unknown_key() {
        // An unknown key in metadata should be silently ignored
        let content = "<!-- format:1 -->\n<!-- next-id:2 -->\n\n## [ ] Task\n<!-- id:1 priority:medium created:2025-01-15T10:00:00+00:00 unknown_key:value -->\n";
        let tf = parse(content, false).unwrap();
        assert_eq!(tf.tasks.len(), 1);
        assert_eq!(tf.tasks[0].id, 1);
    }

    #[test]
    fn test_parse_metadata_without_html_comment() {
        // Metadata line that doesn't look like an HTML comment
        let content = "<!-- format:1 -->\n<!-- next-id:2 -->\n\n## [ ] Task\nid:1 priority:medium\n";
        let tf = parse(content, false).unwrap();
        // Task has no metadata comment, so it's skipped in tolerant mode
        assert_eq!(tf.tasks.len(), 0);
    }

    #[test]
    fn test_parse_tolerant_skip_with_body_content() {
        // Task with no metadata but with body lines - tolerant mode should skip all
        let content = "<!-- format:1 -->\n<!-- next-id:2 -->\n\n## [ ] Task without metadata\nSome body\nMore body\n\n## [ ] Valid task\n<!-- id:1 priority:medium created:2025-01-15T10:00:00+00:00 -->\n";
        let tf = parse(content, false).unwrap();
        // Only valid task should be included
        assert_eq!(tf.tasks.len(), 1);
        assert_eq!(tf.tasks[0].title, "Valid task");
    }

    #[test]
    fn test_parse_strict_no_metadata_with_body() {
        // Task with no metadata in strict mode
        let content = "<!-- format:1 -->\n<!-- next-id:3 -->\n\n## [ ] Task without metadata\nSome body\n\n## [ ] Valid task\n<!-- id:2 priority:medium created:2025-01-15T10:00:00+00:00 -->\n";
        let result = parse(content, true);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
    }

    #[test]
    fn test_extract_header_value_missing_prefix() {
        // parse a line that doesn't match the expected format header prefix
        let content = "<!-- badformat:1 -->\n<!-- next-id:2 -->\n\n## [ ] Task\n<!-- id:1 created:2025-01-15T10:00:00+00:00 -->\n";
        let tf = parse(content, false).unwrap();
        assert_eq!(tf.format_version, 1); // default, since format wasn't parsed
    }

    #[test]
    fn test_metadata_key_without_value() {
        // A metadata pair with no colon (split_once returns None)
        let content = "<!-- format:1 -->\n<!-- next-id:2 -->\n\n## [ ] Task\n<!-- id:1 priority:medium created:2025-01-15T10:00:00+00:00 novalue -->\n";
        let tf = parse(content, false).unwrap();
        assert_eq!(tf.tasks.len(), 1);
    }

    #[test]
    fn test_parse_next_id_non_numeric() {
        // Non-numeric next-id should result in derived next-id
        let content = "<!-- format:1 -->\n<!-- next-id:abc -->\n\n## [ ] Task\n<!-- id:3 priority:medium created:2025-01-15T10:00:00+00:00 -->\n";
        let tf = parse(content, false).unwrap();
        // next-id abc is not parsed, derived from max id 3 -> next is 4
        assert_eq!(tf.next_id, 4);
    }

    #[test]
    fn test_parse_max_id_equals_next_id_default() {
        // When max_id == next_id (1), it should still be bumped to max_id + 1
        // Tasks exist but next_id is already 1 (default) so max_id (1) >= next_id (1)
        let content = "# Tasks\n\n## [ ] Task\n<!-- id:1 priority:medium created:2025-01-15T10:00:00+00:00 -->\n";
        let tf = parse(content, false).unwrap();
        assert_eq!(tf.next_id, 2);
    }

    #[test]
    fn test_parse_format_version_non_numeric_value() {
        // Non-numeric format value: if let Ok(v) fails, format_version stays default
        let content = "<!-- format:abc -->\n<!-- next-id:2 -->\n\n## [ ] Task\n<!-- id:1 priority:medium created:2025-01-15T10:00:00+00:00 -->\n";
        let tf = parse(content, false).unwrap();
        // format:abc doesn't parse as u32, format_version stays at default 1
        assert_eq!(tf.format_version, 1);
        assert_eq!(tf.tasks.len(), 1);
    }

    #[test]
    fn test_parse_task_heading_at_end_of_file() {
        // Task heading with no following content at all (i >= lines.len() after heading)
        // This covers the `if i < lines.len()` false branch in parse()
        let content = "<!-- format:1 -->\n<!-- next-id:2 -->\n## [ ] Last task";
        let tf = parse(content, false).unwrap();
        // Task with heading at end is skipped (no metadata) in tolerant mode
        assert_eq!(tf.tasks.len(), 0);
    }

    #[test]
    fn test_parse_next_id_empty_value() {
        // Empty next-id value: extract_header_value returns Some(""), parse fails
        let content = "<!-- format:1 -->\n<!-- next-id: -->\n\n## [ ] Task\n<!-- id:5 priority:medium created:2025-01-15T10:00:00+00:00 -->\n";
        let tf = parse(content, false).unwrap();
        // next-id "" fails to parse as u32, derived from max id 5 -> next is 6
        assert_eq!(tf.next_id, 6);
    }

    // -- Covers line 124: max_id < next_id branch (inner if not taken) --
    #[test]
    fn test_parse_next_id_derivation_max_id_zero() {
        // Tasks exist but all have id=0. With no next-id header, next_id defaults to 1.
        // max_id = 0, next_id = 1, so max_id < next_id => inner if NOT taken (line 124).
        // This is a pathological case but exercises the branch.
        // We can't normally create a task with id=0 through the parser, but we can craft content.
        // Actually id:0 IS parseable by the parser.
        let content = "# Tasks\n\n## [ ] Zero id task\n<!-- id:0 priority:medium created:2025-01-15T10:00:00+00:00 -->\n";
        let tf = parse(content, false).unwrap();
        // Task exists with id=0, next_id defaults to 1
        // max_id = 0, next_id = 1, so 0 >= 1 is false => next_id stays at 1
        assert_eq!(tf.tasks.len(), 1);
        assert_eq!(tf.next_id, 1); // unchanged because max_id (0) < next_id (1)
    }

    // -- Covers line 166: parse_metadata_comment returns None for malformed comment --
    #[test]
    fn test_parse_metadata_comment_no_closing_arrow() {
        // A metadata line that starts with <!-- but doesn't end with -->
        // This causes strip_suffix("-->") to return None, so parse_metadata_comment returns None
        // In tolerant mode, the task is skipped
        let content = "<!-- format:1 -->\n<!-- next-id:2 -->\n\n## [ ] Task\n<!-- id:1 priority:medium created:2025-01-15T10:00:00+00:00\n\n## [ ] Task 2\n<!-- id:2 priority:medium created:2025-01-16T10:00:00+00:00 -->\n";
        let tf = parse(content, false).unwrap();
        // First task has malformed metadata (no -->), gets skipped
        // Second task is valid
        assert_eq!(tf.tasks.len(), 1);
        assert_eq!(tf.tasks[0].id, 2);
    }

    // -- Covers line 197: id? returns None when metadata has no id field --
    #[test]
    fn test_parse_metadata_comment_missing_id() {
        // A metadata comment without an id field: id stays None, so id? returns None,
        // making parse_metadata_comment return None -> task is skipped in tolerant mode
        let content = "<!-- format:1 -->\n<!-- next-id:2 -->\n\n## [ ] Task no id\n<!-- priority:medium created:2025-01-15T10:00:00+00:00 -->\n";
        let tf = parse(content, false).unwrap();
        // Task skipped because metadata has no id
        assert_eq!(tf.tasks.len(), 0);
    }

    // -- Covers extract_header_value edge cases (lines 136-137) --
    #[test]
    fn test_parse_format_line_with_content_after_arrow() {
        // A format line that has extra content after --> should not be parsed
        // (doesn't end_with "-->") so extract_header_value is not called
        // This also tests that the header parsing loop correctly handles such lines
        let content = "<!-- format:1 --> extra\n<!-- next-id:2 -->\n\n## [ ] Task\n<!-- id:1 priority:medium created:2025-01-15T10:00:00+00:00 -->\n";
        let tf = parse(content, false).unwrap();
        // format:1 line is not recognized (doesn't end with -->), format_version stays default
        assert_eq!(tf.tasks.len(), 1);
    }

    // -- Note metadata tests --

    #[test]
    fn test_parse_task_with_note_metadata() {
        let content = "<!-- format:2 -->\n<!-- next-id:2 -->\n\n## [ ] Task with note\n<!-- id:1 priority:medium created:2025-01-15T10:00:00+00:00 note:meeting-notes -->\n";
        let tf = parse(content, false).unwrap();
        assert_eq!(tf.tasks.len(), 1);
        assert_eq!(tf.tasks[0].note, Some("meeting-notes".to_string()));
    }

    #[test]
    fn test_parse_task_without_note_metadata() {
        let content = "<!-- format:2 -->\n<!-- next-id:2 -->\n\n## [ ] Task no note\n<!-- id:1 priority:medium created:2025-01-15T10:00:00+00:00 -->\n";
        let tf = parse(content, false).unwrap();
        assert_eq!(tf.tasks.len(), 1);
        assert!(tf.tasks[0].note.is_none());
    }

    #[test]
    fn test_note_metadata_round_trip() {
        use chrono::{TimeZone, Utc};
        use crate::task::{Priority, Status, Task};
        let mut tf = TaskFile::new();
        tf.tasks.push(Task {
            id: 1,
            title: "Linked task".to_string(),
            status: Status::Open,
            priority: Priority::Medium,
            tags: Vec::new(),
            created: Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
            updated: None,
            description: None,
            due_date: None,
            project: None,
            recurrence: None,
            note: Some("my-note".to_string()),
            agent: None,
        });
        let serialized = serialize(&tf);
        assert!(serialized.contains("note:my-note"));
        let parsed = parse(&serialized, false).unwrap();
        assert_eq!(parsed.tasks[0].note, Some("my-note".to_string()));
    }

    #[test]
    fn test_parse_agent_field() {
        let content = "<!-- format:2 -->\n<!-- next-id:2 -->\n\n## [ ] Task with agent\n<!-- id:1 priority:medium created:2025-01-15T10:00:00+00:00 agent:command-center -->\n";
        let tf = parse(content, false).unwrap();
        assert_eq!(tf.tasks[0].agent, Some("command-center".to_string()));
    }

    #[test]
    fn test_parse_human_agent() {
        let content = "<!-- format:2 -->\n<!-- next-id:2 -->\n\n## [ ] Human task\n<!-- id:1 priority:medium created:2025-01-15T10:00:00+00:00 agent:human -->\n";
        let tf = parse(content, false).unwrap();
        assert_eq!(tf.tasks[0].agent, Some("human".to_string()));
    }

    #[test]
    fn test_parse_missing_agent_defaults_none() {
        let content = "<!-- format:2 -->\n<!-- next-id:2 -->\n\n## [ ] Unassigned task\n<!-- id:1 priority:medium created:2025-01-15T10:00:00+00:00 -->\n";
        let tf = parse(content, false).unwrap();
        assert!(tf.tasks[0].agent.is_none());
    }

    #[test]
    fn test_agent_round_trip() {
        use chrono::{TimeZone, Utc};
        use crate::task::{Priority, Status, Task};
        let mut tf = TaskFile::new();
        tf.tasks.push(Task {
            id: 1,
            title: "AI task".to_string(),
            status: Status::Open,
            priority: Priority::High,
            tags: Vec::new(),
            created: Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
            updated: None,
            description: None,
            due_date: None,
            project: None,
            recurrence: None,
            note: None,
            agent: Some("command-center".to_string()),
        });
        let serialized = serialize(&tf);
        assert!(serialized.contains("agent:command-center"));
        let parsed = parse(&serialized, false).unwrap();
        assert_eq!(parsed.tasks[0].agent, Some("command-center".to_string()));
    }

    #[test]
    fn test_agent_none_not_serialized() {
        use chrono::{TimeZone, Utc};
        use crate::task::{Priority, Status, Task};
        let mut tf = TaskFile::new();
        tf.tasks.push(Task {
            id: 1,
            title: "No agent".to_string(),
            status: Status::Open,
            priority: Priority::Medium,
            tags: Vec::new(),
            created: Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap(),
            updated: None,
            description: None,
            due_date: None,
            project: None,
            recurrence: None,
            note: None,
            agent: None,
        });
        let serialized = serialize(&tf);
        assert!(!serialized.contains("agent:"));
    }

    // -- parse_due_date_input tests --
    // today = 2026-03-31 (Tuesday) used throughout

    fn tue() -> NaiveDate { NaiveDate::from_ymd_opt(2026, 3, 31).unwrap() }

    #[test]
    fn test_parse_due_iso_passthrough() {
        let d = parse_due_date_input("2026-04-15", tue()).unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2026, 4, 15).unwrap());
    }

    #[test]
    fn test_parse_due_full_weekday_monday() {
        // Today is Tuesday; next Monday is 2026-04-06 (6 days away)
        let d = parse_due_date_input("Monday", tue()).unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2026, 4, 6).unwrap());
    }

    #[test]
    fn test_parse_due_abbrev_fri() {
        // Today is Tuesday; next Friday is 2026-04-03 (3 days away)
        let d = parse_due_date_input("fri", tue()).unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2026, 4, 3).unwrap());
    }

    #[test]
    fn test_parse_due_case_insensitive() {
        let lower = parse_due_date_input("wednesday", tue()).unwrap();
        let upper = parse_due_date_input("WEDNESDAY", tue()).unwrap();
        let mixed = parse_due_date_input("Wednesday", tue()).unwrap();
        assert_eq!(lower, upper);
        assert_eq!(lower, mixed);
    }

    #[test]
    fn test_parse_due_same_weekday_advances_one_week() {
        // Today is Tuesday; "tuesday" should give next Tuesday (7 days)
        let d = parse_due_date_input("tuesday", tue()).unwrap();
        assert_eq!(d, NaiveDate::from_ymd_opt(2026, 4, 7).unwrap());
    }

    #[test]
    fn test_parse_due_unrecognized_returns_none() {
        assert!(parse_due_date_input("soon", tue()).is_none());
        assert!(parse_due_date_input("asap", tue()).is_none());
        assert!(parse_due_date_input("2026/04/01", tue()).is_none());
    }

    #[test]
    fn test_parse_due_empty_returns_none() {
        assert!(parse_due_date_input("", tue()).is_none());
    }
}
