use std::fs;
use std::path::Path;
use serde_json::{json, Value};
use crate::frs::ast::{Thread, Message};

use uuid::Uuid;
use chrono::Utc;

/// Persist a parsed Thread into .fur/threads + .fur/messages
pub fn persist_frs(thread: &Thread) -> String {
    let fur_dir = Path::new(".fur");
    if !fur_dir.exists() {
        panic!("🚨 .fur directory not initialized. Run `fur new` at least once.");
    }

    // 1. Create new thread ID
    let thread_id = Uuid::new_v4().to_string();
    let timestamp = Utc::now().to_rfc3339();

    // 2. Collect messages and write them
    let mut message_ids: Vec<String> = Vec::new();
    persist_messages(&thread.messages, None, &mut message_ids);

    // 3. Write thread JSON
    let thread_json = json!({
        "id": thread_id,
        "created_at": timestamp,
        "title": thread.title,
        "tags": thread.tags,
        "messages": message_ids,
    });

    let thread_path = fur_dir.join("threads").join(format!("{}.json", thread_id));
    fs::write(&thread_path, serde_json::to_string_pretty(&thread_json).unwrap())
        .expect("❌ Could not write thread file");

    // 4. Update index.json
    let index_path = fur_dir.join("index.json");
    let mut index_data: Value =
        serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();

    index_data["threads"]
        .as_array_mut()
        .unwrap()
        .push(thread_id.clone().into());
    index_data["active_thread"] = thread_id.clone().into();
    index_data["current_message"] = Value::Null;

    fs::write(
        &index_path,
        serde_json::to_string_pretty(&index_data).unwrap(),
    )
    .unwrap();

    println!("🌱 Imported thread into .fur: {} — \"{}\"", &thread_id[..8], thread.title);

    thread_id
}

fn persist_messages(msgs: &[Message], parent: Option<String>, all_ids: &mut Vec<String>) {
    for m in msgs {
        let msg_id = Uuid::new_v4().to_string();

        // Children are persisted recursively
        let mut child_ids: Vec<String> = Vec::new();
        persist_messages(&m.children, Some(msg_id.clone()), &mut child_ids);

        let msg_json = json!({
            "id": msg_id,
            "avatar": m.avatar,
            "name": m.avatar, // store avatar name, emoji already in avatars.json
            "text": m.text,
            "markdown": m.file,
            "parent": parent,
            "children": child_ids,
            "timestamp": Utc::now().to_rfc3339(),
        });

        let path = Path::new(".fur/messages").join(format!("{}.json", msg_id));
        fs::write(&path, serde_json::to_string_pretty(&msg_json).unwrap())
            .expect("❌ Could not write message file");

        all_ids.push(msg_id);
    }
}

