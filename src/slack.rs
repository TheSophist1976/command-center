use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::config;

// -- Slack API types --

#[derive(Debug, Deserialize, Clone)]
pub struct SlackMessage {
    pub ts: String,
    pub text: String,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub channel: Option<String>,
    #[serde(default)]
    pub subtype: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SlackChannel {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub is_member: bool,
    #[serde(skip)]
    pub display_name: String,
    #[serde(skip)]
    pub conversation_type: String,
    #[serde(default)]
    pub user: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub is_channel: Option<bool>,
    #[serde(default)]
    pub is_group: Option<bool>,
    #[serde(default)]
    pub is_im: Option<bool>,
    #[serde(default)]
    pub is_mpim: Option<bool>,
}

fn resolve_conversation_type(ch: &SlackChannel) -> &'static str {
    if ch.is_im.unwrap_or(false) {
        "im"
    } else if ch.is_mpim.unwrap_or(false) {
        "mpim"
    } else if ch.is_group.unwrap_or(false) {
        "private"
    } else {
        "channel"
    }
}

#[derive(Debug, Deserialize)]
struct SlackChannelListResponse {
    ok: bool,
    #[serde(default)]
    channels: Vec<SlackChannel>,
    #[serde(default)]
    response_metadata: Option<ResponseMetadata>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SlackHistoryResponse {
    ok: bool,
    #[serde(default)]
    messages: Vec<SlackMessage>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResponseMetadata {
    #[serde(default)]
    next_cursor: Option<String>,
}

// -- API base URL --

pub fn api_base_url() -> String {
    std::env::var("SLACK_API_BASE_URL")
        .unwrap_or_else(|_| "https://slack.com/api".to_string())
}

// -- API calls --

pub struct FetchChannelsResult {
    pub channels: Vec<SlackChannel>,
    pub scope_warnings: Vec<String>,
}

pub fn fetch_channels(client: &reqwest::blocking::Client, token: &str, types: Option<&str>) -> Result<FetchChannelsResult, String> {
    let all_types = types.unwrap_or("public_channel,private_channel,mpim,im");
    let mut all_channels = Vec::new();
    let mut scope_warnings = Vec::new();
    let base = api_base_url();

    let type_list: Vec<&str> = all_types.split(',').map(|s| s.trim()).collect();

    // Fetch all conversation types concurrently
    let mut type_results: Vec<Result<(Vec<SlackChannel>, Vec<String>), String>> = Vec::new();
    std::thread::scope(|s| {
        let handles: Vec<_> = type_list.iter().map(|&conv_type| {
            let base = &base;
            s.spawn(move || -> Result<(Vec<SlackChannel>, Vec<String>), String> {
                let mut channels = Vec::new();
                let mut warnings = Vec::new();
                let mut cursor: Option<String> = None;
                loop {
                    let mut req = client
                        .get(format!("{}/conversations.list", base))
                        .bearer_auth(token)
                        .query(&[
                            ("types", conv_type),
                            ("exclude_archived", "true"),
                            ("limit", "200"),
                        ]);
                    if let Some(ref c) = cursor {
                        req = req.query(&[("cursor", c.as_str())]);
                    }

                    let response = req
                        .send()
                        .map_err(|e| format!("Failed to fetch Slack channels: {}", e))?;

                    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
                        return Err("Slack token is invalid or expired. Run `task auth slack` to re-authenticate.".to_string());
                    }
                    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                        return Err("Slack API rate limited. Please try again later.".to_string());
                    }
                    if !response.status().is_success() {
                        return Err(format!("Slack API error: {}", response.status()));
                    }

                    let body: SlackChannelListResponse = response
                        .json()
                        .map_err(|e| format!("Failed to parse Slack channels response: {}", e))?;

                    if !body.ok {
                        let err = body.error.unwrap_or_else(|| "unknown error".to_string());
                        if err == "missing_scope" {
                            warnings.push(format!("Missing scope for {} conversations", conv_type));
                            break;
                        }
                        return Err(format!("Slack API error: {}", err));
                    }

                    channels.extend(body.channels);

                    match body.response_metadata {
                        Some(meta) => match meta.next_cursor {
                            Some(c) if !c.is_empty() => cursor = Some(c),
                            _ => break,
                        },
                        None => break,
                    }
                }
                Ok((channels, warnings))
            })
        }).collect();

        for handle in handles {
            type_results.push(handle.join().unwrap_or_else(|_| Err("Thread panicked".to_string())));
        }
    });

    for result in type_results {
        match result {
            Ok((channels, warnings)) => {
                all_channels.extend(channels);
                scope_warnings.extend(warnings);
            }
            Err(e) => return Err(e),
        }
    }

    // Compute conversation_type and display_name for each channel
    for ch in &mut all_channels {
        let conv_type = resolve_conversation_type(ch);
        ch.conversation_type = conv_type.to_string();
        ch.display_name = match conv_type {
            "private" => format!("\u{1f512} #{}", ch.name),
            "im" => {
                // Display name will be resolved later with user cache
                let user_id = ch.user.as_deref().unwrap_or("unknown");
                format!("DM with {}", user_id)
            }
            "mpim" => {
                // MPIM names are auto-generated; will be resolved later
                format!("Group: {}", ch.name)
            }
            _ => format!("#{}", ch.name),
        };
    }

    Ok(FetchChannelsResult { channels: all_channels, scope_warnings })
}

