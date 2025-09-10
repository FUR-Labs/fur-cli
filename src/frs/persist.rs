use uuid::Uuid;
use chrono::Utc;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::io::{self, Write};

use crate::frs::ast::{Thread, Message};

/// Persist a parsed Thread into .fur/threads + .fur/messages
pub fn persist_frs(thread: &Thread) -> String {
    let fur_dir = Path::new(".fur");
    if !fur_dir.exists() {
        panic!("🚨 .fur directory not initialized. Run `fur new` at least once.");
    }

    // --- Check if a thread with the same title already exists ---
    let index_path = fur_dir.join("index.json");
    let mut index_data: Value =
        serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();

    let mut overwrite = false;
    let mut old_thread_id: Option<String> = None;

    if let Some(threads) = index_data["threads"].as_array() {
        for tid in threads {
            if let Some(tid_str) = tid.as_str() {
                let tpath = fur_dir.join("threads").join(format!("{}.json", tid_str));
                if let Ok(txt) = fs::read_to_string(&tpath) {
                    if let Ok(tjson) = serde_json::from_str::<Value>(&txt) {
                        if tjson["title"].as_str() == Some(&thread.title) {
                            // Found duplicate title
                            println!("⚠️ Thread with title \"{}\" already exists.", thread.title);
                            print!("Overwrite? [Y/n]: ");
                            io::stdout().flush().unwrap();

                            let mut input = String::new();
                            io::stdin().read_line(&mut input).unwrap();
                            let response = input.trim().to_lowercase();

                            if response.is_empty() || response == "y" || response == "yes" {
                                overwrite = true;
                                old_thread_id = Some(tid_str.to_string());
                            } else {
                                println!("🚫 Skipped importing thread \"{}\".", thread.title);
                                return tid_str.to_string();
                            }
                        }
                    }
                }
            }
        }
    }

    // --- If overwrite, delete old thread + messages ---
    if overwrite {
        if let Some(tid) = &old_thread_id {
            delete_old_thread(tid);
            if let Some(arr) = index_data["threads"].as_array_mut() {
                arr.retain(|v| v.as_str() != Some(tid));
            }
        }
    }

    // --- Now persist fresh thread ---
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
    index_data["threads"].as_array_mut().unwrap().push(thread_id.clone().into());
    index_data["active_thread"] = thread_id.clone().into();
    index_data["current_message"] = Value::Null;
    if index_data["schema_version"].as_str() == Some("0.1") {
        index_data["schema_version"] = Value::String("0.2".to_string());
    }

    fs::write(&index_path, serde_json::to_string_pretty(&index_data).unwrap()).unwrap();

    println!("🌱 Imported thread into .fur: {} — \"{}\"", &thread_id[..8], thread.title);
    thread_id
}

/// Delete an old thread and all its message files.
fn delete_old_thread(thread_id: &str) {
    let fur_dir = Path::new(".fur");
    let thread_path = fur_dir.join("threads").join(format!("{}.json", thread_id));

    if let Ok(content) = fs::read_to_string(&thread_path) {
        if let Ok(thread_json) = serde_json::from_str::<Value>(&content) {
            if let Some(msgs) = thread_json["messages"].as_array() {
                for m in msgs {
                    if let Some(mid) = m.as_str() {
                        delete_message_recursive(mid, fur_dir);
                    }
                }
            }
        }
    }

    let _ = fs::remove_file(thread_path);
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
