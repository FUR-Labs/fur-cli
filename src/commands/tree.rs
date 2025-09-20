use std::fs;
use std::path::Path;
use serde_json::Value;
use clap::Parser;
use std::collections::HashMap;
use crate::frs::avatars::resolve_avatar;
use colored::*;

#[derive(Parser)]
pub struct TreeArgs {
    #[clap(skip)]
    pub thread_override: Option<String>,
}

pub fn run_tree(_args: TreeArgs) {
    let fur_dir = Path::new(".fur");
    let index_path = fur_dir.join("index.json");

    if !index_path.exists() {
        eprintln!("{}", "🚨 .fur/ not found. Run `fur new` first.".red().bold());
        return;
    }

    // Load index and thread
    let index_data: Value =
        serde_json::from_str(&fs::read_to_string(&index_path).expect("❌ Cannot read index.json"))
            .unwrap();

    let thread_id = if let Some(ref override_id) = args.thread_override {
        override_id
    } else {
        index["active_thread"].as_str().unwrap_or("")
    };
    let thread_path = fur_dir.join("threads").join(format!("{}.json", thread_id));
    let thread_data: Value =
        serde_json::from_str(&fs::read_to_string(&thread_path).expect("❌ Cannot read thread"))
            .unwrap();

    // Load avatars.json once
    let avatars: Value = serde_json::from_str(
        &fs::read_to_string(fur_dir.join("avatars.json")).unwrap_or_else(|_| "{}".to_string())
    ).unwrap_or(serde_json::json!({}));

    println!(
        "{} {}",
        "🌳 Thread Tree:".bold().cyan(),
        thread_data["title"].as_str().unwrap_or("Untitled").green().bold()
    );

    if let Some(messages) = thread_data["messages"].as_array() {
        let id_to_message = build_id_to_message(&fur_dir, &thread_data);
        for (idx, msg_id) in messages.iter().enumerate() {
            if let Some(mid) = msg_id.as_str() {
                render_message(&id_to_message, mid, "", idx == messages.len() - 1, &avatars);
            }
        }
    }
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

/// Recursive tree renderer
fn render_message(
    id_to_message: &HashMap<String, Value>,
    msg_id: &str,
    prefix: &str,
    is_last: bool,
    avatars: &Value,
) {
    if let Some(msg) = id_to_message.get(msg_id) {
        // build tree connector
        let branch_symbol = if is_last { "└──" } else { "├──" };
        let tree_prefix = format!("{}{}", prefix, branch_symbol.bright_green());

        let avatar_key = msg["avatar"].as_str().unwrap_or("???");
        let (name, emoji) = resolve_avatar(avatars, avatar_key);

        let text = msg.get("text").and_then(|v| v.as_str()).unwrap_or_else(|| {
            msg.get("markdown")
                .and_then(|v| v.as_str())
                .unwrap_or("<no content>")
        });

        let id_display = msg_id[..8].to_string();

        if msg.get("markdown").is_some() {
            println!(
                "{} {} {} {} {} {}",
                tree_prefix,
                "[Root]".cyan(),
                emoji.yellow(),
                format!("[{}]", name).bright_yellow(),
                text.white(),
                format!("📄 {}", id_display).magenta()
            );
        } else {
            println!(
                "{} {} {} {} {}",
                tree_prefix,
                "[Root]".cyan(),
                emoji.yellow(),
                format!("[{}]", name).bright_yellow(),
                format!("{} {}", text.white(), id_display.bright_black())
            );
        }

        // Lifetime-safe empty vec
        let empty: Vec<Value> = Vec::new();
        let children = msg["children"].as_array().unwrap_or(&empty);
        let branches = msg["branches"].as_array().unwrap_or(&empty);

        // merge both: if branches exist, prefer them
        if !branches.is_empty() {
            for (_b_idx, branch) in branches.iter().enumerate() {
                if let Some(arr) = branch.as_array() {
                    for (i, child_id) in arr.iter().enumerate() {
                        if let Some(cid) = child_id.as_str() {
                            let new_prefix = format!(
                                "{}{}   ",
                                prefix,
                                if is_last { "    " } else { "│  " }.bright_green()
                            );
                            render_message(id_to_message, cid, &new_prefix, i == arr.len() - 1, avatars);
                        }
                    }
                }
            }
        } else {
            for (i, child_id) in children.iter().enumerate() {
                if let Some(cid) = child_id.as_str() {
                    let new_prefix = format!(
                        "{}{}   ",
                        prefix,
                        if is_last { "    " } else { "│  " }.bright_green()
                    );
                    render_message(id_to_message, cid, &new_prefix, i == children.len() - 1, avatars);
                }
            }
        }
    }
}
