use std::fs;
use std::path::Path;
use serde_json::{Value, json};
use std::collections::HashMap;
use crate::frs::avatars::resolve_avatar;

pub fn run_status() {
    let fur_dir = Path::new(".fur");
    let index_path = fur_dir.join("index.json");

    if !index_path.exists() {
        eprintln!("🚨 .fur/ not found. Run `fur new` first.");
        return;
    }

    // Load avatars once
    let avatars: Value = serde_json::from_str(
        &fs::read_to_string(fur_dir.join("avatars.json")).unwrap_or_else(|_| "{}".to_string())
    ).unwrap_or(json!({}));

    // Load index + thread
    let (index, thread, mut current_msg_id) = load_index_and_thread(&fur_dir);

    // Preload all messages
    let id_to_message = build_id_to_message(&fur_dir, &thread);

    // Default current message if empty
    if current_msg_id.is_empty() {
        if let Some(first) = thread["messages"].as_array().and_then(|arr| arr.get(0)) {
            if let Some(fid) = first.as_str() {
                current_msg_id = fid.to_string();
            }
        }
    }

    println!("🧠 Current FUR Status");
    println!("─────────────────────────────");
    println!(
        "📌 Active thread: {} ({})",
        thread["title"].as_str().unwrap_or("Untitled"),
        index["active_thread"].as_str().unwrap_or("?")
    );
    println!("🧭 Current message: {}", current_msg_id);
    println!("─────────────────────────────");

    // Print lineage (ancestors)
    print_lineage(&id_to_message, &current_msg_id, &avatars);

    println!("─────────────────────────────");
    println!("Next messages from here:");

    // Print children/siblings
    print_next_messages(&id_to_message, &thread, &current_msg_id, &avatars);
}

/// Load index.json and active thread
fn load_index_and_thread(fur_dir: &Path) -> (Value, Value, String) {
    let index_path = fur_dir.join("index.json");
    let index: Value =
        serde_json::from_str(&fs::read_to_string(&index_path).expect("❌ Cannot read index.json"))
            .unwrap();

    let thread_id = index["active_thread"].as_str().unwrap_or("❓");
    let current_msg_id = index["current_message"].as_str().unwrap_or("").to_string();

    let thread_path = fur_dir.join("threads").join(format!("{}.json", thread_id));
    let thread: Value =
        serde_json::from_str(&fs::read_to_string(&thread_path).expect("❌ Cannot read thread"))
            .unwrap();

    (index, thread, current_msg_id)
}

/// Preload all messages into a HashMap
fn build_id_to_message(fur_dir: &Path, thread: &Value) -> HashMap<String, Value> {
    let mut id_to_message = HashMap::new();
    let mut to_visit: Vec<String> = thread["messages"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|id| id.as_str().map(|s| s.to_string()))
        .collect();

    while let Some(mid) = to_visit.pop() {
        let path = fur_dir.join("messages").join(format!("{}.json", mid));
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(json) = serde_json::from_str::<Value>(&content) {
                // enqueue children + branches
                if let Some(children) = json["children"].as_array() {
                    for c in children {
                        if let Some(cid) = c.as_str() {
                            to_visit.push(cid.to_string());
                        }
                    }
                }
                if let Some(branches) = json["branches"].as_array() {
                    for block in branches {
                        if let Some(arr) = block.as_array() {
                            for c in arr {
                                if let Some(cid) = c.as_str() {
                                    to_visit.push(cid.to_string());
                                }
                            }
                        }
                    }
                }
                id_to_message.insert(mid.clone(), json);
            }
        }
    }
    id_to_message
}

/// Show lineage (ancestors)
fn print_lineage(id_to_message: &HashMap<String, Value>, current_msg_id: &str, avatars: &Value) {
    let mut lineage = vec![];
    let mut current = current_msg_id.to_string();
    while let Some(msg) = id_to_message.get(&current) {
        lineage.push(current.clone());
        match msg["parent"].as_str() {
            Some(parent_id) => current = parent_id.to_string(),
            None => break,
        }
    }
    lineage.reverse();

    for id in &lineage {
        if let Some(msg) = id_to_message.get(id) {
            let avatar_key = msg["avatar"].as_str().unwrap_or("???");
            let (name, emoji) = resolve_avatar(avatars, avatar_key);
            let text = msg.get("text").and_then(|v| v.as_str()).unwrap_or_else(|| {
                msg.get("markdown")
                    .and_then(|v| v.as_str())
                    .unwrap_or("<no content>")
            });

            let preview = text.lines().next().unwrap_or("").chars().take(40).collect::<String>();
            let marker = if *id == current_msg_id { "🧭 (current)" } else { "✅" };
            let id_display = &id[..8];
            let branch_label = compute_branch_label(id, id_to_message);

            if msg.get("markdown").is_some() {
                println!("{preview} {emoji} [{name}] 📄 {id_display} {branch_label} {marker}");
            } else {
                println!("{preview} {emoji} [{name}] {id_display} {branch_label} {marker}");
            }
        }
    }
}

