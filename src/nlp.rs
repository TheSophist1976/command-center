use serde::{Deserialize, Serialize};

use crate::task::Task;

// -- Action types --

#[derive(Debug, Clone, PartialEq)]
pub enum NlpAction {
    Filter(FilterCriteria),
    Update {
        match_criteria: FilterCriteria,
        set_fields: SetFields,
        description: String,
    },
    Message(String),
    ShowTasks {
        task_ids: Vec<u32>,
        text: String,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct FilterCriteria {
    pub project: Option<String>,
    pub status: Option<String>,
    pub priority: Option<String>,
    pub tag: Option<String>,
    pub title_contains: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct SetFields {
    pub priority: Option<String>,
    pub status: Option<String>,
    pub tags: Option<Vec<String>>,
}

// -- Task context for prompt --

#[derive(Serialize)]
struct TaskSummary {
    id: u32,
    title: String,
    status: String,
    priority: String,
    tags: Vec<String>,
    due_date: Option<String>,
    project: Option<String>,
}

pub fn build_task_context(tasks: &[Task]) -> String {
    let limit = tasks.len().min(200);
    let summaries: Vec<TaskSummary> = tasks[..limit]
        .iter()
        .map(|t| TaskSummary {
            id: t.id,
            title: t.title.clone(),
            status: t.status.to_string(),
            priority: t.priority.to_string(),
            tags: t.tags.clone(),
            due_date: t.due_date.map(|d| d.format("%Y-%m-%d").to_string()),
            project: t.project.clone(),
        })
        .collect();

    let mut json = serde_json::to_string(&summaries).unwrap_or_else(|_| "[]".to_string());
    if tasks.len() > 200 {
        json.push_str(&format!("\n(Showing 200 of {} total tasks)", tasks.len()));
    }
    json
}

// -- Prompt construction --

fn build_system_prompt(task_context: &str) -> String {
    format!(
        r#"You are a task manager assistant. The user will give you a natural language command about their tasks. You must respond with ONLY a valid JSON object (no markdown, no explanation).

The user's current tasks:
{}

Respond with exactly one of these JSON formats:

1. To filter/search tasks:
{{"action":"filter","criteria":{{"project":null,"status":null,"priority":null,"tag":null,"title_contains":null}}}}
Fill in non-null values for the fields the user wants to filter by. Values: status is "open" or "done", priority is "critical"/"high"/"medium"/"low", project and tag are strings, title_contains is a substring to match.

2. To bulk-update tasks:
{{"action":"update","match":{{"project":null,"status":null,"priority":null,"tag":null,"title_contains":null}},"set":{{"priority":null,"status":null,"tags":null}},"description":"Human-readable description of the change"}}
Fill in "match" with criteria to find tasks, and "set" with fields to change. Only include non-null fields in "set".

3. To respond with a message (for questions, unclear queries, or unsupported actions):
{{"action":"message","text":"Your response text here"}}
Use this when the query is ambiguous, conversational, asks a question about the tasks, or requests something you cannot do via filter/update. You can answer questions about the user's tasks using the task data above — including counts, summaries, and queries across all fields (id, title, status, priority, tags, due_date, project).

4. To show specific tasks inline in the conversation:
{{"action":"show_tasks","task_ids":[1,3,7],"text":"Here are your high-priority tasks:"}}
Use this when the user wants to see a specific group of tasks displayed. The task_ids array contains the IDs of tasks to display. Use this instead of filter when the user wants to see tasks as part of the conversation without changing the main table view.

Rules:
- Respond with ONLY the JSON object, nothing else
- Use null for fields that are not relevant
- Match project and tag names case-insensitively against the task data
- For clear filter or update requests, prefer filter/update over message
- For questions, ambiguous input, or unsupported requests, use message
- When the user asks to "show" or "list" specific tasks, prefer show_tasks over filter
- The description field in update should say what will happen (e.g., "Set priority to high on 5 frontend tasks")
- This is a multi-turn conversation. Use context from prior messages to understand follow-up queries (e.g., "mark those as high priority" refers to previously discussed tasks)"#,
        task_context
    )
}

// -- Claude API --

#[derive(Serialize)]
struct ApiRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<ApiMessage>,
}

#[derive(Serialize, Clone, Debug)]
pub struct ApiMessage {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize)]
struct ApiResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: Option<String>,
}

#[derive(Deserialize)]
struct ApiError {
    error: Option<ApiErrorDetail>,
}

#[derive(Deserialize)]
struct ApiErrorDetail {
    message: Option<String>,
}

