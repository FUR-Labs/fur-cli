use std::fs;
use std::path::Path;
use serde_json::{json, Value};
use clap::Parser;
use chrono::Utc;
use uuid::Uuid;

use crate::frs::avatars::{load_avatars, resolve_avatar};

#[derive(Parser, Debug)]
pub struct JotArgs {
    /// Optional avatar name (defaults to 'main' if omitted)
    #[arg(index = 1)]
    pub avatar: Option<String>,

    /// Optional jot text
    #[arg(index = 2)]
    pub positional_text: Option<String>,

    /// Jot text (takes precedence over positional)
    #[arg(long)]
    pub text: Option<String>,

    /// Attach markdown file
    #[arg(long, alias = "file")]
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
    let avatar_map = avatars
        .as_object()
        .expect("avatars.json must be a valid JSON object");

    // Determine avatar + text
    let (avatar_name, jot_text) = match (&args.avatar, &args.positional_text) {
        (Some(a), Some(t)) => (a.clone(), Some(t.clone())),
        (Some(a), None) => {
            if avatar_map.contains_key(a) {
                (a.clone(), args.text.clone())
            } else {
                let default_avatar = avatar_map
                    .get("main")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                (default_avatar, Some(a.clone()))
            }
        }
        (None, Some(t)) => {
            let default_avatar = avatar_map
                .get("main")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            (default_avatar, Some(t.clone()))
        }
        (None, None) => {
            let default_avatar = avatar_map
                .get("main")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            (default_avatar, args.text.clone())
        }
    };

    // Final text resolution (flag overrides positional)
    let final_text = args.text.clone().or(jot_text);


    // Optionally: prevent empty jots
    if final_text.is_none() && args.markdown.is_none() {
        eprintln!("🛑 You must provide either text or a markdown file.");
        return;
    }

    let message_id = Uuid::new_v4().to_string();
    let timestamp = Utc::now().to_rfc3339();

    let mut message = json!({
        "id": message_id,
        "avatar": avatar_name,
        "timestamp": timestamp,
        "children": [],
        "branches": [],
    });

    if let Some(text) = final_text {
        message["text"] = json!(text);
    }
    if let Some(md) = args.markdown {
        message["markdown"] = json!(md);
    }
    if let Some(ref parent_id) = args.parent {
        message["parent"] = json!(parent_id);
    }

    // Save message
    let msg_path = fur_dir.join("messages").join(format!("{}.json", message_id));
    if let Ok(serialized) = serde_json::to_string_pretty(&message) {
        let _ = fs::write(msg_path, serialized);
    }

    // Load index
    let index_path = fur_dir.join("index.json");
    let mut index_data: Value = serde_json::from_str(
        &fs::read_to_string(&index_path).expect("Cannot read index.json"),
    )
    .unwrap();

    let thread_id = index_data["active_thread"]
        .as_str()
        .unwrap()
        .to_string();

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
