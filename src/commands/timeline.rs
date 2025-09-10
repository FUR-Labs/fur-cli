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

    println!("🧵 Thread: {}\n", &thread_data["title"]);

    // Enumerate top-level messages as Root 1..N
    for msg_id in messages {
        if let Some(id_str) = msg_id.as_str() {
            let path = vec![]; // start empty
            print_message_recursive(&fur_dir, id_str, 0, &path, &args);
        }
    }
}

/// Pretty-join a path like [1,2,3] -> "1.2.3"
fn path_str(path: &[usize]) -> String {
    path.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(".")
}

/// Recursively print a message and its children, with Root/Branch notation
fn print_message_recursive(
    fur_dir: &Path,
    msg_id: &str,
    depth: usize,
    path: &[usize],
    args: &TimelineArgs,
) {
    let msg_path = fur_dir.join("messages").join(format!("{}.json", msg_id));
    if let Ok(msg_content) = fs::read_to_string(&msg_path) {
        if let Ok(msg_json) = serde_json::from_str::<Value>(&msg_content) {
            let time = msg_json["timestamp"].as_str().unwrap_or("???");
            let avatar = msg_json["avatar"].as_str().unwrap_or("🐾");
            let name = msg_json["name"].as_str().unwrap_or("unknown");

            // Handle missing text with a fallback message
            let text = msg_json["text"].as_str().unwrap_or_else(|| {
                if msg_json["markdown"].is_null() {
                    "No comment"
                } else {
                    "No comment, just a file:"
                }
            });

            let indent = "    ".repeat(depth);
            let label = if depth == 0 {
                "[Root]".to_string()
            } else {
                format!("[Branch {}]", path_str(path))
            };

            println!(
                "{indent}🕰️  {time} {label} {avatar} [{name}]:\n{indent}{text}\n"
            );

            // Markdown reference if present
            if let Some(path_str) = msg_json["markdown"].as_str() {
                println!("{indent}🔍 Resolving markdown file at: {path_str}");
                if args.verbose {
                    if let Ok(contents) = fs::read_to_string(path_str) {
                        println!("{indent}📄 Linked Markdown Content:\n{contents}");
                    } else {
                        println!("{indent}⚠️ Could not read linked markdown file at: {path_str}");
                    }
                } else {
                    println!("{indent}📂 Linked Markdown file: {path_str}");
                }
            }

            // Recurse into children, numbering them
            if let Some(children) = msg_json["children"].as_array() {
                for (i, child_id) in children.iter().enumerate() {
                    if let Some(child_str) = child_id.as_str() {
                        let mut child_path = path.to_vec();
                        child_path.push(i + 1); // 1-based numbering
                        print_message_recursive(fur_dir, child_str, depth + 1, &child_path, args);
                    }
                }
            }
        }
    }
}
