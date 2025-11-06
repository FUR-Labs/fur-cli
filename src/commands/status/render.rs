use colored::*;
use serde_json::Value;
use std::collections::HashMap;
use crate::frs::avatars::resolve_avatar;

type Map = HashMap<String, Value>;

/// Print active thread title
pub fn print_active_thread(index: &Value) {
    println!(
        "{} {} {}",
        "Active thread:".bright_cyan().bold(),
        index["title"]
            .as_str()
            .unwrap_or("Untitled")
            .bright_green().bold(),
        format!("({})", index["active_thread"].as_str().unwrap_or("?"))
            .bright_black()
    );
}

/// Print current message line
pub fn print_current_message(current_msg_id: &str) {
    println!(
        "{} {}",
        "Current message:".bright_cyan().bold(),
        current_msg_id.bright_black()
    );
}

/// Print lineage (ancestors)
pub fn print_lineage(
    map: &HashMap<String, Value>,
    current: &str,
    avatars: &Value
) {
    let mut chain = vec![];
    let mut cur = current.to_string();

    while let Some(msg) = map.get(&cur) {
        chain.push(cur.clone());
        match msg["parent"].as_str() {
            Some(pid) => cur = pid.to_string(),
            None => break,
        }
    }

    chain.reverse();

    for mid in chain {
        if let Some(msg) = map.get(&mid) {
            let avatar_key = msg["avatar"].as_str().unwrap_or("???");
            let (name, emoji) = resolve_avatar(avatars, avatar_key);

            let text = msg.get("text")
                .and_then(|v| v.as_str())
                .or_else(|| msg["markdown"].as_str())
                .unwrap_or("<no content>");

            let preview = text.lines().next().unwrap_or("")
                .chars().take(40).collect::<String>();

            let marker = if mid == current {
                "(current)".cyan().bold()
            } else {
                "✅".green()
            };

            let branch_label = compute_branch_label(&mid, map);

            println!(
                "{} {} {} {} {} {}",
                preview.white(),
                emoji,
                format!("[{}]", name).bright_yellow().bold(),
                &mid[..8].bright_black(),
                branch_label.bright_green(),
                marker
            );
        }
    }
}

pub fn print_next_messages(
    map: &Map,
    thread: &Value,
    current: &str,
    avatars: &Value,
) {
    let curr_msg = match map.get(current) {
        Some(v) => v,
        None => return println!("{}", "(No current message found.)".red()),
    };

    let next = get_children(curr_msg)
        .or_else(|| get_sibling_branch(map, curr_msg, current))
        .or_else(|| get_top_level_siblings(thread, current))
        .unwrap_or_default();

    if next.is_empty() {
        println!("{}", "(No further messages in this branch.)".bright_black());
        return;
    }

    for cid in next {
        if let Some(msg) = map.get(&cid) {
            render_preview(msg, avatars, &cid, map);
        }
    }
}


fn get_children(curr_msg: &Value) -> Option<Vec<String>> {
    let arr = curr_msg["children"].as_array()?;
    let v = arr
        .iter()
        .filter_map(|c| c.as_str().map(|s| s.to_string()))
        .collect::<Vec<_>>();

    if v.is_empty() { None } else { Some(v) }
}

fn get_sibling_branch(map: &Map, curr_msg: &Value, current: &str) -> Option<Vec<String>> {
    let parent_id = curr_msg["parent"].as_str()?;
    let parent = map.get(parent_id)?;

    let blocks = parent["branches"].as_array()?;
    for block in blocks {
        if let Some(arr) = block.as_array() {
            if let Some(pos) = arr.iter().position(|v| v.as_str() == Some(current)) {
                let sibs = arr.iter()
                    .skip(pos + 1)
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>();
                if !sibs.is_empty() {
                    return Some(sibs);
                }
            }
        }
    }

    None
}

fn get_top_level_siblings(thread: &Value, current: &str) -> Option<Vec<String>> {
    let arr = thread["messages"].as_array()?;

    let pos = arr.iter().position(|v| v.as_str() == Some(current))?;
    let v = arr.iter()
        .skip(pos + 1)
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect::<Vec<_>>();

    if v.is_empty() { None } else { Some(v) }
}


fn render_preview(msg: &Value, avatars: &Value, cid: &str, map: &Map) {
    let avatar_key = msg["avatar"].as_str().unwrap_or("???");
    let (name, emoji) = resolve_avatar(avatars, avatar_key);

    let text = msg.get("text")
        .and_then(|v| v.as_str())
        .or_else(|| msg["markdown"].as_str())
        .unwrap_or("<no content>");

    let preview = text.lines().next().unwrap_or("")
        .chars().take(40).collect::<String>();

    let branch_label = compute_branch_label(cid, map);

    println!(
        "🔹 {} {} {} {} {}",
        preview.white(),
        emoji,
        format!("[{}]", name).bright_yellow().bold(),
        &cid[..8].bright_black(),
        branch_label.bright_green(),
    );
}


/// Compute branch label
pub fn compute_branch_label(
    msg_id: &str,
    map: &HashMap<String, Value>
) -> String {
    let mut labels = vec![];
    let mut cur = msg_id;

    while let Some(msg) = map.get(cur) {
        if let Some(pid) = msg["parent"].as_str() {
            if let Some(parent) = map.get(pid) {
                if let Some(blocks) = parent["branches"].as_array() {
                    for (i, block) in blocks.iter().enumerate() {
                        if let Some(arr) = block.as_array() {
                            if arr.iter().any(|v| v.as_str() == Some(cur)) {
                                labels.push(format!("{}", i + 1));
                            }
                        }
                    }
                }
                cur = pid;
                continue;
            }
        }
        break;
    }

    labels.reverse();
    if labels.is_empty() {
        "[Root]".to_string()
    } else {
        format!("[Branch {}]", labels.join("."))
    }
}
