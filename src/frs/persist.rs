use uuid::Uuid;
use chrono::Utc;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

use crate::frs::ast::{Thread, Message};

/// Persist a parsed Thread into .fur/threads + .fur/messages
pub fn persist_frs(thread: &Thread) -> String {
    let fur_dir = Path::new(".fur");
    if !fur_dir.exists() {
        panic!("🚨 .fur directory not initialized. Run `fur new` at least once.");
    }

    let thread_id = Uuid::new_v4().to_string();
    let timestamp = Utc::now().to_rfc3339();

    // Persist only the *root* jots; recursion handles nested branches
    let root_ids = persist_level(&thread.messages, None);

    let thread_json = json!({
        "id": thread_id,
        "created_at": timestamp,
        "title": thread.title,
        "tags": thread.tags,
        "messages": root_ids, // only roots here
    });

    let thread_path = fur_dir.join("threads").join(format!("{}.json", thread_id));
    fs::write(&thread_path, serde_json::to_string_pretty(&thread_json).unwrap())
        .expect("❌ Could not write thread file");

    // Update index.json
    let index_path = fur_dir.join("index.json");
    let mut index_data: Value =
        serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();

    index_data["threads"].as_array_mut().unwrap().push(thread_id.clone().into());
    index_data["active_thread"] = thread_id.clone().into();
    index_data["current_message"] = Value::Null;

    fs::write(&index_path, serde_json::to_string_pretty(&index_data).unwrap()).unwrap();

    println!("🌱 Imported thread into .fur: {} — \"{}\"", &thread_id[..8], thread.title);
    thread_id
}

/// Persist a list of messages that share the same parent.
/// Returns the IDs of **these** messages (not descendants).
fn persist_level(msgs: &[Message], parent: Option<String>) -> Vec<String> {
    let mut ids_at_this_level: Vec<String> = Vec::new();

    for m in msgs {
        let msg_id = Uuid::new_v4().to_string();

        // For this message, persist each branch block; collect:
        //  - groups of direct child IDs (per block)
        //  - a flat list of all direct child IDs
        let mut branch_groups_ids: Vec<Vec<String>> = Vec::new();
        let mut direct_children_ids: Vec<String> = Vec::new();

        for branch_block in &m.branches {
            let group_ids = persist_level(branch_block, Some(msg_id.clone()));
            if !group_ids.is_empty() {
                direct_children_ids.extend(group_ids.clone());
                branch_groups_ids.push(group_ids);
            }
        }

        // Now write this message JSON
        let msg_json = json!({
            "id": msg_id,
            "avatar": m.avatar,
            "name": m.avatar,
            "text": m.text,
            "markdown": m.file,
            "parent": parent,
            "children": direct_children_ids,   // flat direct children (compat)
            "branches": branch_groups_ids,     // grouped branch blocks (new)
            "timestamp": Utc::now().to_rfc3339(),
        });

        let path = Path::new(".fur/messages").join(format!("{}.json", msg_id));
        fs::write(&path, serde_json::to_string_pretty(&msg_json).unwrap())
            .expect("❌ Could not write message file");

        ids_at_this_level.push(msg_id);
    }

    ids_at_this_level
}