fn api_base_url() -> String {
    std::env::var("CLAUDE_API_BASE_URL")
        .unwrap_or_else(|_| "https://api.anthropic.com".to_string())
}

pub fn call_claude_api(api_key: &str, system_prompt: &str, messages: &[ApiMessage]) -> Result<String, String> {
    let client = reqwest::blocking::Client::new();
    let base = api_base_url();

    let request = ApiRequest {
        model: "claude-haiku-4-5-20251001".to_string(),
        max_tokens: 1024,
        system: system_prompt.to_string(),
        messages: messages.to_vec(),
    };

    let response = client
        .post(format!("{}/v1/messages", base))
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .map_err(|e| format!("Claude API request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        if let Ok(err) = serde_json::from_str::<ApiError>(&body) {
            if let Some(detail) = err.error {
                if let Some(msg) = detail.message {
                    return Err(format!("Claude API error ({}): {}", status, msg));
                }
            }
        }
        return Err(format!("Claude API error: {}", status));
    }

    let api_response: ApiResponse = response
        .json()
        .map_err(|e| format!("Failed to parse Claude API response: {}", e))?;

    api_response
        .content
        .into_iter()
        .find_map(|block| block.text)
        .ok_or_else(|| "Claude returned empty response".to_string())
}

// -- Response parsing --

#[derive(Deserialize)]
struct RawAction {
    action: String,
    criteria: Option<FilterCriteria>,
    #[serde(rename = "match")]
    match_criteria: Option<FilterCriteria>,
    set: Option<SetFields>,
    description: Option<String>,
    text: Option<String>,
    task_ids: Option<Vec<u32>>,
}

pub fn parse_response(json_str: &str) -> Result<NlpAction, String> {
    // Strip markdown code fences if present
    let cleaned = json_str
        .trim()
        .strip_prefix("```json")
        .or_else(|| json_str.trim().strip_prefix("```"))
        .unwrap_or(json_str.trim());
    let cleaned = cleaned
        .strip_suffix("```")
        .unwrap_or(cleaned)
        .trim();

    let raw: RawAction = serde_json::from_str(cleaned)
        .map_err(|e| format!("Could not understand the response: {}", e))?;

    match raw.action.as_str() {
        "filter" => {
            let criteria = raw.criteria.unwrap_or_default();
            Ok(NlpAction::Filter(criteria))
        }
        "update" => {
            let match_criteria = raw.match_criteria.unwrap_or_default();
            let set_fields = raw.set.unwrap_or_default();
            let description = raw.description.unwrap_or_else(|| "Bulk update".to_string());
            Ok(NlpAction::Update {
                match_criteria,
                set_fields,
                description,
            })
        }
        "message" => {
            let text = raw.text.unwrap_or_else(|| "No response from model".to_string());
            Ok(NlpAction::Message(text))
        }
        "show_tasks" => {
            let task_ids = raw.task_ids.unwrap_or_default();
            let text = raw.text.unwrap_or_else(|| "Here are the matching tasks:".to_string());
            Ok(NlpAction::ShowTasks { task_ids, text })
        }
        other => Err(format!("Unknown action type: {}", other)),
    }
}

// -- Public API --

fn log_debug(msg: &str) {
    if let Ok(path) = std::env::var("TASK_NLP_LOG") {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let _ = writeln!(f, "[{}] {}", chrono::Local::now().format("%H:%M:%S"), msg);
        }
    }
}

/// Interpret a conversation with the NLP model. Returns the parsed action and raw response text.
/// The raw response text is returned so the caller can append it to the conversation history.
pub fn interpret(tasks: &[Task], messages: &[ApiMessage], api_key: &str) -> Result<(NlpAction, String), String> {
    let task_context = build_task_context(tasks);
    let system_prompt = build_system_prompt(&task_context);

    log_debug(&format!("--- NLP Request ---"));
    log_debug(&format!("Messages: {} total", messages.len()));
    log_debug(&format!("Task count: {}", tasks.len()));
    log_debug(&format!("System prompt:\n{}", system_prompt));

    let response_text = call_claude_api(api_key, &system_prompt, messages)?;

    log_debug(&format!("Claude response: {}", response_text));

    let action = parse_response(&response_text)?;

    log_debug(&format!("Parsed action: {:?}", action));
    log_debug(&format!("--- End ---\n"));

    Ok((action, response_text))
}

