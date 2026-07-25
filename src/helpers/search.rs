use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

pub fn search_messages_in_conversation(
    msg_ids: &[String],
    messages_dir: &Path,
    queries: &[String],
) -> Vec<Value> {
    let mut matches = Vec::new();

    for mid in msg_ids {
        let msg_path = messages_dir.join(format!("{}.json", mid));
        let content = match fs::read_to_string(&msg_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let msg: Value = match serde_json::from_str(&content) {
            Ok(j) => j,
            Err(_) => continue,
        };

        let avatar = msg["avatar"].as_str().unwrap_or("unknown").to_string();

        if let Some(hit) = search_text_field(&msg, mid, &avatar, queries) {
            matches.push(hit);
            continue;
        }

        if let Some(hit) = search_markdown_field(&msg, mid, &avatar, queries) {
            matches.push(hit);
        }
    }

    matches
}

pub fn search_text_field(
    msg: &Value,
    mid: &str,
    avatar: &str,
    queries: &[String],
) -> Option<Value> {
    let text = msg["text"].as_str()?;

    if let Some((q, snippet)) = match_any_query(text, queries) {
        return Some(json!({
            "message_id": mid,
            "avatar": avatar,
            "source": "text",
            "query": q,
            "snippet": snippet
        }));
    }

    None
}

pub fn search_markdown_field(
    msg: &Value,
    mid: &str,
    avatar: &str,
    queries: &[String],
) -> Option<Value> {
    let md_path_raw = msg["markdown"].as_str()?;

    if let Some((q, snippet)) = search_markdown(md_path_raw, queries) {
        return Some(json!({
            "message_id": mid,
            "avatar": avatar,
            "source": "markdown",
            "query": q,
            "snippet": snippet
        }));
    }

    None
}

/// Parse a search string into individual queries.
/// Supports:
///   "deep learning"
///   "deep learning, neural models"
///   "deep, learning"
pub fn parse_queries(q: &str) -> Vec<String> {
    let lowered = q.trim().to_lowercase();

    // Case 1: contains commas → split
    if lowered.contains(',') {
        return lowered
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }

    // Case 2: no comma → single query
    vec![lowered]
}

/// Load all conversation JSONs in threads/
pub fn list_conversations(threads_dir: &Path) -> Vec<(String, Value)> {
    let mut out = Vec::new();

    for entry in fs::read_dir(threads_dir).unwrap_or_else(|_| panic!("Cannot read threads dir")) {
        if let Ok(e) = entry {
            let path = e.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(json) = serde_json::from_str::<Value>(&content) {
                            out.push((stem.to_string(), json));
                        }
                    }
                }
            }
        }
    }

    out
}

/// Search plain text for any query. Returns (query, snippet)
pub fn match_any_query(text: &str, queries: &[String]) -> Option<(String, String)> {
    let lower = text.to_lowercase();

    for q in queries {
        if let Some(pos) = lower.find(q) {
            return Some((q.clone(), snippet_around(text, pos, q.len())));
        }
    }

    None
}

/// Search markdown content safely; returns Some((query, snippet)) or None
pub fn search_markdown(md_path_raw: &str, queries: &[String]) -> Option<(String, String)> {
    let md_path = PathBuf::from(md_path_raw);
    let path = if md_path.is_absolute() {
        md_path
    } else {
        PathBuf::from(".").join(md_path_raw)
    };

    let content = fs::read_to_string(&path).ok()?;
    match_any_query(&content, queries)
}

/// Extract a snippet of ±40 characters around the match
pub fn snippet_around(text: &str, pos: usize, len: usize) -> String {
    let start = pos.saturating_sub(40);
    let end = pos + len + 40;

    // Try a cheap, fast byte slice first (safe)
    if let Some(s) = text.get(start..end) {
        return s.replace('\n', " ");
    }

    // If that failed, fall back to UTF-8 safe slicing
    let chars: Vec<char> = text.chars().collect();
    let total = chars.len();

    let cpos = text[..pos].chars().count(); // convert byte offset → char index
    let clen = text[pos..pos + len].chars().count();

    let cstart = cpos.saturating_sub(40);
    let cend = (cpos + clen + 40).min(total);

    chars[cstart..cend]
        .iter()
        .collect::<String>()
        .replace('\n', " ")
}
