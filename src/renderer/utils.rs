use std::fs;
use std::path::Path;
use serde_json::Value;
use chrono::{DateTime, FixedOffset, Local};

use crate::frs::avatars::resolve_avatar;

/// Struct holding normalized message info
pub struct MessageInfo {
    pub date_str: String,
    pub time_str: String,
    pub name: String,
    pub emoji: String,
    pub text: String,
    pub markdown: Option<String>,
    pub children: Vec<String>,
}

/// Load and normalize a message JSON
pub fn load_message(fur_dir: &Path, msg_id: &str, avatars: &Value) -> Option<MessageInfo> {
    let msg_path = fur_dir.join("messages").join(format!("{}.json", msg_id));
    let msg_content = fs::read_to_string(&msg_path).ok()?;
    let msg_json: Value = serde_json::from_str(&msg_content).ok()?;

    // Timestamp
    let raw_time = msg_json["timestamp"].as_str().unwrap_or("???");
    let (date_str, time_str) = if let Ok(dt) = raw_time.parse::<DateTime<FixedOffset>>() {
        let local_dt = dt.with_timezone(&Local);
        (local_dt.format("%Y-%m-%d").to_string(), local_dt.format("%H:%M:%S").to_string())
    } else {
        (raw_time.to_string(), "".to_string())
    };

    // Avatar
    let avatar_key = msg_json["avatar"].as_str().unwrap_or("???");
    let (name, emoji) = resolve_avatar(avatars, avatar_key);

    // Text & markdown
    let text = msg_json["text"].as_str().unwrap_or("<no content>").to_string();
    let markdown = msg_json["markdown"].as_str().map(|s| s.to_string());

    // Children
    let children = msg_json["children"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_else(Vec::new);

    Some(MessageInfo {
        date_str,
        time_str,
        name,
        emoji,
        text,
        markdown,
        children,
    })
}
