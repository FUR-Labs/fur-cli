use std::fs;
use std::path::{Path, PathBuf};
use serde_json::{json, Value};

use crate::schema::make_message_metadata;
use crate::frs::avatars::{load_avatars, resolve_avatar};
use crate::commands::jot::JotArgs;

pub struct FurContext {
    pub fur_dir: PathBuf,
    pub avatars: Value,
    pub conversation_id: String,
}

pub fn load_context() -> Result<FurContext, String> {
    let fur_dir = Path::new(".fur");

    if !fur_dir.exists() {
        return Err("🚨 .fur/ not found. Run `fur new` first.".into());
    }

    let avatars = load_avatars();
    let index = read_json(&fur_dir.join("index.json"));

    let conversation_id = index["active_thread"]
        .as_str()
        .unwrap_or("main")
        .to_string();

    Ok(FurContext {
        fur_dir: fur_dir.to_path_buf(),
        avatars,
        conversation_id,
    })
}


pub fn resolve_avatar_and_text(avatars: &Value, args: &JotArgs) -> (String, Option<String>) {
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

pub fn validate_inputs(text: &Option<String>, markdown: &Option<String>) -> Result<(), String> {
    if text.is_none() && markdown.is_none() {
        Err("🛑 You must provide either text or a markdown file.".into())
    } else {
        Ok(())
    }
}

pub fn build_message(
    avatar: &str,
    text: Option<String>,
    markdown: Option<String>,
    img: Option<String>,
    parent: Option<String>,
) -> Value {
    make_message_metadata(avatar, text, markdown, img, parent)
}

// ================================
// jot effects
// ================================


pub fn save_message(fur_dir: &Path, msg_id: &str, msg: &Value) {
    let path = fur_dir.join("messages").join(format!("{}.json", msg_id));
    write_json(&path, msg);
}

pub fn update_conversation(ctx: &FurContext, msg_id: &str, parent: Option<&str>) {
    if let Some(pid) = parent {
        attach_to_parent(&ctx.fur_dir, pid, msg_id);
        return;
    }

    let convo_path = ctx
        .fur_dir
        .join("threads")
        .join(format!("{}.json", ctx.conversation_id));

    let mut conversation = read_json(&convo_path);

    if let Some(arr) = conversation["messages"].as_array_mut() {
        arr.push(json!(msg_id));
    }

    write_json(&convo_path, &conversation);
}

fn attach_to_parent(fur_dir: &Path, parent_id: &str, message_id: &str) {
    let parent_path = fur_dir
        .join("messages")
        .join(format!("{}.json", parent_id));

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

pub fn update_index(fur_dir: &Path, msg_id: &str) {
    let index_path = fur_dir.join("index.json");
    let mut index = read_json(&index_path);

    index["current_message"] = json!(msg_id);

    write_json(&index_path, &index);
}

pub fn print_confirmation(avatars: &Value, avatar_name: &str, msg_id: &str, conversation_id: &str) {
    let (_, emoji) = resolve_avatar(avatars, avatar_name);

    println!(
        "✍️ Message jotted down: [{}] {} [{}] {}",
        &msg_id[..8],
        conversation_id,
        avatar_name,
        emoji
    );
}


// ================================
// IO helpers
// ================================

fn read_json(path: &Path) -> Value {
    serde_json::from_str(
        &fs::read_to_string(path).expect("Cannot read JSON")
    ).unwrap()
}

fn write_json(path: &Path, value: &Value) {
    fs::write(
        path,
        serde_json::to_string_pretty(value).unwrap(),
    ).unwrap();
}
