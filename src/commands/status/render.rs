use colored::*;
use serde_json::Value;
use std::collections::HashMap;
use crate::frs::avatars::resolve_avatar;

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

/// Print children + siblings
pub fn print_next_messages(
    map: &HashMap<String, Value>,
    thread: &Value,
    current: &str,
    avatars: &Value
) {
    let Some(curr_msg) = map.get(current) else {
        println!("{}", "(No current message found.)".red());
        return;
    };

    let mut next = vec![];

    // direct children
    if let Some(children) = curr_msg["children"].as_array() {
        next.extend(
            children.iter()
                .filter_map(|c| c.as_str().map(|s| s.to_string()))
        );
    }

    // siblings if no children
    if next.is_empty() {
        if let Some(pid) = curr_msg["parent"].as_str() {
            if let Some(parent) = map.get(pid) {
                if let Some(blocks) = parent["branches"].as_array() {
                    for block in blocks {
                        if let Some(arr) = block.as_array() {
                            if let Some(pos) = arr.iter().position(|x| x.as_str() == Some(current)) {
                                for sib in arr.iter().skip(pos + 1) {
                                    if let Some(cid) = sib.as_str() {
                                        next.push(cid.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // top-level siblings
    if next.is_empty() && curr_msg["parent"].is_null() {
        if let Some(arr) = thread["messages"].as_array() {
            if let Some(pos) = arr.iter().position(|v| v.as_str() == Some(current)) {
                for sib in arr.iter().skip(pos + 1) {
                    if let Some(cid) = sib.as_str() {
                        next.push(cid.to_string());
                    }
                }
            }
        }
    }

    if next.is_empty() {
        println!("{}", "(No further messages in this branch.)".bright_black());
        return;
    }

    // render
    for cid in next {
        if let Some(msg) = map.get(&cid) {
            let avatar_key = msg["avatar"].as_str().unwrap_or("???");
            let (name, emoji) = resolve_avatar(avatars, avatar_key);

            let text = msg.get("text")
                .and_then(|v| v.as_str())
                .or_else(|| msg["markdown"].as_str())
                .unwrap_or("<no content>");

            let preview = text.lines().next().unwrap_or("")
                .chars().take(40).collect::<String>();

            let branch_label = compute_branch_label(&cid, map);

            println!(
                "🔹 {} {} {} {} {}",
                preview.white(),
                emoji,
                format!("[{}]", name).bright_yellow().bold(),
                &cid[..8].bright_black(),
                branch_label.bright_green()
            );
        }
    }
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
