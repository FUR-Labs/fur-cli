use std::fs;
use std::path::Path;
use serde_json::{json, Value};
use clap::Parser;

use crate::schema::make_message_metadata;
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

    /// Attach image (PNG, JPG, etc.)
    #[arg(long)]
    pub img: Option<String>,

    /// Parent message ID (optional, for replies)
    #[arg(long)]
    pub parent: Option<String>,
}


fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("Cannot read JSON")).unwrap()
}

fn write_json(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_string_pretty(value).unwrap()).unwrap();
}

pub fn run_jot(args: JotArgs) {
    let fur_dir = Path::new(".fur");
    if !fur_dir.exists() {
        eprintln!("🚨 .fur/ not found. Run `fur new` first.");
        return;
    }

    // Load avatars
    let avatars = load_avatars();

    // Resolve avatar and text
    let (avatar_name, jot_text) = resolve_avatar_and_text(&avatars, &args);
    let final_text = args.text.clone().or(jot_text);

    if final_text.is_none() && args.markdown.is_none() {
        eprintln!("🛑 You must provide either text or a markdown file.");
        return;
    }

    // Build message metadata using centralized schema
    let message = make_message_metadata(
        &avatar_name,
        final_text.clone(),
        args.markdown.clone(),
        args.img.clone(),
        args.parent.clone(),
    );
    let message_id = message["id"].as_str().unwrap().to_string();

    // Save message file
    let msg_path = fur_dir.join("messages").join(format!("{}.json", message_id));
    write_json(&msg_path, &message);

    // Load index + thread
    let index_path = fur_dir.join("index.json");
    let mut index = read_json(&index_path);

    let thread_id = index["active_thread"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let thread_path = fur_dir.join("threads").join(format!("{}.json", thread_id));
    let mut thread = read_json(&thread_path);

    // === Linkage ===
    if let Some(ref parent_id) = args.parent {
        attach_to_parent(fur_dir, parent_id, &message_id);
    } else {
        if let Some(arr) = thread["messages"].as_array_mut() {
            arr.push(json!(message_id));
        }
    }

    // Save updated thread + index
    write_json(&thread_path, &thread);
    index["current_message"] = json!(message_id);
    write_json(&index_path, &index);

    // Display confirmation
    let (_, emoji) = resolve_avatar(&avatars, &avatar_name);
    println!(
        "✍️ Message jotted: [{}] {} [{}] {}",
        &message_id[..8],
        thread_id,
        avatar_name,
        emoji
    );
}


fn resolve_avatar_and_text(avatars: &Value, args: &JotArgs) -> (String, Option<String>) {
    let map = avatars.as_object().expect("avatars.json must be valid");

    let main = map
        .get("main")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    match (&args.avatar, &args.positional_text) {
        (Some(a), Some(t)) => (a.clone(), Some(t.clone())),
        (Some(a), None) if map.contains_key(a) => (a.clone(), args.text.clone()),
        (Some(a), None) => (main, Some(a.clone())),
        (None, Some(t)) => (main, Some(t.clone())),
        (None, None) => (main, args.text.clone()),
    }
}

fn attach_to_parent(fur_dir: &Path, parent_id: &str, message_id: &str) {
    let parent_path = fur_dir.join("messages").join(format!("{}.json", parent_id));
    if let Ok(content) = fs::read_to_string(&parent_path) {
        if let Ok(mut parent) = serde_json::from_str::<Value>(&content) {
            if let Some(children) = parent["children"].as_array_mut() {
                children.push(json!(message_id));
            } else {
                parent["children"] = json!([message_id]);
            }
            write_json(&parent_path, &parent);
        }
    }
}
