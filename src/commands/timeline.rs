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

    for msg_id in messages {
        let msg_id_str = msg_id.as_str().unwrap();
        let msg_path = fur_dir.join("messages").join(format!("{}.json", msg_id_str));
        if let Ok(msg_content) = fs::read_to_string(&msg_path) {
            if let Ok(msg_json) = serde_json::from_str::<Value>(&msg_content) {
                let time = msg_json["timestamp"].as_str().unwrap_or("???");
                let role = msg_json["role"].as_str().unwrap_or("unknown");

                // Handle missing text with a fallback message
                let text = msg_json["text"].as_str().unwrap_or_else(|| {
                    if msg_json["markdown"].is_null() {
                        "No comment"
                    } else {
                        "No comment, just a file:"
                    }
                });


                println!("🕰️  {} [{}]:\n{}\n", time, role, text);

                // Check for markdown reference
                if let Some(path_str) = msg_json["markdown"].as_str() {
                    println!("🔍 Resolving markdown file at: {}", path_str);

                    // Check verbose flag to decide whether to show full content or just the file path
                    if args.verbose {
                        if let Ok(contents) = fs::read_to_string(path_str) {
                            println!("📄 Linked Markdown Content:\n{}", contents);
                        } else {
                            // If file can't be read, show the path
                            println!("⚠️ Could not read linked markdown file at: {}", path_str);
                        }
                    } else {
                        // Just display the file path
                        println!("📂 Linked Markdown file: {}", path_str);
                    }
                }
            }
        }
    }
}
