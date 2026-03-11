use std::fs;
use std::path::Path;
use serde_json::Value;
use std::collections::HashMap;

pub fn load_index_and_conversation(fur_dir: &Path)
    -> (Value, Value, String)
{
    let index_path = fur_dir.join("index.json");
    let index: Value = read_json(&index_path);

    let conversation_id = index["active_thread"].as_str().unwrap_or("");
    let current = index["current_message"].as_str().unwrap_or("").to_string();

    let convo_path = fur_dir.join("threads").join(format!("{}.json", conversation_id));
    let conversation: Value = read_json(&convo_path);

    (index, conversation, current)
}


/// Preload all reachable messages
pub fn load_conversation_messages(
    fur_dir: &Path,
    conversation: &Value
) -> HashMap<String, Value> {

    let mut id_to_message = HashMap::new();

    let mut stack: Vec<String> = conversation["messages"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|id| id.as_str().map(|s| s.to_string()))
        .collect();

    let messages_dir = fur_dir.join("messages");

    while let Some(mid) = stack.pop() {

        let msg_path = messages_dir.join(format!("{}.json", mid));

        if let Some(content) = crate::security::io::read_text_file(&msg_path) {

            if let Ok(mut obj) = serde_json::from_str::<Value>(&content) {

                // ─────────────────────────────
                // Lazy schema upgrade
                // ─────────────────────────────
                if crate::commands::jot::upgrade_message_schema(&mut obj) {

                    // write upgraded message back to disk
                    if let Ok(serialized) = serde_json::to_string_pretty(&obj) {
                        let _ = fs::write(&msg_path, serialized);
                    }
                }

                // ─────────────────────────────
                // enqueue children
                // ─────────────────────────────
                if let Some(children) = obj["children"].as_array() {
                    for c in children {
                        if let Some(cid) = c.as_str() {
                            stack.push(cid.to_string());
                        }
                    }
                }

                // ─────────────────────────────
                // enqueue branch blocks
                // ─────────────────────────────
                if let Some(blocks) = obj["branches"].as_array() {
                    for block in blocks {
                        if let Some(arr) = block.as_array() {
                            for c in arr {
                                if let Some(cid) = c.as_str() {
                                    stack.push(cid.to_string());
                                }
                            }
                        }
                    }
                }

                id_to_message.insert(mid.clone(), obj);
            }
        }
    }

    id_to_message
}


/// If current_message missing, use first in conversation.messages
pub fn first_message_fallback(conversation: &Value) -> String {
    conversation["messages"]
        .as_array()
        .and_then(|arr| arr.get(0))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn read_json(path: &Path) -> Value {

    use crate::security::io::read_text_file;

    serde_json::from_str(
        &read_text_file(path)
            .expect("❌ Project locked. Run `fur unlock`.")
    ).unwrap()
}