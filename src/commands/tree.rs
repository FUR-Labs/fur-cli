use std::collections::HashSet;
use std::fs;
use std::path::Path;
use serde_json::{Value, json};
use clap::Parser;

use crate::frs::avatars::{resolve_avatar};

#[derive(Parser)]
pub struct TreeArgs {}

pub fn run_tree(_args: TreeArgs) {
    let fur_dir = Path::new(".fur");
    let index_path = fur_dir.join("index.json");

    if !index_path.exists() {
        eprintln!("🚨 .fur/ not found. Run `fur new` first.");
        return;
    }

    // Load index + thread
    let index_data: Value = serde_json::from_str(
        &fs::read_to_string(&index_path).expect("Cannot read index.json")
    ).unwrap();

    let thread_id = index_data["active_thread"].as_str().unwrap();
    let thread_path = fur_dir.join("threads").join(format!("{}.json", thread_id));
    let thread_data: Value = serde_json::from_str(
        &fs::read_to_string(&thread_path).expect("Cannot read thread")
    ).unwrap();

    // Load avatars
    let avatars: Value = serde_json::from_str(
        &fs::read_to_string(fur_dir.join("avatars.json")).unwrap_or_else(|_| "{}".to_string())
    ).unwrap_or(json!({}));

    println!("🌳 Thread Tree: \"{}\"\n", &thread_data["title"]);

    // Root messages
    let empty = vec![];
    let messages = thread_data["messages"].as_array().unwrap_or(&empty);

    let mut visited = HashSet::new();

    for msg_id in messages {
        if let Some(id) = msg_id.as_str() {
            render_message(&fur_dir, id, "Root".to_string(), "", &avatars, &mut visited);
        }
    }
}

/// Recursive renderer with visited-set
fn render_message(
    fur_dir: &Path,
    msg_id: &str,
    label: String,
    prefix: &str,
    avatars: &Value,
    visited: &mut HashSet<String>
) {
    // Skip if already visited
    if visited.contains(msg_id) {
        return;
    }
    visited.insert(msg_id.to_string());

    // Load message
    let msg_path = fur_dir.join("messages").join(format!("{}.json", msg_id));
    let msg_content = match fs::read_to_string(&msg_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let msg_json: Value = match serde_json::from_str(&msg_content) {
        Ok(v) => v,
        Err(_) => return,
    };

    let text = msg_json["text"].as_str().unwrap_or_else(|| {
        if msg_json["markdown"].is_null() {
            "<no content>"
        } else {
            "📄 file"
        }
    });

    let avatar_key = msg_json["avatar"].as_str().unwrap_or("???");
    let (name, emoji) = resolve_avatar(avatars, avatar_key);

    println!("{}├── [{}] {} [{}]: {}", prefix, label, emoji, name, text);

    // Prepare deeper prefix
    let new_prefix = format!("{}│   ", prefix);

    // Branch-aware recursion
    if let Some(branches) = msg_json["branches"].as_array() {
        if !branches.is_empty() {
            for (b_idx, branch) in branches.iter().enumerate() {
                if let Some(branch_arr) = branch.as_array() {
                    for child_id in branch_arr {
                        if let Some(c_id) = child_id.as_str() {
                            let new_label = format!("{}.{}", label.replace("Branch ", ""), b_idx + 1);
                            render_message(fur_dir, c_id, new_label, &new_prefix, avatars, visited);
                        }
                    }
                }
            }
            return; // ✅ don’t fall back to children if branches exist
        }
    }

    // Legacy children
    if let Some(children) = msg_json["children"].as_array() {
        for child_id in children {
            if let Some(c_id) = child_id.as_str() {
                render_message(fur_dir, c_id, label.clone(), &new_prefix, avatars, visited);
            }
        }
    }
}