/// Resolve display names for IM/MPIM conversations using the user cache.
/// Call after fetch_channels to replace raw user IDs with display names.
pub fn resolve_channel_display_names(
    channels: &mut [SlackChannel],
    client: &reqwest::blocking::Client,
    token: &str,
    cache: &mut HashMap<String, String>,
) {
    // Collect user IDs that need resolution
    let user_ids: Vec<String> = channels.iter()
        .filter(|ch| ch.conversation_type == "im")
        .filter_map(|ch| ch.user.clone())
        .collect();
    resolve_users_batch(client, token, &user_ids, cache);

    // Update display names
    for ch in channels.iter_mut() {
        match ch.conversation_type.as_str() {
            "im" => {
                if let Some(ref uid) = ch.user {
                    let name = cache.get(uid.as_str()).cloned().unwrap_or_else(|| uid.clone());
                    ch.display_name = format!("DM with {}", name);
                }
            }
            "mpim" => {
                // MPIM names from API are like "mpdm-user1--user2--1"
                // For now keep the API name; full resolution would need conversations.members
                // which is a separate API call. The name is usable as-is.
            }
            _ => {}
        }
    }
}

pub fn fetch_messages(
    client: &reqwest::blocking::Client,
    token: &str,
    channel_id: &str,
    oldest: Option<&str>,
    limit: usize,
) -> Result<Vec<SlackMessage>, String> {
    let base = api_base_url();
    let limit_str = limit.min(200).to_string();

    let mut req = client
        .get(format!("{}/conversations.history", base))
        .bearer_auth(token)
        .query(&[
            ("channel", channel_id),
            ("limit", &limit_str),
        ]);

    if let Some(ts) = oldest {
        req = req.query(&[("oldest", ts)]);
    }

    let response = req
        .send()
        .map_err(|e| format!("Failed to fetch Slack messages: {}", e))?;

    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err("Slack API rate limited. Please try again later.".to_string());
    }
    if !response.status().is_success() {
        return Err(format!("Slack API error: {}", response.status()));
    }

    let body: SlackHistoryResponse = response
        .json()
        .map_err(|e| format!("Failed to parse Slack messages response: {}", e))?;

    if !body.ok {
        let err = body.error.unwrap_or_else(|| "unknown error".to_string());
        if err == "not_in_channel" || err == "channel_not_found" {
            return Err(format!("Bot is not a member of channel {}. Add the bot to the channel first.", channel_id));
        }
        return Err(format!("Slack API error: {}", err));
    }

    Ok(body.messages)
}


// -- Channel info (unread state) --

#[derive(Debug, Clone)]
pub struct ChannelInfo {
    pub last_read: Option<String>,
    pub latest_ts: Option<String>,
}

impl ChannelInfo {
    pub fn has_unread(&self) -> bool {
        match (&self.last_read, &self.latest_ts) {
            (Some(lr), Some(lt)) => lt.as_str() > lr.as_str(),
            // No latest info (common for non-DM channels) — assume might have unread
            (Some(_), None) => true,
            (None, Some(_)) => true,
            (None, None) => true,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ConversationInfoResponse {
    ok: bool,
    #[serde(default)]
    channel: Option<ConversationInfoChannel>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ConversationInfoChannel {
    #[serde(default)]
    last_read: Option<String>,
    #[serde(default)]
    latest: Option<ConversationLatest>,
}

#[derive(Debug, Deserialize)]
struct ConversationLatest {
    #[serde(default)]
    ts: Option<String>,
}

pub fn fetch_channel_info(client: &reqwest::blocking::Client, token: &str, channel_id: &str) -> Result<ChannelInfo, String> {
    let base = api_base_url();

    let response = client
        .get(format!("{}/conversations.info", base))
        .bearer_auth(token)
        .query(&[("channel", channel_id)])
        .send()
        .map_err(|e| format!("Failed to fetch channel info: {}", e))?;

    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err("Slack API rate limited. Please try again later.".to_string());
    }
    if !response.status().is_success() {
        return Err(format!("Slack API error: {}", response.status()));
    }

    let body: ConversationInfoResponse = response
        .json()
        .map_err(|e| format!("Failed to parse conversations.info response: {}", e))?;

    if !body.ok {
        let err = body.error.unwrap_or_else(|| "unknown error".to_string());
        return Err(format!("Slack API error: {}", err));
    }

    let channel = body.channel.unwrap_or(ConversationInfoChannel { last_read: None, latest: None });
    Ok(ChannelInfo {
        last_read: channel.last_read,
        latest_ts: channel.latest.and_then(|l| l.ts),
    })
}

#[derive(Debug, Deserialize)]
struct SlackMarkResponse {
    ok: bool,
    #[serde(default)]
    error: Option<String>,
}

pub fn mark_read(client: &reqwest::blocking::Client, token: &str, channel_id: &str, ts: &str) -> Result<(), String> {
    let base = api_base_url();

    let body = serde_json::json!({
        "channel": channel_id,
        "ts": ts,
    });

    let response = client
        .post(format!("{}/conversations.mark", base))
        .bearer_auth(token)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| format!("Failed to call conversations.mark: {}", e))?;

    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err("Slack API rate limited. Try again later.".to_string());
    }
    if !response.status().is_success() {
        return Err(format!("Slack API error: {}", response.status()));
    }

    let resp: SlackMarkResponse = response
        .json()
        .map_err(|e| format!("Failed to parse conversations.mark response: {}", e))?;

    if !resp.ok {
        let err = resp.error.unwrap_or_else(|| "unknown error".to_string());
        if err == "missing_scope" {
            return Err("Could not sync read state to Slack — add write scopes to your app".to_string());
        }
        return Err(format!("Slack API error: {}", err));
    }

    Ok(())
}

// -- User name cache --

fn slack_users_cache_path() -> PathBuf {
    let base = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"));
    base.join("task-manager").join("slack_users.json")
}

pub fn read_user_cache() -> HashMap<String, String> {
    let path = slack_users_cache_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn write_user_cache(cache: &HashMap<String, String>) -> Result<(), String> {
    let path = slack_users_cache_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }
    let json = serde_json::to_string_pretty(cache)
        .map_err(|e| format!("Failed to serialize user cache: {}", e))?;
    fs::write(&path, json)
        .map_err(|e| format!("Failed to write user cache: {}", e))?;
    Ok(())
}

pub fn fetch_user_info(client: &reqwest::blocking::Client, token: &str, user_id: &str) -> Result<String, String> {
    let base = api_base_url();

    let response = client
        .get(format!("{}/users.info", base))
        .bearer_auth(token)
        .query(&[("user", user_id)])
        .send()
        .map_err(|e| format!("Failed to fetch user info: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Slack API error: {}", response.status()));
    }

