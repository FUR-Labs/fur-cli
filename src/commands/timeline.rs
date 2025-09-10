use std::fs;
use std::path::Path;
use serde_json::Value;
use clap::Parser;

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

    let empty = vec![];
    let messages = thread_data["messages"].as_array().unwrap_or(&empty);

    if messages.is_empty() {
        println!("🕳️ Thread is empty.");
        return;
    }

    println!("🧵 Thread: \"{}\"\n", &thread_data["title"]);

    // Root messages (stem level)
    for msg_id in messages {
        if let Some(id) = msg_id.as_str() {
            render_message(&fur_dir, id, "Root".to_string(), 0, args.verbose);
        }
    }
}

/// Recursive renderer
fn render_message(fur_dir: &Path, msg_id: &str, label: String, depth: usize, verbose: bool) {
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
    let avatar = msg_json["avatar"].as_str().unwrap_or("🐾");
    let name = msg_json["name"].as_str().unwrap_or("unknown");

    // Message text or fallback
    let text = msg_json["text"].as_str().unwrap_or_else(|| {
        if msg_json["markdown"].is_null() {
            "No comment"
        } else {
            "No comment, just a file:"
        }
    });

    let indent = "    ".repeat(depth);
    println!(
        "{}🕰️  {} [{}] {} [{}]:\n{}{}\n",
        indent,
        time,
        label,
        avatar,
        name,
        indent,
        text
    );

    // Markdown linked file
    if let Some(path_str) = msg_json["markdown"].as_str() {
        println!("{}🔍 Resolving markdown file at: {}", indent, path_str);
        if verbose {
            if let Ok(contents) = fs::read_to_string(path_str) {
                println!("{}📄 Linked Markdown Content:\n{}", indent, contents);
            } else {
                println!("{}⚠️ Could not read linked markdown file at: {}", indent, path_str);
            }
        } else {
            println!("{}📂 Linked Markdown file: {}", indent, path_str);
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
                                // First-level branches
                                format!("Branch {}", b_idx + 1)
                            } else {
                                // Nested branches
                                format!("{}.{}", label.replace("Branch ", ""), b_idx + 1)
                            };
                            render_message(fur_dir, c_id, new_label, depth + 1, verbose);
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
                render_message(fur_dir, c_id, label.clone(), depth + 1, verbose);
            }
        }
    }
}
