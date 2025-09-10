use std::fs;
use std::path::Path;
use serde_json::{json, Value};
use clap::Parser;
use chrono::Utc;
use uuid::Uuid;

use crate::frs::avatars::{load_avatars, resolve_avatar};

#[derive(Parser)]
pub struct JotArgs {
    /// Optional avatar name (default = main)
    pub avatar: Option<String>,

    /// Jot text
    #[arg(long)]
    pub text: Option<String>,

    /// Attach markdown file
    #[arg(long)]
    pub markdown: Option<String>,

    /// Parent message ID (optional, for replies)
    #[arg(long)]
    pub parent: Option<String>,
}

pub fn run_jot(args: JotArgs) {
    let fur_dir = Path::new(".fur");
    if !fur_dir.exists() {
        eprintln!("🚨 .fur/ not found. Run `fur new` first.");
        return;
    }

    // Load avatars
    let avatars = load_avatars();

    // Pick avatar name (prefer explicit, else main, else unknown)
    let avatar_name = args.avatar.unwrap_or_else(|| {
        avatars
            .get("main")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string()
    });

    // Create new message JSON
    let message_id = Uuid::new_v4().to_string();
    let timestamp = Utc::now().to_rfc3339();

    let mut message = json!({
        "id": message_id,
        "avatar": avatar_name,    // ✅ store name only
        "timestamp": timestamp,
        "children": [],
        "branches": [],
    });

    if let Some(text) = args.text {
        message["text"] = json!(text);
    }
    if let Some(md) = args.markdown {
        message["markdown"] = json!(md);
    }
    if let Some(ref parent_id) = args.parent {
        message["parent"] = json!(parent_id);
    }

    // Save message to messages/
    let msg_path = fur_dir.join("messages").join(format!("{}.json", message_id));
    if let Ok(serialized) = serde_json::to_string_pretty(&message) {
        let _ = fs::write(msg_path, serialized);
    }

    // Update thread index
    let index_path = fur_dir.join("index.json");
    let mut index_data: Value = serde_json::from_str(
        &fs::read_to_string(&index_path).expect("Cannot read index.json"),
    )
    .unwrap();

    let thread_id = index_data["active_thread"]
        .as_str()
        .unwrap()
        .to_string(); // ✅ clone to break borrow

    let thread_path = fur_dir.join("threads").join(format!("{}.json", thread_id));
    let mut thread_data: Value =
        serde_json::from_str(&fs::read_to_string(&thread_path).expect("Cannot read thread"))
            .unwrap();

    // Insert message into correct place
    if let Some(ref parent_id) = args.parent {
        // Load parent and attach child
        let parent_path = fur_dir.join("messages").join(format!("{}.json", parent_id));
        if let Ok(content) = fs::read_to_string(&parent_path) {
            if let Ok(mut parent_json) = serde_json::from_str::<Value>(&content) {
                if let Some(children) = parent_json["children"].as_array_mut() {
                    children.push(json!(message_id));
                } else {
                    parent_json["children"] = json!([message_id]);
                }

                // Save updated parent
                if let Ok(serialized) = serde_json::to_string_pretty(&parent_json) {
                    let _ = fs::write(parent_path, serialized);
                }
            }
        }
    } else {
        // Root-level message → append only once
        if let Some(arr) = thread_data["messages"].as_array_mut() {
            arr.push(json!(message_id));
        }
    }

    // Save updated thread
    if let Ok(serialized) = serde_json::to_string_pretty(&thread_data) {
        let _ = fs::write(thread_path, serialized);
    }

    // Update index current message
    index_data["current_message"] = json!(message_id);
    if let Ok(serialized) = serde_json::to_string_pretty(&index_data) {
        let _ = fs::write(index_path, serialized);
    }

    // Resolve emoji for display
    let (_, emoji) = resolve_avatar(&avatars, &avatar_name);

    println!(
        "✍️ Message jotted down to thread {}: {} [{}] {}",
        thread_id,
        &message_id[..8],
        avatar_name,
        emoji
    );
}