    let body: serde_json::Value = response
        .json()
        .map_err(|e| format!("Failed to parse user info response: {}", e))?;

    if !body["ok"].as_bool().unwrap_or(false) {
        let err = body["error"].as_str().unwrap_or("unknown error");
        return Err(format!("Slack API error: {}", err));
    }

    let profile = &body["user"]["profile"];
    let display = profile["display_name"].as_str().filter(|s| !s.is_empty());
    let real = profile["real_name"].as_str().filter(|s| !s.is_empty());
    let real_norm = profile["real_name_normalized"].as_str().filter(|s| !s.is_empty());
    let name = body["user"]["name"].as_str().filter(|s| !s.is_empty());

    Ok(display.or(real).or(real_norm).or(name)
        .unwrap_or(user_id)
        .to_string())
}

pub fn resolve_users_batch(client: &reqwest::blocking::Client, token: &str, user_ids: &[String], cache: &mut HashMap<String, String>) {
    let unique: Vec<String> = user_ids.iter()
        .filter(|id| !cache.contains_key(id.as_str()))
        .map(|id| id.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    if unique.is_empty() {
        return;
    }

    let mut resolved: Vec<(String, String)> = Vec::new();
    let mut missing_scope = false;
    'chunks: for chunk in unique.chunks(6) {
        let mut chunk_missing_scope = false;
        std::thread::scope(|s| {
            let handles: Vec<_> = chunk.iter().map(|user_id| {
                s.spawn(move || -> Result<Option<(String, String)>, ()> {
                    match fetch_user_info(client, token, user_id) {
                        Ok(name) => Ok(Some((user_id.clone(), name))),
                        Err(e) if e.contains("missing_scope") => Err(()),
                        Err(e) => {
                            eprintln!("Warning: could not resolve user {}: {}", user_id, e);
                            Ok(None)
                        }
                    }
                })
            }).collect();
            for handle in handles {
                match handle.join() {
                    Ok(Ok(Some(pair))) => resolved.push(pair),
                    Ok(Err(())) => { chunk_missing_scope = true; }
                    _ => {}
                }
            }
        });
        if chunk_missing_scope {
            missing_scope = true;
            break 'chunks;
        }
    }

    if missing_scope {
        eprintln!("Warning: Slack token is missing 'users:read' scope — user names cannot be resolved. Re-authenticate with `task auth slack`.");
    }

    for (id, name) in resolved {
        cache.insert(id, name);
    }
}


// -- Slack Inbox Storage --

#[derive(Debug, Clone, PartialEq)]
pub enum InboxMessageStatus {
    Open,
    Done,
}

#[derive(Debug, Clone)]
pub struct SlackInboxMessage {
    pub channel_id: String,
    pub channel_name: String,
    pub user_id: String,
    pub user_name: String,
    pub text: String,
    pub ts: String,
    pub status: InboxMessageStatus,
    pub link: String,
}

#[derive(Debug, Clone)]
pub struct SlackInbox {
    pub workspace: String,
    pub last_sync: Option<DateTime<Utc>>,
    pub messages: Vec<SlackInboxMessage>,
}

impl SlackInbox {
    pub fn new() -> Self {
        Self {
            workspace: String::new(),
            last_sync: None,
            messages: Vec::new(),
        }
    }

    pub fn open_messages(&self) -> Vec<&SlackInboxMessage> {
        self.messages.iter()
            .filter(|m| m.status == InboxMessageStatus::Open)
            .collect()
    }
}

pub fn inbox_path() -> PathBuf {
    let base = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"));
    base.join("task-manager").join("slack").join("inbox.md")
}

pub fn deep_link(workspace: &str, channel_id: &str, ts: &str) -> String {
    let ts_no_dot = ts.replace('.', "");
    format!("https://{}.slack.com/archives/{}/p{}", workspace, channel_id, ts_no_dot)
}

pub fn load_inbox() -> SlackInbox {
    let path = inbox_path();
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return SlackInbox::new(),
    };
    parse_inbox(&content)
}

