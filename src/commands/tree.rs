use std::fs;
use std::path::Path;
use serde_json::Value;
use crate::frs::avatars::load_avatars;

/// Run the tree command
pub fn run_tree() {
    let fur_dir = Path::new(".fur");
    let avatars = load_avatars();

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

    let empty = vec![];
    let messages = thread_data["messages"].as_array().unwrap_or(&empty);

    if messages.is_empty() {
        println!("🕳️ Thread is empty.");
        return;
    }

    println!("🌳 Thread Tree: \"{}\"\n", &thread_data["title"]);

    for (idx, msg_id) in messages.iter().enumerate() {
        if let Some(id) = msg_id.as_str() {
            let is_last = idx == messages.len() - 1;
            render_message(&fur_dir, id, "Root".to_string(), "".to_string(), is_last, &avatars);
        }
    }
}



/// Reads a message JSON from disk
fn load_message(fur_dir: &Path, msg_id: &str) -> Option<Value> {
    let msg_path = fur_dir.join("messages").join(format!("{}.json", msg_id));
    let msg_content = fs::read_to_string(&msg_path).ok()?;
    serde_json::from_str(&msg_content).ok()
}

/// Prints a single message line (tree connector + emoji + text)
fn print_message_line(msg_json: &Value, label: &str, prefix: &str, is_last: bool, avatars: &Value) {
    let name = msg_json["name"].as_str().unwrap_or("unknown");
    let emoji = avatars
        .get(name)
        .and_then(|v| v.as_str())
        .unwrap_or("🐾");
    let text = msg_json["text"].as_str().unwrap_or_else(|| {
        if msg_json["markdown"].is_null() {
            "No comment"
        } else {
            "📄 file"
        }
    });

    let connector = if is_last { "└──" } else { "├──" };
    println!("{}{} [{}] {} [{}]: {}", prefix, connector, label, emoji, name, text);
}


/// Recursive renderer
pub fn render_message(
    fur_dir: &Path,
    msg_id: &str,
    label: String,
    prefix: String,
    is_last: bool,
    avatars: &Value,
) {
    let msg_json = match load_message(fur_dir, msg_id) {
        Some(v) => v,
        None => return,
    };

    // Print this node
    print_message_line(&msg_json, &label, &prefix, is_last, avatars);

    // Build new prefix for children
    let new_prefix = if is_last {
        format!("{}    ", prefix)
    } else {
        format!("{}│   ", prefix)
    };

    // Prefer branch recursion if present
    if let Some(branches) = msg_json["branches"].as_array() {
        for (b_idx, branch) in branches.iter().enumerate() {
            if let Some(branch_arr) = branch.as_array() {
                for (c_idx, child_id) in branch_arr.iter().enumerate() {
                    if let Some(c_id) = child_id.as_str() {
                        let child_label = if label == "Root" {
                            format!("Branch {}", b_idx + 1)
                        } else {
                            format!("{}.{}", label.replace("Branch ", ""), b_idx + 1)
                        };
                        let child_last = c_idx == branch_arr.len() - 1;
                        render_message(
                            fur_dir,
                            c_id,
                            child_label,
                            new_prefix.clone(),
                            child_last,
                            avatars,
                        );
                    }
                }
            }
        }
        return; // ✅ don’t fall back to children if branches exist
    }

    // Fallback: legacy `children`
    if let Some(children) = msg_json["children"].as_array() {
        for (idx, child_id) in children.iter().enumerate() {
            if let Some(c_id) = child_id.as_str() {
                let child_last = idx == children.len() - 1;
                render_message(
                    fur_dir,
                    c_id,
                    label.clone(),
                    new_prefix.clone(),
                    child_last,
                    avatars,
                );
            }
        }
    }
}
