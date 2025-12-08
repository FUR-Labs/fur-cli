use serde_json::json;
use std::fs;
use std::path::Path;

use crate::commands::jot::{self, JotArgs};
use crate::commands::chat;
use crate::commands::message::{MsgArgs, detect_id, resolve_target_message};

/// Main entry point: orchestrates insertion
pub fn run_insert(args: &MsgArgs, insert_before: bool) {
    let Some(target_pfx) = &args.id_prefix else {
        eprintln!("❌ Must supply a target message ID prefix.");
        return;
    };

    // 1. Resolve prefix → full ID
    let target_id = detect_id(&Some(target_pfx.clone()))
        .unwrap_or_else(|| resolve_target_message(Some(target_pfx.clone())));

    // 2. Create new message via jot/chat passthrough
    let new_id = match create_message_from_passthrough(args) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("❌ {}", e);
            return;
        }
    };

    // 3. Insert relative to target
    insert_message_relative_to(&target_id, &new_id, insert_before);

    println!(
        "➕ Inserted {} {} {}",
        &new_id[..8],
        if insert_before { "before" } else { "after" },
        &target_id[..8]
    );
}

/// Create new message by delegating to jot/chat
fn create_message_from_passthrough(args: &MsgArgs) -> Result<String, String> {
    if args.rest.is_empty() {
        return Err("No jot/chat command provided.".into());
    }

    let cmd = args.rest[0].as_str();
    let rest = &args.rest[1..];

    match cmd {
        "jot" => {
            // jot <avatar?> <text?>
            let avatar = rest.get(0).cloned();
            let text = rest.get(1).cloned();

            let jargs = JotArgs {
                avatar,
                positional_text: text,
                text: None,
                markdown: None,
                img: None,
                parent: None,
            };

            jot::run_jot(jargs);
            Ok(get_current_message_id())
        }

        "chat" => {
            // chat <avatar?>
            let avatar = rest.get(0).cloned();
            chat::run_chat(avatar);
            Ok(get_current_message_id())
        }

        other => Err(format!("Unknown passthrough subcommand: {}", other)),
    }
}

/// Insert a message ID into threads/<thread>.json
fn insert_message_relative_to(target_id: &str, new_id: &str, insert_before: bool) {
    let fur_dir = Path::new(".fur");

    // Load index
    let index_path = fur_dir.join("index.json");
    let index: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();
    let convo_id = index["active_thread"].as_str().unwrap();

    // Load conversation
    let convo_path = fur_dir.join("threads").join(format!("{}.json", convo_id));
    let mut convo: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&convo_path).unwrap()).unwrap();

    let arr = convo["messages"].as_array_mut().unwrap();

    // Remove message from natural append location (jot/chat always append to the end)
    arr.retain(|v| v.as_str() != Some(new_id));

    // Find target index
    let idx = arr.iter().position(|v| v.as_str() == Some(target_id));

    if let Some(i) = idx {
        if insert_before {
            arr.insert(i, json!(new_id));
        } else {
            arr.insert(i + 1, json!(new_id));
        }
    } else {
        eprintln!("❌ Could not find target message {}.", target_id);
        return;
    }

    // Save updated conversation
    fs::write(&convo_path, serde_json::to_string_pretty(&convo).unwrap()).unwrap();
}

/// Retrieve the last message ID created
pub fn get_current_message_id() -> String {
    let idx_path = Path::new(".fur/index.json");
    let index: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(idx_path).unwrap()).unwrap();

    index["current_message"]
        .as_str()
        .expect("No current_message after jot/chat")
        .to_string()
}