/// Show children/siblings after current
fn print_next_messages(id_to_message: &HashMap<String, Value>, thread: &Value, current_msg_id: &str, avatars: &Value) {
    if let Some(curr_msg) = id_to_message.get(current_msg_id) {
        let mut next_ids: Vec<String> = vec![];

        // children
        if let Some(children) = curr_msg["children"].as_array() {
            next_ids.extend(children.iter().filter_map(|c| c.as_str().map(|s| s.to_string())));
        }

        // siblings
        if next_ids.is_empty() {
            if let Some(parent_id) = curr_msg["parent"].as_str() {
                if let Some(parent) = id_to_message.get(parent_id) {
                    if let Some(branches) = parent["branches"].as_array() {
                        for block in branches {
                            if let Some(arr) = block.as_array() {
                                if let Some(pos) = arr.iter().position(|c| c.as_str() == Some(current_msg_id)) {
                                    for sib in arr.iter().skip(pos + 1) {
                                        if let Some(cid) = sib.as_str() {
                                            next_ids.push(cid.to_string());
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
        if next_ids.is_empty() && curr_msg["parent"].is_null() {
            if let Some(thread_msgs) = thread["messages"].as_array() {
                if let Some(pos) = thread_msgs.iter().position(|id| id.as_str() == Some(current_msg_id)) {
                    for sib in thread_msgs.iter().skip(pos + 1) {
                        if let Some(cid) = sib.as_str() {
                            next_ids.push(cid.to_string());
                        }
                    }
                }
            }
        }

        if next_ids.is_empty() {
            println!("(No further messages in this branch.)");
        } else {
            for child_id in next_ids {
                if let Some(msg) = id_to_message.get(&child_id) {
                    let avatar_key = msg["avatar"].as_str().unwrap_or("???");
                    let (name, emoji) = resolve_avatar(avatars, avatar_key);
                    let text = msg.get("text").and_then(|v| v.as_str()).unwrap_or_else(|| {
                        msg.get("markdown")
                            .and_then(|v| v.as_str())
                            .unwrap_or("<no content>")
                    });

                    let preview = text.lines().next().unwrap_or("").chars().take(40).collect::<String>();
                    let id_display = &child_id[..8];
                    let branch_label = compute_branch_label(&child_id, id_to_message);

                    if msg.get("markdown").is_some() {
                        println!("🔹 {preview} {emoji} [{name}] 📄 {id_display} {branch_label}");
                    } else {
                        println!("🔹 {preview} {emoji} [{name}] {id_display} {branch_label}");
                    }
                }
            }
        }
    } else {
        println!("(No current message found.)");
    }
}

/// Walks backwards from a message to compute its branch path label
fn compute_branch_label(msg_id: &str, id_to_message: &HashMap<String, Value>) -> String {
    // climb up until root, accumulating indices
    let mut labels = vec![];
    let mut current_id = msg_id;

    while let Some(msg) = id_to_message.get(current_id) {
        if let Some(parent_id) = msg["parent"].as_str() {
            if let Some(parent) = id_to_message.get(parent_id) {
                // Check if in branches
                if let Some(branches) = parent["branches"].as_array() {
                    for (b_idx, branch) in branches.iter().enumerate() {
                        if let Some(arr) = branch.as_array() {
                            if arr.iter().any(|c| c.as_str() == Some(current_id)) {
                                labels.push(format!("{}", b_idx + 1));
                            }
                        }
                    }
                }
            }
            current_id = parent_id;
        } else {
            break;
        }
    }

    labels.reverse();
    if labels.is_empty() {
        "[Root]".to_string()
    } else {
        format!("[Branch {}]", labels.join("."))
    }
}
