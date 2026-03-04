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
        task_ids: Option<Vec<u32>>,
    },
    Message(String),
    ShowTasks {
        task_ids: Vec<u32>,
        text: String,
    },
    SetRecurrence {
        task_id: u32,
        recurrence: Option<String>,
        description: String,
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
    pub due_date: Option<String>,
}

// -- Task context for prompt --

#[derive(Serialize, Deserialize)]
struct TaskSummary {
    id: u32,
    title: String,
    status: String,
    priority: String,
    tags: Vec<String>,
    due_date: Option<String>,
    project: Option<String>,
    recurrence: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
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
            recurrence: t.recurrence.map(|r| r.to_string()),
            description: t.description.clone(),
        })
        .collect();

    let mut json = serde_json::to_string(&summaries).unwrap_or_else(|_| "[]".to_string());
    if tasks.len() > 200 {
        json.push_str(&format!("\n(Showing 200 of {} total tasks)", tasks.len()));
    }
    json
}

// -- Prompt construction --

fn build_system_prompt(task_context: &str, today: &str) -> String {
    format!(
        r#"You are a task manager assistant. The user will give you a natural language command about their tasks. You must respond with ONLY a valid JSON object (no markdown, no explanation).

Today's date is {}.

The user's current tasks:
{}

Respond with exactly one of these JSON formats:

1. To filter/search tasks:
{{"action":"filter","criteria":{{"project":null,"status":null,"priority":null,"tag":null,"title_contains":null}}}}
Fill in non-null values for the fields the user wants to filter by. Values: status is "open" or "done", priority is "critical"/"high"/"medium"/"low", project and tag are strings, title_contains is a substring to match.

2. To bulk-update tasks:
{{"action":"update","match":{{"project":null,"status":null,"priority":null,"tag":null,"title_contains":null}},"task_ids":null,"set":{{"priority":null,"status":null,"tags":null,"due_date":null}},"description":"Human-readable description of the change"}}
Fill in "match" with criteria to find tasks, and "set" with fields to change. Only include non-null fields in "set". You can use EITHER "match" criteria OR "task_ids" (an array of task ID numbers) to select tasks. Use "task_ids" when you know the exact task IDs to update. The due_date field should be in YYYY-MM-DD format — resolve relative dates like "today", "tomorrow", "next monday" to absolute dates using the provided current date. Set due_date to "" to clear a task's due date.

3. To respond with a message (for questions, unclear queries, or unsupported actions):
{{"action":"message","text":"Your response text here"}}
Use this when the query is ambiguous, conversational, asks a question about the tasks, or requests something you cannot do via filter/update. You can answer questions about the user's tasks using the task data above — including counts, summaries, and queries across all fields (id, title, status, priority, tags, due_date, project, description).

4. To show specific tasks inline in the conversation:
{{"action":"show_tasks","task_ids":[1,3,7],"text":"Here are your high-priority tasks:"}}
Use this when the user wants to see a specific group of tasks displayed. The task_ids array contains the IDs of tasks to display. Use this instead of filter when the user wants to see tasks as part of the conversation without changing the main table view.

5. To set or remove recurrence on a task:
{{"action":"set_recurrence","task_id":5,"recurrence":"weekly","description":"Set task 5 to repeat weekly"}}
Valid recurrence values: "daily", "weekly", "monthly", "yearly", "monthly:N:DAY" (e.g., "monthly:3:thu" for 3rd Thursday). Set "recurrence" to null to remove recurrence. The task_id must reference an existing task.

Rules:
- Respond with ONLY the JSON object, nothing else
- Use null for fields that are not relevant
- Match project and tag names case-insensitively against the task data
- For clear filter or update requests, prefer filter/update over message
- For questions, ambiguous input, or unsupported requests, use message
- When the user asks to "show" or "list" specific tasks, prefer show_tasks over filter
- The description field in update should say what will happen (e.g., "Set priority to high on 5 frontend tasks")
- This is a multi-turn conversation. Use context from prior messages to understand follow-up queries (e.g., "mark those as high priority" refers to previously discussed tasks)
- Use the provided current date to interpret relative time references such as "today", "this week", "overdue", "tomorrow", etc.
- You have access to a `fetch_url` tool that can read web pages. Use it when the user mentions a URL, asks to summarize a link, or needs information from a web page referenced in a task. After fetching, incorporate the content into your response using the message action.
- You have access to a `query_tasks` tool that can search and filter tasks. Use it for date-based queries (overdue tasks, tasks due this week, due before/after a date) as it performs accurate date comparison. Also useful for complex filtering. This is more reliable than scanning the task list manually. After querying, use the results to construct your response (show_tasks, update, filter, or message)."#,
        today, task_context
    )
}

