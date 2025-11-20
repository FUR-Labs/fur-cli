use uuid::Uuid;
use chrono::Utc;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::io::{self, Write};

use crate::frs::ast::{Thread, Message};
use crate::frs::ast::ScriptItem;

/// Persist a parsed Thread into .fur/threads + .fur/messages
pub fn persist_frs(conversation: &Thread) -> String {
    let fur_dir = Path::new(".fur");
    if !fur_dir.exists() {
        panic!("🚨 .fur directory not initialized. Run `fur new` at least once.");
    }

    // --- Check if a conversation with the same title already exists ---
    let index_path = fur_dir.join("index.json");
    let mut index_data: Value =
        serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();

    let mut overwrite = false;
    let mut old_conversation_id: Option<String> = None;

    if let Some(threads) = index_data["threads"].as_array() {
        for tid in threads {
            if let Some(tid_str) = tid.as_str() {
                let tpath = fur_dir.join("threads").join(format!("{}.json", tid_str));
                if let Ok(txt) = fs::read_to_string(&tpath) {
                    if let Ok(tjson) = serde_json::from_str::<Value>(&txt) {
                        if tjson["title"].as_str() == Some(&conversation.title) {
                            // Found duplicate title
                            println!("⚠️ Thread with title \"{}\" already exists.", conversation.title);
                            print!("Overwrite? [Y/n]: ");
                            io::stdout().flush().unwrap();

                            let mut input = String::new();
                            io::stdin().read_line(&mut input).unwrap();
                            let response = input.trim().to_lowercase();

                            if response.is_empty() || response == "y" || response == "yes" {
                                overwrite = true;
                                old_conversation_id = Some(tid_str.to_string());
                            } else {
                                println!("🚫 Skipped importing conversation \"{}\".", conversation.title);
                                return tid_str.to_string();
                            }
                        }
                    }
                }
            }
        }
    }

    // --- If overwrite, delete old conversation + messages ---
    if overwrite {
        if let Some(tid) = &old_conversation_id {
            delete_old_conversation(tid);
            if let Some(arr) = index_data["threads"].as_array_mut() {
                arr.retain(|v| v.as_str() != Some(tid));
            }
        }
    }

    // --- Now persist fresh conversation ---
    let conversation_id = Uuid::new_v4().to_string();
    let timestamp = Utc::now().to_rfc3339();

    // Persist only the *root* jots; recursion handles nested branches
    let root_ids = persist_level(
        &conversation.items.iter().filter_map(|item| {
            if let ScriptItem::Message(m) = item { Some(m) } else { None }
        }).cloned().collect::<Vec<_>>(),
        None
    );


    let conversation_json = json!({
        "id": conversation_id,
        "created_at": timestamp,
        "title": conversation.title,
        "tags": conversation.tags,
        "messages": root_ids, // only roots here
    });

    let convo_path = fur_dir.join("threads").join(format!("{}.json", conversation_id));
    fs::write(&convo_path, serde_json::to_string_pretty(&conversation_json).unwrap())
        .expect("❌ Could not write conversation file");

    // Update index.json
    index_data["threads"].as_array_mut().unwrap().push(conversation_id.clone().into());
    index_data["active_thread"] = conversation_id.clone().into();
    index_data["current_message"] = Value::Null;
    if index_data["schema_version"].as_str() == Some("0.1") {
        index_data["schema_version"] = Value::String("0.2".to_string());
    }

    fs::write(&index_path, serde_json::to_string_pretty(&index_data).unwrap()).unwrap();

    println!("🌱 Imported conversation into .fur: {} — \"{}\"", &conversation_id[..8], conversation.title);
    conversation_id
}