// -- Tests --

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Priority, Status, Task};
    use chrono::Utc;

    fn make_task(id: u32, title: &str) -> Task {
        Task {
            id,
            title: title.to_string(),
            status: Status::Open,
            priority: Priority::Medium,
            tags: vec!["test".to_string()],
            created: Utc::now(),
            updated: None,
            description: None,
            due_date: None,
            project: Some("TestProject".to_string()),
        }
    }

    #[test]
    fn build_task_context_includes_tasks() {
        let tasks = vec![make_task(1, "Hello"), make_task(2, "World")];
        let ctx = build_task_context(&tasks);
        assert!(ctx.contains("Hello"));
        assert!(ctx.contains("World"));
        assert!(ctx.contains("TestProject"));
    }

    #[test]
    fn build_task_context_truncates_at_200() {
        let tasks: Vec<Task> = (1..=250).map(|i| make_task(i, &format!("Task {}", i))).collect();
        let ctx = build_task_context(&tasks);
        assert!(ctx.contains("Task 200"));
        assert!(!ctx.contains("\"Task 201\""));
        assert!(ctx.contains("Showing 200 of 250 total tasks"));
    }

    #[test]
    fn parse_response_valid_filter() {
        let json = r#"{"action":"filter","criteria":{"project":"FLOW AI","status":"open","priority":null,"tag":null,"title_contains":null}}"#;
        let result = parse_response(json).unwrap();
        match result {
            NlpAction::Filter(c) => {
                assert_eq!(c.project, Some("FLOW AI".to_string()));
                assert_eq!(c.status, Some("open".to_string()));
                assert!(c.priority.is_none());
            }
            _ => panic!("Expected Filter"),
        }
    }

    #[test]
    fn parse_response_valid_update() {
        let json = r#"{"action":"update","match":{"tag":"frontend","status":null,"priority":null,"project":null,"title_contains":null},"set":{"priority":"high","status":null,"tags":null},"description":"Set priority high on frontend tasks"}"#;
        let result = parse_response(json).unwrap();
        match result {
            NlpAction::Update { match_criteria, set_fields, description } => {
                assert_eq!(match_criteria.tag, Some("frontend".to_string()));
                assert_eq!(set_fields.priority, Some("high".to_string()));
                assert_eq!(description, "Set priority high on frontend tasks");
            }
            _ => panic!("Expected Update"),
        }
    }

    #[test]
    fn parse_response_strips_markdown_fences() {
        let json = "```json\n{\"action\":\"filter\",\"criteria\":{\"project\":\"Work\",\"status\":null,\"priority\":null,\"tag\":null,\"title_contains\":null}}\n```";
        let result = parse_response(json).unwrap();
        match result {
            NlpAction::Filter(c) => assert_eq!(c.project, Some("Work".to_string())),
            _ => panic!("Expected Filter"),
        }
    }

    #[test]
    fn parse_response_invalid_json() {
        let result = parse_response("not json at all");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Could not understand"));
    }

    #[test]
    fn parse_response_unknown_action() {
        let json = r#"{"action":"delete","criteria":{}}"#;
        let result = parse_response(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown action type"));
    }

    #[test]
    fn parse_response_valid_message() {
        let json = r#"{"action":"message","text":"You have 5 high-priority tasks."}"#;
        let result = parse_response(json).unwrap();
        match result {
            NlpAction::Message(text) => {
                assert_eq!(text, "You have 5 high-priority tasks.");
            }
            _ => panic!("Expected Message"),
        }
    }

    #[test]
    fn parse_response_message_with_markdown_fences() {
        let json = "```json\n{\"action\":\"message\",\"text\":\"I can't do that.\"}\n```";
        let result = parse_response(json).unwrap();
        match result {
            NlpAction::Message(text) => assert_eq!(text, "I can't do that."),
            _ => panic!("Expected Message"),
        }
    }

    #[test]
    fn parse_response_valid_show_tasks() {
        let json = r#"{"action":"show_tasks","task_ids":[1,3,7],"text":"Here are your high-priority tasks:"}"#;
        let result = parse_response(json).unwrap();
        match result {
            NlpAction::ShowTasks { task_ids, text } => {
                assert_eq!(task_ids, vec![1, 3, 7]);
                assert_eq!(text, "Here are your high-priority tasks:");
            }
            _ => panic!("Expected ShowTasks"),
        }
    }

    #[test]
    fn parse_response_show_tasks_empty_ids() {
        let json = r#"{"action":"show_tasks","task_ids":[],"text":"No tasks match that criteria."}"#;
        let result = parse_response(json).unwrap();
        match result {
            NlpAction::ShowTasks { task_ids, text } => {
                assert!(task_ids.is_empty());
                assert_eq!(text, "No tasks match that criteria.");
            }
            _ => panic!("Expected ShowTasks"),
        }
    }
}