fn parse_inbox(content: &str) -> SlackInbox {
    let mut inbox = SlackInbox::new();
    let mut current_heading = String::new();
    let mut current_meta: Option<(String, String, String, String)> = None; // ts, channel, user, status
    let mut current_link = String::new();

    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("<!-- workspace: ") {
            if let Some(val) = rest.strip_suffix(" -->") {
                inbox.workspace = val.to_string();
            }
        } else if let Some(rest) = line.strip_prefix("<!-- last-sync: ") {
            if let Some(val) = rest.strip_suffix(" -->") {
                inbox.last_sync = DateTime::parse_from_rfc3339(val)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc));
            }
        } else if line.starts_with("<!-- hwm:") {
            // Legacy HWM lines — silently ignore
        } else if line.starts_with("## ") {
            // Save previous message if any
            if let Some((ts, channel, user, status)) = current_meta.take() {
                let heading_text = current_heading.trim().to_string();
                // Extract channel_name and user_name from heading: [channel_name] user_name: text
                let (channel_name, user_name, text) = parse_heading(&heading_text);
                inbox.messages.push(SlackInboxMessage {
                    channel_id: channel,
                    channel_name,
                    user_id: user,
                    user_name,
                    text,
                    ts,
                    status: if status == "done" { InboxMessageStatus::Done } else { InboxMessageStatus::Open },
                    link: current_link.clone(),
                });
                current_link.clear();
            }
            current_heading = line[3..].to_string();
        } else if line.starts_with("<!-- ts:") {
            // Parse: <!-- ts:XXX channel:YYY user:ZZZ status:SSS -->
            let meta = line.trim_start_matches("<!-- ").trim_end_matches(" -->");
            let mut ts = String::new();
            let mut channel = String::new();
            let mut user = String::new();
            let mut status = String::new();
            for part in meta.split_whitespace() {
                if let Some(v) = part.strip_prefix("ts:") { ts = v.to_string(); }
                else if let Some(v) = part.strip_prefix("channel:") { channel = v.to_string(); }
                else if let Some(v) = part.strip_prefix("user:") { user = v.to_string(); }
                else if let Some(v) = part.strip_prefix("status:") { status = v.to_string(); }
            }
            current_meta = Some((ts, channel, user, status));
        } else if line.starts_with("<!-- link: ") {
            if let Some(val) = line.strip_prefix("<!-- link: ").and_then(|r| r.strip_suffix(" -->")) {
                current_link = val.to_string();
            }
        }
    }

    // Save last message
    if let Some((ts, channel, user, status)) = current_meta.take() {
        let heading_text = current_heading.trim().to_string();
        let (channel_name, user_name, text) = parse_heading(&heading_text);
        inbox.messages.push(SlackInboxMessage {
            channel_id: channel,
            channel_name,
            user_id: user,
            user_name,
            text,
            ts,
            status: if status == "done" { InboxMessageStatus::Done } else { InboxMessageStatus::Open },
            link: current_link,
        });
    }

    inbox
}

fn parse_heading(heading: &str) -> (String, String, String) {
    // Format: [channel_name] user_name: text
    if let Some(rest) = heading.strip_prefix('[') {
        if let Some(bracket_end) = rest.find(']') {
            let channel_name = rest[..bracket_end].to_string();
            let after_bracket = rest[bracket_end + 1..].trim_start();
            if let Some(colon_pos) = after_bracket.find(": ") {
                let user_name = after_bracket[..colon_pos].to_string();
                let text = after_bracket[colon_pos + 2..].to_string();
                return (channel_name, user_name, text);
            }
            return (channel_name, String::new(), after_bracket.to_string());
        }
    }
    (String::new(), String::new(), heading.to_string())
}

pub fn save_inbox(inbox: &SlackInbox) -> Result<(), String> {
    let path = inbox_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create inbox directory: {}", e))?;
    }

    let content = serialize_inbox(inbox);

    // Write atomically via temp file
    let tmp_path = path.with_extension("md.tmp");
    let mut file = fs::File::create(&tmp_path)
        .map_err(|e| format!("Failed to create temp inbox file: {}", e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write temp inbox file: {}", e))?;
    file.flush()
        .map_err(|e| format!("Failed to flush temp inbox file: {}", e))?;
    drop(file);

    fs::rename(&tmp_path, &path)
        .map_err(|e| format!("Failed to rename temp inbox file: {}", e))?;

    Ok(())
}

fn serialize_inbox(inbox: &SlackInbox) -> String {
    let mut out = String::new();
    out.push_str("<!-- slack-inbox -->\n");
    if !inbox.workspace.is_empty() {
        out.push_str(&format!("<!-- workspace: {} -->\n", inbox.workspace));
    }
    if let Some(sync) = inbox.last_sync {
        out.push_str(&format!("<!-- last-sync: {} -->\n", sync.to_rfc3339()));
    }
    out.push('\n');

    for msg in &inbox.messages {
        let status_str = match msg.status {
            InboxMessageStatus::Open => "open",
            InboxMessageStatus::Done => "done",
        };
        out.push_str(&format!("## [{}] {}: {}\n", msg.channel_name, msg.user_name, msg.text));
        out.push_str(&format!("<!-- ts:{} channel:{} user:{} status:{} -->\n", msg.ts, msg.channel_id, msg.user_id, status_str));
        out.push_str(&format!("<!-- link: {} -->\n\n", msg.link));
    }

    out
}

pub fn inbox_has_message(inbox: &SlackInbox, channel_id: &str, ts: &str) -> bool {
    inbox.messages.iter().any(|m| m.channel_id == channel_id && m.ts == ts)
}

pub fn prune_done_messages(inbox: &mut SlackInbox) {
    let cutoff = Utc::now().timestamp() as f64 - (7.0 * 24.0 * 60.0 * 60.0);
    inbox.messages.retain(|m| {
        if m.status != InboxMessageStatus::Done {
            return true;
        }
        // Parse ts as float seconds
        let msg_ts: f64 = m.ts.parse().unwrap_or(0.0);
        msg_ts >= cutoff
    });
}

// -- Workspace name --

pub fn fetch_workspace_name(client: &reqwest::blocking::Client, token: &str) -> Result<String, String> {
    // Check config cache first
    if let Some(name) = config::read_config_value("slack-workspace") {
        if !name.is_empty() {
            return Ok(name);
        }
    }

    let base = api_base_url();

    let response = client
        .post(format!("{}/auth.test", base))
        .bearer_auth(token)
        .send()
        .map_err(|e| format!("Failed to call auth.test: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("Slack API error: {}", response.status()));
    }

    let body: serde_json::Value = response
        .json()
        .map_err(|e| format!("Failed to parse auth.test response: {}", e))?;

    if !body["ok"].as_bool().unwrap_or(false) {
        let err = body["error"].as_str().unwrap_or("unknown error");
        return Err(format!("Slack API error: {}", err));
    }

    // Extract workspace name from url field (e.g., "https://myteam.slack.com/")
    let url = body["url"].as_str().unwrap_or("");
    let workspace = url
        .strip_prefix("https://")
        .and_then(|s| s.strip_suffix(".slack.com/"))
        .unwrap_or_else(|| {
            // Fallback to team field
            body["team"].as_str().unwrap_or("workspace")
        })
        .to_string();

    let _ = config::write_config_value("slack-workspace", &workspace);
    Ok(workspace)
}

// -- Send message --

#[derive(Debug, Deserialize)]
struct SlackPostMessageResponse {
    ok: bool,
    #[serde(default)]
    error: Option<String>,
}

pub fn send_message(client: &reqwest::blocking::Client, token: &str, channel_id: &str, text: &str) -> Result<(), String> {
    let base = api_base_url();

    let body = serde_json::json!({
        "channel": channel_id,
        "text": text,
    });

    let response = client
        .post(format!("{}/chat.postMessage", base))
        .bearer_auth(token)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| format!("Failed to send Slack message: {}", e))?;

    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err("Slack API rate limited. Try again later.".to_string());
    }
    if !response.status().is_success() {
        return Err(format!("Slack API error: {}", response.status()));
    }

    let resp: SlackPostMessageResponse = response
        .json()
        .map_err(|e| format!("Failed to parse chat.postMessage response: {}", e))?;

    if !resp.ok {
        let err = resp.error.unwrap_or_else(|| "unknown error".to_string());
        if err == "missing_scope" {
            return Err("Missing chat:write scope. Re-authenticate with `task auth slack`.".to_string());
        }
        return Err(format!("Slack API error: {}", err));
    }

    Ok(())
}