// -- Claude API --

#[derive(Serialize)]
struct ApiRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<ApiMessageRaw>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ToolDef>>,
}

/// Public message type used by the TUI for conversation history.
/// Content is always a plain string (user text or assistant JSON response).
#[derive(Serialize, Clone, Debug)]
pub struct ApiMessage {
    pub role: String,
    pub content: String,
}

/// Internal message type for the API that supports structured content blocks.
#[derive(Serialize, Clone, Debug)]
struct ApiMessageRaw {
    role: String,
    content: serde_json::Value,
}

impl From<&ApiMessage> for ApiMessageRaw {
    fn from(msg: &ApiMessage) -> Self {
        ApiMessageRaw {
            role: msg.role.clone(),
            content: serde_json::Value::String(msg.content.clone()),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
struct ToolDef {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

#[derive(Deserialize, Debug)]
struct ApiResponse {
    content: Vec<ContentBlock>,
    #[allow(dead_code)]
    stop_reason: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: Option<String>,
    text: Option<String>,
    id: Option<String>,
    name: Option<String>,
    input: Option<serde_json::Value>,
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

fn fetch_url_tool_def() -> ToolDef {
    ToolDef {
        name: "fetch_url".to_string(),
        description: "Fetch a web page and return its text content. Use when the user asks about a URL or wants content summarized.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch"
                }
            },
            "required": ["url"]
        }),
    }
}

fn query_tasks_tool_def() -> ToolDef {
    ToolDef {
        name: "query_tasks".to_string(),
        description: "Search and filter the user's tasks. Use this for date-based queries (overdue, due this week), or when you need to find tasks matching specific criteria. More reliable than scanning the task list manually.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "status": {
                    "type": "string",
                    "description": "Filter by status: \"open\" or \"done\"",
                    "enum": ["open", "done"]
                },
                "priority": {
                    "type": "string",
                    "description": "Filter by priority: \"critical\", \"high\", \"medium\", or \"low\"",
                    "enum": ["critical", "high", "medium", "low"]
                },
                "project": {
                    "type": "string",
                    "description": "Filter by project name (case-insensitive)"
                },
                "tag": {
                    "type": "string",
                    "description": "Filter by tag (case-insensitive)"
                },
                "title_contains": {
                    "type": "string",
                    "description": "Filter by title substring (case-insensitive)"
                },
                "overdue": {
                    "type": "boolean",
                    "description": "If true, return only open tasks with due_date before today"
                },
                "has_due_date": {
                    "type": "boolean",
                    "description": "If true, return tasks with a due date; if false, tasks without"
                },
                "has_recurrence": {
                    "type": "boolean",
                    "description": "If true, return recurring tasks; if false, non-recurring tasks"
                }
            }
        }),
    }
}

/// Fetch a URL and extract readable text from the HTML.
/// Strips script/style tags, then removes remaining HTML tags.
/// Truncates to 4000 characters.
pub fn fetch_url(url: &str) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("task-manager/0.1 (URL summarizer)")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(url)
        .send()
        .map_err(|e| {
            if e.is_timeout() {
                "Request timed out after 10 seconds".to_string()
            } else {
                format!("Failed to fetch URL: {}", e)
            }
        })?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let body = response
        .text()
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    Ok(html_to_text(&body))
}