/// Ephemeral persist: writes a conversation into `.fur/tmp/` for previews.
/// Returns ephemeral conversation_id.
pub fn persist_ephemeral(conversation: &Thread) -> String {
    let fur_dir = Path::new(".fur/tmp");
    if !fur_dir.exists() {
        fs::create_dir_all(fur_dir).expect("❌ Could not create .fur/tmp/");
    }

    let conversation_id = format!("ephemeral-{}", Uuid::new_v4().to_string());
    let timestamp = Utc::now().to_rfc3339();

    let root_ids = persist_level(
        &conversation.items.iter().filter_map(|item| {
            if let ScriptItem::Message(m) = item { Some(m) } else { None }
        }).cloned().collect::<Vec<_>>(),
        None
    );

    let conversation_json = json!({
        "id": conversation_id,
        "created_at": timestamp,
        "title": conversation.title,
        "tags": conversation.tags,
        "messages": root_ids,
    });

    let convo_path = fur_dir.join(format!("{}.json", conversation_id));
    fs::write(&convo_path, serde_json::to_string_pretty(&conversation_json).unwrap())
        .expect("❌ Could not write ephemeral conversation file");

    conversation_id
}

/// Clean up ephemeral conversation + messages
pub fn cleanup_ephemeral(conversation_id: &str) {
    let fur_dir = Path::new(".fur/tmp");
    let convo_path = fur_dir.join(format!("{}.json", conversation_id));
    let _ = fs::remove_file(convo_path);
    // NOTE: if we want to also clean messages, we can follow `delete_message_recursive`.
}



/// Delete an old conversation and all its message files.
fn delete_old_conversation(conversation_id: &str) {
    let fur_dir = Path::new(".fur");
    let convo_path = fur_dir.join("threads").join(format!("{}.json", conversation_id));

    if let Ok(content) = fs::read_to_string(&convo_path) {
        if let Ok(conversation_json) = serde_json::from_str::<Value>(&content) {
            if let Some(msgs) = conversation_json["messages"].as_array() {
                for m in msgs {
                    if let Some(mid) = m.as_str() {
                        delete_message_recursive(mid, fur_dir);
                    }
                }
            }
        }
    }

    let _ = fs::remove_file(convo_path);
}

/// Recursively delete a message and its children/branches.
fn delete_message_recursive(msg_id: &str, fur_dir: &Path) {
    let msg_path = fur_dir.join("messages").join(format!("{}.json", msg_id));
    if let Ok(content) = fs::read_to_string(&msg_path) {
        if let Ok(msg_json) = serde_json::from_str::<Value>(&content) {
            // delete children
            if let Some(children) = msg_json["children"].as_array() {
                for c in children {
                    if let Some(cid) = c.as_str() {
                        delete_message_recursive(cid, fur_dir);
                    }
                }
            }
            // delete branches
            if let Some(branches) = msg_json["branches"].as_array() {
                for block in branches {
                    if let Some(arr) = block.as_array() {
                        for c in arr {
                            if let Some(cid) = c.as_str() {
                                delete_message_recursive(cid, fur_dir);
                            }
                        }
                    }
                }
            }
        }
    }
    let _ = fs::remove_file(msg_path);
}

/// Persist a list of messages that share the same parent.
/// Returns the IDs of **these** messages (not descendants).
fn persist_level(msgs: &[Message], parent: Option<String>) -> Vec<String> {
    let mut ids_at_this_level: Vec<String> = Vec::new();

    for m in msgs {
        let msg_id = Uuid::new_v4().to_string();

        let mut branch_groups_ids: Vec<Vec<String>> = Vec::new();
        let mut direct_children_ids: Vec<String> = Vec::new();

        for branch_block in &m.branches {
            let group_ids = persist_level(branch_block, Some(msg_id.clone()));
            if !group_ids.is_empty() {
                direct_children_ids.extend(group_ids.clone());
                branch_groups_ids.push(group_ids);
            }
        }

        let msg_json = json!({
            "id": msg_id,
            "avatar": m.avatar,
            "name": m.avatar,
            "text": m.text,
            "markdown": m.file,
            "attachment": m.attachment,
            "parent": parent,
            "children": direct_children_ids,
            "branches": branch_groups_ids,
            "timestamp": Utc::now().to_rfc3339(),
        });

        let path = Path::new(".fur/messages").join(format!("{}.json", msg_id));
        fs::write(&path, serde_json::to_string_pretty(&msg_json).unwrap())
            .expect("❌ Could not write message file");

        ids_at_this_level.push(msg_id);
    }

    ids_at_this_level
}