// -- Relative timestamp --

pub fn relative_time(ts: &str) -> String {
    let msg_ts: f64 = ts.parse().unwrap_or(0.0);
    let now = Utc::now().timestamp() as f64;
    let diff = (now - msg_ts).max(0.0) as u64;

    if diff < 60 {
        "just now".to_string()
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else if diff < 172800 {
        "yesterday".to_string()
    } else {
        format!("{}d ago", diff / 86400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- api_base_url --

    static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_api_base_url_default() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let original = std::env::var("SLACK_API_BASE_URL").ok();
        unsafe { std::env::remove_var("SLACK_API_BASE_URL"); }
        let url = api_base_url();
        assert_eq!(url, "https://slack.com/api");
        if let Some(val) = original {
            unsafe { std::env::set_var("SLACK_API_BASE_URL", val); }
        }
    }

    #[test]
    fn test_api_base_url_override() {
        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let original = std::env::var("SLACK_API_BASE_URL").ok();
        unsafe { std::env::set_var("SLACK_API_BASE_URL", "http://localhost:9999"); }
        let url = api_base_url();
        assert_eq!(url, "http://localhost:9999");
        if let Some(val) = original {
            unsafe { std::env::set_var("SLACK_API_BASE_URL", val); }
        } else {
            unsafe { std::env::remove_var("SLACK_API_BASE_URL"); }
        }
    }

    // -- API type deserialization --

    #[test]
    fn test_deserialize_slack_message() {
        let json = r#"{"ts":"1709654321.000100","text":"Hello world","user":"U123"}"#;
        let msg: SlackMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.ts, "1709654321.000100");
        assert_eq!(msg.text, "Hello world");
        assert_eq!(msg.user, Some("U123".to_string()));
    }

    #[test]
    fn test_deserialize_slack_message_minimal() {
        let json = r#"{"ts":"1.0","text":"hi"}"#;
        let msg: SlackMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.ts, "1.0");
        assert_eq!(msg.text, "hi");
        assert!(msg.user.is_none());
        assert!(msg.channel.is_none());
    }

    #[test]
    fn test_deserialize_slack_channel() {
        let json = r#"{"id":"C123","name":"general","is_member":true,"is_channel":true}"#;
        let ch: SlackChannel = serde_json::from_str(json).unwrap();
        assert_eq!(ch.id, "C123");
        assert_eq!(ch.name, "general");
        assert!(ch.is_member);
        assert_eq!(ch.is_channel, Some(true));
        assert_eq!(ch.display_name, "");
        assert_eq!(ch.conversation_type, "");
    }

    #[test]
    fn test_deserialize_slack_channel_im() {
        let json = r#"{"id":"D123","name":"","is_im":true,"user":"U456","is_member":true}"#;
        let ch: SlackChannel = serde_json::from_str(json).unwrap();
        assert_eq!(ch.id, "D123");
        assert!(ch.is_im.unwrap_or(false));
        assert_eq!(ch.user, Some("U456".to_string()));
    }

    #[test]
    fn test_resolve_conversation_type() {
        let mut ch = SlackChannel {
            id: "C1".into(), name: "general".into(), is_member: true,
            display_name: String::new(), conversation_type: String::new(),
            user: None, is_channel: Some(true), is_group: None, is_im: None, is_mpim: None,
        };
        assert_eq!(resolve_conversation_type(&ch), "channel");

        ch.is_channel = None; ch.is_group = Some(true);
        assert_eq!(resolve_conversation_type(&ch), "private");

        ch.is_group = None; ch.is_im = Some(true);
        assert_eq!(resolve_conversation_type(&ch), "im");

        ch.is_im = None; ch.is_mpim = Some(true);
        assert_eq!(resolve_conversation_type(&ch), "mpim");
    }

    #[test]
    fn test_deserialize_channel_list_response() {
        let json = r#"{
            "ok": true,
            "channels": [
                {"id":"C1","name":"general","is_member":true,"is_channel":true},
                {"id":"C2","name":"random","is_member":false,"is_channel":true}
            ],
            "response_metadata": {"next_cursor": "abc123"}
        }"#;
        let resp: SlackChannelListResponse = serde_json::from_str(json).unwrap();
        assert!(resp.ok);
        assert_eq!(resp.channels.len(), 2);
        assert_eq!(resp.response_metadata.unwrap().next_cursor, Some("abc123".to_string()));
    }

    #[test]
    fn test_deserialize_history_response() {
        let json = r#"{
            "ok": true,
            "messages": [
                {"ts":"1.0","text":"hello","user":"U1"},
                {"ts":"2.0","text":"world"}
            ]
        }"#;
        let resp: SlackHistoryResponse = serde_json::from_str(json).unwrap();
        assert!(resp.ok);
        assert_eq!(resp.messages.len(), 2);
    }

    #[test]
    fn test_deserialize_error_response() {
        let json = r#"{"ok": false, "error": "invalid_auth"}"#;
        let resp: SlackHistoryResponse = serde_json::from_str(json).unwrap();
        assert!(!resp.ok);
        assert_eq!(resp.error, Some("invalid_auth".to_string()));
    }

    // -- resolve_conversation_type --

    #[test]
    fn test_resolve_conversation_type_channel() {
        let ch = SlackChannel {
            id: "C1".into(), name: "general".into(), is_member: true,
            display_name: String::new(), conversation_type: String::new(),
            user: None, is_channel: Some(true), is_group: None, is_im: None, is_mpim: None,
        };
        assert_eq!(resolve_conversation_type(&ch), "channel");
    }

    #[test]
    fn test_resolve_conversation_type_private() {
        let ch = SlackChannel {
            id: "G1".into(), name: "secret".into(), is_member: true,
            display_name: String::new(), conversation_type: String::new(),
            user: None, is_channel: None, is_group: Some(true), is_im: None, is_mpim: None,
        };
        assert_eq!(resolve_conversation_type(&ch), "private");
    }

    #[test]
    fn test_resolve_conversation_type_im() {
        let ch = SlackChannel {
            id: "D1".into(), name: "".into(), is_member: true,
            display_name: String::new(), conversation_type: String::new(),
            user: Some("U123".into()), is_channel: None, is_group: None, is_im: Some(true), is_mpim: None,
        };
        assert_eq!(resolve_conversation_type(&ch), "im");
    }

    #[test]
    fn test_resolve_conversation_type_mpim() {
        let ch = SlackChannel {
            id: "G2".into(), name: "mpdm-a--b--1".into(), is_member: true,
            display_name: String::new(), conversation_type: String::new(),
            user: None, is_channel: None, is_group: None, is_im: None, is_mpim: Some(true),
        };
        assert_eq!(resolve_conversation_type(&ch), "mpim");
    }

    // -- user cache --

    #[test]
    fn test_user_cache_read_nonexistent() {
        let cache = read_user_cache();
        assert!(cache.is_empty() || !cache.is_empty());
    }

    #[test]
    fn test_resolve_users_batch_deduplicates() {
        let client = reqwest::blocking::Client::new();
        let mut cache = HashMap::new();
        cache.insert("U1".to_string(), "Alice".to_string());
        let ids = vec!["U1".to_string(), "U1".to_string(), "U1".to_string()];
        resolve_users_batch(&client, "fake-token", &ids, &mut cache);
        assert_eq!(cache.get("U1"), Some(&"Alice".to_string()));
    }

    // -- display name generation --

    #[test]
    fn test_display_name_public_channel() {
        let mut channels = vec![SlackChannel {
            id: "C1".into(), name: "general".into(), is_member: true,
            display_name: String::new(), conversation_type: String::new(),
            user: None, is_channel: Some(true), is_group: None, is_im: None, is_mpim: None,
        }];
        for ch in &mut channels {
            ch.conversation_type = resolve_conversation_type(ch).to_string();
            ch.display_name = match ch.conversation_type.as_str() {
                "private" => format!("\u{1f512} #{}", ch.name),
                "im" => format!("DM with {}", ch.user.as_deref().unwrap_or("unknown")),
                "mpim" => format!("Group: {}", ch.name),
                _ => format!("#{}", ch.name),
            };
        }
        assert_eq!(channels[0].display_name, "#general");
        assert_eq!(channels[0].conversation_type, "channel");
    }

    #[test]
    fn test_display_name_private_channel() {
        let mut ch = SlackChannel {
            id: "G1".into(), name: "secret".into(), is_member: true,
            display_name: String::new(), conversation_type: String::new(),
            user: None, is_channel: None, is_group: Some(true), is_im: None, is_mpim: None,
        };
        ch.conversation_type = resolve_conversation_type(&ch).to_string();
        ch.display_name = format!("\u{1f512} #{}", ch.name);
        assert_eq!(ch.display_name, "\u{1f512} #secret");
    }

    #[test]
    fn test_display_name_im_with_cache() {
        let mut channels = vec![SlackChannel {
            id: "D1".into(), name: "".into(), is_member: true,
            display_name: "DM with U456".into(), conversation_type: "im".into(),
            user: Some("U456".into()), is_channel: None, is_group: None, is_im: Some(true), is_mpim: None,
        }];
        let client = reqwest::blocking::Client::new();
        let mut cache = HashMap::new();
        cache.insert("U456".to_string(), "Bob".to_string());
        resolve_channel_display_names(&mut channels, &client, "fake-token", &mut cache);
        assert_eq!(channels[0].display_name, "DM with Bob");
    }

    #[test]
    fn test_deserialize_mixed_conversation_types() {
        let json = r#"{
            "ok": true,
            "channels": [
                {"id":"C1","name":"general","is_member":true,"is_channel":true},
                {"id":"G1","name":"secret","is_member":true,"is_group":true},
                {"id":"D1","name":"","is_member":true,"is_im":true,"user":"U123"},
                {"id":"G2","name":"mpdm-a--b--1","is_member":true,"is_mpim":true}
            ]
        }"#;
        let resp: SlackChannelListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.channels.len(), 4);
        assert_eq!(resolve_conversation_type(&resp.channels[0]), "channel");
        assert_eq!(resolve_conversation_type(&resp.channels[1]), "private");
        assert_eq!(resolve_conversation_type(&resp.channels[2]), "im");
        assert_eq!(resp.channels[2].user, Some("U123".to_string()));
        assert_eq!(resolve_conversation_type(&resp.channels[3]), "mpim");
    }

    // -- deep link --

    #[test]
    fn test_deep_link() {
        let link = deep_link("myteam", "C0123ABC", "1709654321.000100");
        assert_eq!(link, "https://myteam.slack.com/archives/C0123ABC/p1709654321000100");
    }

    #[test]
    fn test_deep_link_no_dot() {
        let link = deep_link("team", "C1", "12345");
        assert_eq!(link, "https://team.slack.com/archives/C1/p12345");
    }

    // -- inbox load/save roundtrip --

    #[test]
    fn test_inbox_roundtrip() {
        let mut inbox = SlackInbox::new();
        inbox.workspace = "testteam".to_string();
        inbox.last_sync = Some(Utc::now());
        inbox.messages.push(SlackInboxMessage {
            channel_id: "C1".to_string(),
            channel_name: "#general".to_string(),
            user_id: "U1".to_string(),
            user_name: "Alice".to_string(),
            text: "Hello world".to_string(),
            ts: "100.0".to_string(),
            status: InboxMessageStatus::Open,
            link: "https://testteam.slack.com/archives/C1/p1000".to_string(),
        });
        inbox.messages.push(SlackInboxMessage {
            channel_id: "C2".to_string(),
            channel_name: "#random".to_string(),
            user_id: "U2".to_string(),
            user_name: "Bob".to_string(),
            text: "Done task".to_string(),
            ts: "200.0".to_string(),
            status: InboxMessageStatus::Done,
            link: "https://testteam.slack.com/archives/C2/p2000".to_string(),
        });

        let serialized = serialize_inbox(&inbox);
        let loaded = parse_inbox(&serialized);

        assert_eq!(loaded.workspace, "testteam");
        assert!(loaded.last_sync.is_some());
        assert_eq!(loaded.messages.len(), 2);
        assert_eq!(loaded.messages[0].channel_name, "#general");
        assert_eq!(loaded.messages[0].user_name, "Alice");
        assert_eq!(loaded.messages[0].text, "Hello world");
        assert_eq!(loaded.messages[0].status, InboxMessageStatus::Open);
        assert_eq!(loaded.messages[1].status, InboxMessageStatus::Done);
    }

    #[test]
    fn test_inbox_empty_parse() {
        let inbox = parse_inbox("");
        assert!(inbox.messages.is_empty());
        assert!(inbox.workspace.is_empty());
    }

    // -- deduplication --

    #[test]
    fn test_inbox_has_message() {
        let mut inbox = SlackInbox::new();
        inbox.messages.push(SlackInboxMessage {
            channel_id: "C1".to_string(),
            channel_name: "#general".to_string(),
            user_id: "U1".to_string(),
            user_name: "Alice".to_string(),
            text: "test".to_string(),
            ts: "100.0".to_string(),
            status: InboxMessageStatus::Open,
            link: String::new(),
        });

        assert!(inbox_has_message(&inbox, "C1", "100.0"));
        assert!(!inbox_has_message(&inbox, "C1", "200.0"));
        assert!(!inbox_has_message(&inbox, "C2", "100.0"));
    }

    // -- pruning --

    #[test]
    fn test_prune_done_messages() {
        let mut inbox = SlackInbox::new();
        let old_ts = "1000000.0"; // very old
        let recent_ts = format!("{}.0", Utc::now().timestamp());

        inbox.messages.push(SlackInboxMessage {
            channel_id: "C1".to_string(), channel_name: "#a".to_string(),
            user_id: "U1".to_string(), user_name: "A".to_string(),
            text: "old done".to_string(), ts: old_ts.to_string(),
            status: InboxMessageStatus::Done, link: String::new(),
        });
        inbox.messages.push(SlackInboxMessage {
            channel_id: "C1".to_string(), channel_name: "#a".to_string(),
            user_id: "U1".to_string(), user_name: "A".to_string(),
            text: "recent done".to_string(), ts: recent_ts.clone(),
            status: InboxMessageStatus::Done, link: String::new(),
        });
        inbox.messages.push(SlackInboxMessage {
            channel_id: "C1".to_string(), channel_name: "#a".to_string(),
            user_id: "U1".to_string(), user_name: "A".to_string(),
            text: "old open".to_string(), ts: old_ts.to_string(),
            status: InboxMessageStatus::Open, link: String::new(),
        });

        prune_done_messages(&mut inbox);

        assert_eq!(inbox.messages.len(), 2);
        assert_eq!(inbox.messages[0].text, "recent done");
        assert_eq!(inbox.messages[1].text, "old open");
    }

    // -- relative time --

    #[test]
    fn test_relative_time() {
        let now = Utc::now().timestamp() as f64;
        assert_eq!(relative_time(&format!("{}", now)), "just now");
        assert_eq!(relative_time(&format!("{}", now - 300.0)), "5m ago");
        assert_eq!(relative_time(&format!("{}", now - 7200.0)), "2h ago");
        assert_eq!(relative_time(&format!("{}", now - 90000.0)), "yesterday");
        assert_eq!(relative_time(&format!("{}", now - 259200.0)), "3d ago");
    }

    // -- parse heading --

    #[test]
    fn test_parse_heading() {
        let (ch, user, text) = parse_heading("[#general] Alice: Hello world");
        assert_eq!(ch, "#general");
        assert_eq!(user, "Alice");
        assert_eq!(text, "Hello world");
    }

    #[test]
    fn test_parse_heading_no_bracket() {
        let (ch, user, text) = parse_heading("Some random heading");
        assert_eq!(ch, "");
        assert_eq!(user, "");
        assert_eq!(text, "Some random heading");
    }

    // -- send_message response deserialization --

    #[test]
    fn test_deserialize_post_message_response() {
        let json = r#"{"ok": true}"#;
        let resp: SlackPostMessageResponse = serde_json::from_str(json).unwrap();
        assert!(resp.ok);
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_deserialize_post_message_error() {
        let json = r#"{"ok": false, "error": "not_in_channel"}"#;
        let resp: SlackPostMessageResponse = serde_json::from_str(json).unwrap();
        assert!(!resp.ok);
        assert_eq!(resp.error, Some("not_in_channel".to_string()));
    }

    // -- conversations.info response parsing --

    #[test]
    fn test_deserialize_channel_info_with_unread() {
        let json = r#"{
            "ok": true,
            "channel": {
                "last_read": "1709654321.000100",
                "latest": {"ts": "1709654322.000200"}
            }
        }"#;
        let resp: ConversationInfoResponse = serde_json::from_str(json).unwrap();
        assert!(resp.ok);
        let ch = resp.channel.unwrap();
        assert_eq!(ch.last_read, Some("1709654321.000100".to_string()));
        assert_eq!(ch.latest.unwrap().ts, Some("1709654322.000200".to_string()));
    }

    #[test]
    fn test_deserialize_channel_info_no_unread() {
        let json = r#"{
            "ok": true,
            "channel": {
                "last_read": "1709654322.000200",
                "latest": {"ts": "1709654322.000200"}
            }
        }"#;
        let resp: ConversationInfoResponse = serde_json::from_str(json).unwrap();
        let ch = resp.channel.unwrap();
        let info = ChannelInfo {
            last_read: ch.last_read,
            latest_ts: ch.latest.and_then(|l| l.ts),
        };
        assert!(!info.has_unread());
    }

    #[test]
    fn test_channel_info_has_unread() {
        let info = ChannelInfo {
            last_read: Some("100.0".to_string()),
            latest_ts: Some("200.0".to_string()),
        };
        assert!(info.has_unread());

        let info_no = ChannelInfo {
            last_read: Some("200.0".to_string()),
            latest_ts: Some("200.0".to_string()),
        };
        assert!(!info_no.has_unread());

        let info_none = ChannelInfo {
            last_read: None,
            latest_ts: Some("100.0".to_string()),
        };
        assert!(info_none.has_unread());

        // No latest info (common for non-DM channels) — assume might have unread
        let info_no_latest = ChannelInfo {
            last_read: Some("100.0".to_string()),
            latest_ts: None,
        };
        assert!(info_no_latest.has_unread());

        let info_empty = ChannelInfo {
            last_read: None,
            latest_ts: None,
        };
        assert!(info_empty.has_unread());
    }

    // -- conversations.mark response parsing --

    #[test]
    fn test_deserialize_mark_response_success() {
        let json = r#"{"ok": true}"#;
        let resp: SlackMarkResponse = serde_json::from_str(json).unwrap();
        assert!(resp.ok);
    }

    #[test]
    fn test_deserialize_mark_response_error() {
        let json = r#"{"ok": false, "error": "missing_scope"}"#;
        let resp: SlackMarkResponse = serde_json::from_str(json).unwrap();
        assert!(!resp.ok);
        assert_eq!(resp.error, Some("missing_scope".to_string()));
    }

    // -- legacy HWM backward compat --

    #[test]
    fn test_legacy_hwm_lines_ignored() {
        let content = "<!-- slack-inbox -->\n<!-- workspace: testteam -->\n<!-- hwm:C1:100.0 -->\n<!-- hwm:C2:200.0 -->\n\n## [#general] Alice: Hello\n<!-- ts:100.0 channel:C1 user:U1 status:open -->\n<!-- link: https://testteam.slack.com/archives/C1/p1000 -->\n";
        let inbox = parse_inbox(content);
        assert_eq!(inbox.workspace, "testteam");
        assert_eq!(inbox.messages.len(), 1);
        assert_eq!(inbox.messages[0].text, "Hello");
    }
}