/// Extract readable text from HTML by stripping tags.
fn html_to_text(html: &str) -> String {
    // Remove script and style blocks
    let mut result = html.to_string();
    for tag in &["script", "style", "noscript"] {
        loop {
            let open = format!("<{}", tag);
            let close = format!("</{}>", tag);
            if let Some(start) = result.to_lowercase().find(&open) {
                if let Some(end) = result.to_lowercase()[start..].find(&close) {
                    result = format!("{}{}", &result[..start], &result[start + end + close.len()..]);
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    // Remove all HTML tags
    let mut text = String::with_capacity(result.len());
    let mut in_tag = false;
    for ch in result.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => text.push(ch),
            _ => {}
        }
    }

    // Collapse whitespace and trim
    let text: String = text
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    // Truncate to 4000 characters
    if text.len() > 4000 {
        format!("{}...\n\n[Content truncated at 4000 characters]", &text[..4000])
    } else {
        text
    }
}

/// Execute a query_tasks tool call: filter tasks by criteria and return JSON.
fn execute_query_tasks(input: &serde_json::Value, tasks: &[Task]) -> String {
    let today = chrono::Local::now().date_naive();

    let status = input.get("status").and_then(|v| v.as_str());
    let priority = input.get("priority").and_then(|v| v.as_str());
    let project = input.get("project").and_then(|v| v.as_str());
    let tag = input.get("tag").and_then(|v| v.as_str());
    let title_contains = input.get("title_contains").and_then(|v| v.as_str());
    let overdue = input.get("overdue").and_then(|v| v.as_bool());
    let has_due_date = input.get("has_due_date").and_then(|v| v.as_bool());
    let has_recurrence = input.get("has_recurrence").and_then(|v| v.as_bool());

    let matching: Vec<&Task> = tasks
        .iter()
        .filter(|t| {
            if let Some(s) = status {
                if !t.status.to_string().eq_ignore_ascii_case(s) { return false; }
            }
            if let Some(p) = priority {
                if !t.priority.to_string().eq_ignore_ascii_case(p) { return false; }
            }
            if let Some(proj) = project {
                match &t.project {
                    Some(p) => if !p.eq_ignore_ascii_case(proj) { return false; },
                    None => return false,
                }
            }
            if let Some(tg) = tag {
                if !t.tags.iter().any(|x| x.eq_ignore_ascii_case(tg)) { return false; }
            }
            if let Some(tc) = title_contains {
                if !t.title.to_lowercase().contains(&tc.to_lowercase()) { return false; }
            }
            if overdue == Some(true) {
                let is_overdue = t.status.to_string().eq_ignore_ascii_case("open")
                    && t.due_date.is_some_and(|d| d < today);
                if !is_overdue { return false; }
            }
            if let Some(hdd) = has_due_date {
                if hdd && t.due_date.is_none() { return false; }
                if !hdd && t.due_date.is_some() { return false; }
            }
            if let Some(hr) = has_recurrence {
                if hr && t.recurrence.is_none() { return false; }
                if !hr && t.recurrence.is_some() { return false; }
            }
            true
        })
        .collect();

    let total = matching.len();
    let cap = 50;
    let summaries: Vec<TaskSummary> = matching
        .into_iter()
        .take(cap)
        .map(|t| TaskSummary {
            id: t.id,
            title: t.title.clone(),
            status: t.status.to_string(),
            priority: t.priority.to_string(),
            tags: t.tags.clone(),
            due_date: t.due_date.map(|d| d.format("%Y-%m-%d").to_string()),
            project: t.project.clone(),
            recurrence: t.recurrence.map(|r| r.to_string()),
            description: t.description.clone(),
        })
        .collect();

    let mut json = serde_json::to_string(&summaries).unwrap_or_else(|_| "[]".to_string());
    if total > cap {
        json.push_str(&format!("\n(Showing {} of {} matching tasks)", cap, total));
    }
    json
}

/// Execute a tool_use request from the model.
fn execute_tool(name: &str, input: &serde_json::Value, tasks: &[Task]) -> String {
    match name {
        "fetch_url" => {
            let url = input.get("url").and_then(|v| v.as_str()).unwrap_or("");
            if url.is_empty() {
                return "Error: No URL provided".to_string();
            }
            match fetch_url(url) {
                Ok(text) => text,
                Err(e) => format!("Error fetching URL: {}", e),
            }
        }
        "query_tasks" => execute_query_tasks(input, tasks),
        other => format!("Unknown tool: {}", other),
    }
}

pub fn call_claude_api(api_key: &str, tasks: &[Task], system_prompt: &str, messages: &[ApiMessage]) -> Result<String, String> {
    let client = reqwest::blocking::Client::new();
    let base = api_base_url();

    let mut conversation: Vec<ApiMessageRaw> = messages.iter().map(ApiMessageRaw::from).collect();
    let tools = vec![fetch_url_tool_def(), query_tasks_tool_def()];

    for _iteration in 0..3 {
        let request = ApiRequest {
            model: "claude-haiku-4-5-20251001".to_string(),
            max_tokens: 4096,
            system: system_prompt.to_string(),
            messages: conversation.clone(),
            tools: Some(tools.clone()),
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

        // Check if the model wants to use tools
        let tool_uses: Vec<&ContentBlock> = api_response
            .content
            .iter()
            .filter(|b| b.block_type.as_deref() == Some("tool_use"))
            .collect();

        if tool_uses.is_empty() {
            // No tool use — return the text response
            return api_response
                .content
                .into_iter()
                .find_map(|block| block.text)
                .ok_or_else(|| "Claude returned empty response".to_string());
        }

        // Build the assistant message with all content blocks
        let assistant_content: Vec<serde_json::Value> = api_response
            .content
            .iter()
            .map(|block| {
                if block.block_type.as_deref() == Some("tool_use") {
                    serde_json::json!({
                        "type": "tool_use",
                        "id": block.id,
                        "name": block.name,
                        "input": block.input
                    })
                } else {
                    serde_json::json!({
                        "type": "text",
                        "text": block.text.as_deref().unwrap_or("")
                    })
                }
            })
            .collect();

        conversation.push(ApiMessageRaw {
            role: "assistant".to_string(),
            content: serde_json::Value::Array(assistant_content),
        });

        // Execute each tool and build tool_result blocks
        let tool_results: Vec<serde_json::Value> = tool_uses
            .iter()
            .map(|block| {
                let name = block.name.as_deref().unwrap_or("");
                let input = block.input.as_ref().cloned().unwrap_or(serde_json::json!({}));
                let result = execute_tool(name, &input, tasks);
                log_debug(&format!("Tool {} result: {} chars", name, result.len()));
                serde_json::json!({
                    "type": "tool_result",
                    "tool_use_id": block.id,
                    "content": result
                })
            })
            .collect();

        conversation.push(ApiMessageRaw {
            role: "user".to_string(),
            content: serde_json::Value::Array(tool_results),
        });
    }

    Err("Tool-use loop exceeded maximum iterations".to_string())
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
    task_id: Option<u32>,
    recurrence: Option<serde_json::Value>,
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

    let raw: RawAction = match serde_json::from_str(cleaned) {
        Ok(r) => r,
        Err(_) => {
            // If the response isn't valid JSON, treat it as a plain text message.
            // This happens when the model responds naturally after tool use
            // instead of wrapping its answer in the JSON action format.
            return Ok(NlpAction::Message(cleaned.to_string()));
        }
    };

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
                task_ids: raw.task_ids,
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
        "set_recurrence" => {
            let task_id = raw.task_id.ok_or("set_recurrence requires task_id")?;
            let recurrence = match raw.recurrence {
                Some(serde_json::Value::String(s)) => Some(s),
                Some(serde_json::Value::Null) | None => None,
                _ => None,
            };
            let description = raw.description.unwrap_or_else(|| "Set recurrence".to_string());
            Ok(NlpAction::SetRecurrence { task_id, recurrence, description })
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
    let today = chrono::Local::now().format("%Y-%m-%d (%A)").to_string();
    let system_prompt = build_system_prompt(&task_context, &today);

    log_debug(&format!("--- NLP Request ---"));
    log_debug(&format!("Messages: {} total", messages.len()));
    log_debug(&format!("Task count: {}", tasks.len()));
    log_debug(&format!("System prompt:\n{}", system_prompt));

    let response_text = call_claude_api(api_key, tasks, &system_prompt, messages)?;

    log_debug(&format!("Claude response: {}", response_text));

    let action = parse_response(&response_text)?;

    log_debug(&format!("Parsed action: {:?}", action));
    log_debug(&format!("--- End ---\n"));

    Ok((action, response_text))
}

/// Parse a natural language recurrence pattern into the internal format.
/// Uses a focused NLP prompt that only parses recurrence, not general commands.
/// Returns the recurrence string (e.g., "weekly", "monthly:3:thu") or None for "none"/"remove".
pub fn parse_recurrence_nlp(input: &str, api_key: &str) -> Result<Option<String>, String> {
    let system_prompt = r#"You are a recurrence pattern parser. The user will describe how often a task should repeat. Respond with ONLY a valid JSON object (no markdown, no explanation).

Respond with exactly one of:
{"recurrence":"daily"} - repeats every day
{"recurrence":"weekly"} - repeats every week
{"recurrence":"monthly"} - repeats every month
{"recurrence":"yearly"} - repeats every year
{"recurrence":"monthly:N:DAY"} - repeats on the Nth weekday of each month (e.g., "monthly:3:thu" for 3rd Thursday). DAY is mon/tue/wed/thu/fri/sat/sun. N is 1-5.
{"recurrence":null} - remove recurrence (when user says "none", "stop", "remove", "clear", etc.)

Rules:
- "every day" / "daily" → "daily"
- "every week" / "weekly" → "weekly"
- "every month" / "monthly" → "monthly"
- "every year" / "yearly" / "annually" → "yearly"
- "every third thursday" / "3rd thursday of every month" → "monthly:3:thu"
- "every first monday" / "1st monday" → "monthly:1:mon"
- "none" / "clear" / "remove" / "stop repeating" → null
- Respond with ONLY the JSON, nothing else"#;

    let messages = vec![ApiMessage {
        role: "user".to_string(),
        content: input.to_string(),
    }];

    let response = call_claude_api(api_key, &[], system_prompt, &messages)?;

    // Parse the response
    let cleaned = response
        .trim()
        .strip_prefix("```json")
        .or_else(|| response.trim().strip_prefix("```"))
        .unwrap_or(response.trim());
    let cleaned = cleaned.strip_suffix("```").unwrap_or(cleaned).trim();

    #[derive(Deserialize)]
    struct RecurResponse {
        recurrence: Option<String>,
    }

    let parsed: RecurResponse = serde_json::from_str(cleaned)
        .map_err(|e| format!("Could not parse recurrence response: {}", e))?;

    Ok(parsed.recurrence)
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
            recurrence: None,
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
            NlpAction::Update { match_criteria, set_fields, description, .. } => {
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
    fn parse_response_plain_text_becomes_message() {
        let result = parse_response("not json at all").unwrap();
        match result {
            NlpAction::Message(text) => assert_eq!(text, "not json at all"),
            _ => panic!("Expected Message for plain text response"),
        }
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

    #[test]
    fn system_prompt_includes_today_date() {
        let tasks = vec![make_task(1, "Test task")];
        let ctx = build_task_context(&tasks);
        let prompt = build_system_prompt(&ctx, "2026-03-02 (Monday)");
        assert!(prompt.contains("Today's date is 2026-03-02 (Monday)."));
    }

    #[test]
    fn system_prompt_mentions_fetch_url() {
        let tasks = vec![make_task(1, "Test task")];
        let ctx = build_task_context(&tasks);
        let prompt = build_system_prompt(&ctx, "2026-03-02 (Monday)");
        assert!(prompt.contains("fetch_url"));
    }

    #[test]
    fn html_to_text_strips_tags() {
        let html = "<html><body><h1>Title</h1><p>Hello world</p></body></html>";
        let text = html_to_text(html);
        assert!(text.contains("Title"));
        assert!(text.contains("Hello world"));
        assert!(!text.contains("<h1>"));
        assert!(!text.contains("<p>"));
    }

    #[test]
    fn html_to_text_strips_script_and_style() {
        let html = "<html><head><style>body { color: red; }</style></head><body><script>alert('hi');</script><p>Content</p></body></html>";
        let text = html_to_text(html);
        assert!(text.contains("Content"));
        assert!(!text.contains("alert"));
        assert!(!text.contains("color: red"));
    }

    #[test]
    fn html_to_text_truncates_long_content() {
        let html = format!("<p>{}</p>", "a".repeat(5000));
        let text = html_to_text(&html);
        assert!(text.len() < 5000);
        assert!(text.contains("[Content truncated at 4000 characters]"));
    }

    #[test]
    fn html_to_text_short_content_not_truncated() {
        let html = "<p>Short content</p>";
        let text = html_to_text(html);
        assert_eq!(text, "Short content");
        assert!(!text.contains("truncated"));
    }

    #[test]
    fn execute_tool_fetch_url_empty_url() {
        let result = execute_tool("fetch_url", &serde_json::json!({}), &[]);
        assert!(result.contains("No URL provided"));
    }

    #[test]
    fn execute_tool_unknown_tool() {
        let result = execute_tool("unknown_tool", &serde_json::json!({}), &[]);
        assert!(result.contains("Unknown tool"));
    }

    #[test]
    fn fetch_url_tool_def_has_correct_name() {
        let def = fetch_url_tool_def();
        assert_eq!(def.name, "fetch_url");
        assert!(def.description.contains("web page"));
        assert!(def.input_schema["properties"]["url"].is_object());
    }

    #[test]
    fn parse_response_update_with_due_date() {
        let json = r#"{"action":"update","match":{"project":null,"status":"open","priority":null,"tag":null,"title_contains":null},"set":{"priority":null,"status":null,"tags":null,"due_date":"2026-03-10"},"description":"Set due date to March 10"}"#;
        let result = parse_response(json).unwrap();
        match result {
            NlpAction::Update { set_fields, .. } => {
                assert_eq!(set_fields.due_date, Some("2026-03-10".to_string()));
            }
            _ => panic!("Expected Update action"),
        }
    }

    #[test]
    fn parse_response_update_without_due_date() {
        let json = r#"{"action":"update","match":{"project":null,"status":"open","priority":null,"tag":null,"title_contains":null},"set":{"priority":"high","status":null,"tags":null},"description":"Set priority"}"#;
        let result = parse_response(json).unwrap();
        match result {
            NlpAction::Update { set_fields, .. } => {
                assert!(set_fields.due_date.is_none());
                assert_eq!(set_fields.priority, Some("high".to_string()));
            }
            _ => panic!("Expected Update action"),
        }
    }

    #[test]
    fn system_prompt_includes_due_date_in_update_format() {
        let ctx = build_task_context(&[make_task(1, "Test")]);
        let prompt = build_system_prompt(&ctx, "2026-03-04 (Wednesday)");
        assert!(prompt.contains("\"due_date\":null"));
        assert!(prompt.contains("YYYY-MM-DD"));
    }

    #[test]
    fn parse_response_update_with_task_ids() {
        let json = r#"{"action":"update","match":{},"task_ids":[119,133,137],"set":{"due_date":"2026-03-04"},"description":"Set overdue tasks to today"}"#;
        let result = parse_response(json).unwrap();
        match result {
            NlpAction::Update { task_ids, set_fields, .. } => {
                assert_eq!(task_ids, Some(vec![119, 133, 137]));
                assert_eq!(set_fields.due_date, Some("2026-03-04".to_string()));
            }
            _ => panic!("Expected Update action"),
        }
    }

    #[test]
    fn system_prompt_includes_task_ids_in_update() {
        let ctx = build_task_context(&[make_task(1, "Test")]);
        let prompt = build_system_prompt(&ctx, "2026-03-04 (Wednesday)");
        assert!(prompt.contains("\"task_ids\":null"));
        assert!(prompt.contains("task_ids"));
    }

    #[test]
    fn query_tasks_tool_def_has_correct_name() {
        let def = query_tasks_tool_def();
        assert_eq!(def.name, "query_tasks");
        assert!(def.input_schema["properties"]["overdue"].is_object());
        assert!(def.input_schema["properties"]["status"].is_object());
        assert!(def.input_schema["properties"]["has_due_date"].is_object());
        assert!(def.input_schema["properties"]["has_recurrence"].is_object());
    }

    #[test]
    fn execute_query_tasks_filters_by_status() {
        let mut t1 = make_task(1, "Open task");
        t1.status = Status::Open;
        let mut t2 = make_task(2, "Done task");
        t2.status = Status::Done;
        let tasks = vec![t1, t2];

        let result = execute_query_tasks(&serde_json::json!({"status": "open"}), &tasks);
        let parsed: Vec<TaskSummary> = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].id, 1);
    }

    #[test]
    fn execute_query_tasks_filters_overdue() {
        let today = chrono::Local::now().date_naive();
        let yesterday = today - chrono::Duration::days(1);
        let tomorrow = today + chrono::Duration::days(1);

        let mut t1 = make_task(1, "Overdue");
        t1.due_date = Some(yesterday);
        t1.status = Status::Open;

        let mut t2 = make_task(2, "Due tomorrow");
        t2.due_date = Some(tomorrow);
        t2.status = Status::Open;

        let mut t3 = make_task(3, "Overdue but done");
        t3.due_date = Some(yesterday);
        t3.status = Status::Done;

        let mut t4 = make_task(4, "Due today");
        t4.due_date = Some(today);
        t4.status = Status::Open;

        let tasks = vec![t1, t2, t3, t4];
        let result = execute_query_tasks(&serde_json::json!({"overdue": true}), &tasks);
        let parsed: Vec<TaskSummary> = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].id, 1);
    }

    #[test]
    fn execute_query_tasks_combined_criteria() {
        let mut t1 = make_task(1, "High open");
        t1.priority = Priority::High;
        t1.status = Status::Open;

        let mut t2 = make_task(2, "High done");
        t2.priority = Priority::High;
        t2.status = Status::Done;

        let mut t3 = make_task(3, "Low open");
        t3.priority = Priority::Low;
        t3.status = Status::Open;

        let tasks = vec![t1, t2, t3];
        let result = execute_query_tasks(&serde_json::json!({"status": "open", "priority": "high"}), &tasks);
        let parsed: Vec<TaskSummary> = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].id, 1);
    }

    #[test]
    fn execute_query_tasks_caps_at_50() {
        let tasks: Vec<Task> = (1..=60).map(|i| make_task(i, &format!("Task {}", i))).collect();
        let result = execute_query_tasks(&serde_json::json!({}), &tasks);
        let parsed: Vec<TaskSummary> = serde_json::from_str(result.lines().next().unwrap()).unwrap();
        assert_eq!(parsed.len(), 50);
        assert!(result.contains("Showing 50 of 60"));
    }

    #[test]
    fn system_prompt_mentions_query_tasks() {
        let ctx = build_task_context(&[make_task(1, "Test")]);
        let prompt = build_system_prompt(&ctx, "2026-03-04 (Wednesday)");
        assert!(prompt.contains("query_tasks"));
        assert!(prompt.contains("date-based queries"));
    }
}
