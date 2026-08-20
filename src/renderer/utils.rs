use chrono::{DateTime, FixedOffset, Local};
use serde_json::Value;
use std::fs;
use std::path::Path;

use crate::avatars::resolve_avatar;

#[allow(dead_code)]
pub struct MessageInfo {
    pub date_str: String,
    pub time_str: String,
    pub name: String,
    pub emoji: String,
    pub text: String,
    pub markdown: Option<String>,
    pub attachment: Option<String>,
    #[allow(dead_code)]
    pub children: Vec<String>,
    pub branches: Vec<Vec<String>>,
}

/// One-line preview of a message, for trees and status views.
///
/// viceroy: previews used to fall back to the *path* of an attachment, so a
/// long-form message rendered as `chats/slug/CHAT-2026...` instead of its
/// content. The linked document's first meaningful line is used now.
pub fn preview_of(msg: &Value, width: usize) -> String {
    if let Some(text) = msg.get("text").and_then(|v| v.as_str()) {
        if !text.trim().is_empty() {
            return clip(text, width);
        }
    }

    if let Some(path) = msg.get("markdown").and_then(|v| v.as_str()) {
        if let Some(line) = first_meaningful_line(path) {
            return clip(&line, width);
        }

        // Nothing readable — name the file rather than printing a whole path.
        let name = Path::new(path)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(path);

        return format!("📄 {}", clip(name, width.saturating_sub(2)));
    }

    "<no content>".to_string()
}

/// Skip front matter, comment markers and heading punctuation to find
/// something worth showing.
fn first_meaningful_line(path: &str) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;

    let mut lines = content.lines().peekable();

    if lines.peek().map(|l| l.trim_end()) == Some("---") {
        lines.next();
        for line in lines.by_ref() {
            if line.trim_end() == "---" {
                break;
            }
        }
    }

    for line in lines {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with("<!--") {
            continue;
        }

        let cleaned = trimmed.trim_start_matches('#').trim();

        if !cleaned.is_empty() {
            return Some(cleaned.to_string());
        }
    }

    None
}

fn clip(text: &str, width: usize) -> String {
    let line = text.lines().next().unwrap_or("").trim();

    if line.chars().count() <= width {
        return line.to_string();
    }

    let kept: String = line.chars().take(width.saturating_sub(1)).collect();

    format!("{}…", kept.trim_end())
}

pub fn load_message(fur_dir: &Path, msg_id: &str, avatars: &Value) -> Option<MessageInfo> {
    let msg_path = fur_dir.join("messages").join(format!("{}.json", msg_id));
    let msg_content = fs::read_to_string(&msg_path).ok()?;
    let msg_json: Value = serde_json::from_str(&msg_content).ok()?;

    // Timestamp
    let raw_time = msg_json["timestamp"].as_str().unwrap_or("???");
    let (date_str, time_str) = if let Ok(dt) = raw_time.parse::<DateTime<FixedOffset>>() {
        let local_dt = dt.with_timezone(&Local);
        (
            local_dt.format("%Y-%m-%d").to_string(),
            local_dt.format("%H:%M:%S").to_string(),
        )
    } else {
        (raw_time.to_string(), "".to_string())
    };

    // Avatar
    let avatar_key = msg_json["avatar"].as_str().unwrap_or("???");
    let (name, emoji) = resolve_avatar(avatars, avatar_key);

    // Text & markdown
    let text = msg_json["text"]
        .as_str()
        .unwrap_or("<no content>")
        .to_string();
    let markdown = msg_json["markdown"].as_str().map(|s| s.to_string());

    // Children
    let children = msg_json["children"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_else(Vec::new);

    // Branches
    let branches = msg_json["branches"]
        .as_array()
        .map(|outer| {
            outer
                .iter()
                .filter_map(|block| {
                    block.as_array().map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect::<Vec<_>>()
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(Vec::new);

    let attachment = msg_json["attachment"].as_str().map(|s| s.to_string());

    Some(MessageInfo {
        date_str,
        time_str,
        name,
        emoji,
        text,
        markdown,
        attachment,
        children,
        branches,
    })
}