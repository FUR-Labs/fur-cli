use std::collections::HashSet;
use std::fs;
use std::path::Path;
use serde_json::{Value, json};
use clap::Parser;
use crate::frs::avatars::resolve_avatar;

/// Timeline command structure with verbose flag
#[derive(Parser)]
pub struct TimelineArgs {
    /// Whether to show full content of Markdown files
    #[arg(short, long)]
    pub verbose: bool,
}

pub fn run_timeline(args: TimelineArgs) {
    let fur_dir = Path::new(".fur");
    let index_path = fur_dir.join("index.json");

    if !index_path.exists() {
        eprintln!("🚨 .fur/ not found. Run `fur new` first.");
        return;
    }

    let index_data: Value = serde_json::from_str(
        &fs::read_to_string(&index_path).expect("Cannot read index.json")
    ).unwrap();

    let thread_id = match index_data["active_thread"].as_str() {
        Some(id) => id,
        None => {
            eprintln!("⚠️ No active thread.");
            return;
        }
    };

    let thread_path = fur_dir.join("threads").join(format!("{}.json", thread_id));
    let thread_data: Value = serde_json::from_str(
        &fs::read_to_string(&thread_path).expect("Cannot read thread")
    ).unwrap();

    // load avatars.json once
    let avatars: Value = serde_json::from_str(
        &fs::read_to_string(fur_dir.join("avatars.json")).unwrap_or_else(|_| "{}".to_string())
    ).unwrap_or(json!({}));

    let empty = vec![];
    let messages = thread_data["messages"].as_array().unwrap_or(&empty);

    if messages.is_empty() {
        println!("🕳️ Thread is empty.");
        return;
    }

    println!("🧵 Thread: \"{}\"\n", &thread_data["title"]);

    let mut visited = HashSet::new();

    // Root messages (stem level)
    for msg_id in messages {
        if let Some(id) = msg_id.as_str() {
            render_message(&fur_dir, id, "Root".to_string(), args.verbose, &avatars, &mut visited);
        }
    }
}

/// Recursive renderer (flat, no indentation) with visited-set
fn render_message(
    fur_dir: &Path,
    msg_id: &str,
    label: String,
    verbose: bool,
    avatars: &Value,
    visited: &mut HashSet<String>
) {
    if visited.contains(msg_id) {
        return;
    }
    visited.insert(msg_id.to_string());

    let msg_path = fur_dir.join("messages").join(format!("{}.json", msg_id));
    let msg_content = match fs::read_to_string(&msg_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let msg_json: Value = match serde_json::from_str(&msg_content) {
        Ok(v) => v,
        Err(_) => return,
    };

    let time = msg_json["timestamp"].as_str().unwrap_or("???");
    let avatar_key = msg_json["avatar"].as_str().unwrap_or("???");
    let (name, emoji) = resolve_avatar(avatars, avatar_key);

    // Message text or fallback
    let text = msg_json["text"].as_str().unwrap_or_else(|| {
        if msg_json["markdown"].is_null() {
            "No comment"
        } else {
            "No comment, just a file:"
        }
    });

    println!("🕰️  {} [{}] {} [{}]:", time, label, emoji, name);
    println!("{}\n", text);

    // Markdown linked file
    if let Some(path_str) = msg_json["markdown"].as_str() {
        println!("🔍 Resolving markdown file at: {}", path_str);
        if verbose {
            if let Ok(contents) = fs::read_to_string(path_str) {
                println!("📄 Linked Markdown Content:\n{}", contents);
            } else {
                println!("⚠️ Could not read linked markdown file at: {}", path_str);
            }
        } else {
            println!("📂 Linked Markdown file: {}", path_str);
        }
    }

    // Branch-aware recursion
    if let Some(branches) = msg_json["branches"].as_array() {
        if !branches.is_empty() {
            for (b_idx, branch) in branches.iter().enumerate() {
                if let Some(branch_arr) = branch.as_array() {
                    for child_id in branch_arr {
                        if let Some(c_id) = child_id.as_str() {
                            let new_label = if label == "Root" {
                                format!("Branch {}", b_idx + 1)
                            } else {
                                format!("{}.{}", label.replace("Branch ", ""), b_idx + 1)
                            };
                            render_message(fur_dir, c_id, new_label, verbose, avatars, visited);
                        }
                    }
                }
            }
            return; // ✅ don’t fall back to children if branches exist
        }
    }

    // Legacy fallback: use children if no branches
    if let Some(children) = msg_json["children"].as_array() {
        for child_id in children {
            if let Some(c_id) = child_id.as_str() {
                render_message(fur_dir, c_id, label.clone(), verbose, avatars, visited);
            }
        }
    }
}
