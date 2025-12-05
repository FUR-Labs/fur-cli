use std::fs;
use std::path::{Path, PathBuf};
use serde_json::{Value, json};
use uuid::Uuid;
use chrono::Utc;
use std::collections::HashMap;

pub fn load_conversation_metadata(path: &Path) -> (String, Vec<String>) {
    let convo: Value =
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();

    let title = convo["title"].as_str().unwrap_or("Untitled").to_string();

    let messages = convo["messages"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect::<Vec<_>>();

    (title, messages)
}

pub fn make_new_conversation_header(
    old_title: &str,
    custom_title: Option<String>,
) -> (String, String, String) {
    let new_id = Uuid::new_v4().to_string();
    let timestamp = Utc::now().to_rfc3339();
    let new_title = custom_title.unwrap_or_else(|| format!("Clone of {}", old_title));
    (new_id, new_title, timestamp)
}

pub fn build_id_remap(old_messages: &[String]) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for old in old_messages {
        map.insert(old.clone(), Uuid::new_v4().to_string());
    }
    map
}

pub fn clone_all_messages(id_map: &HashMap<String, String>, old_messages: &[String]) {
    let messages_dir = Path::new(".fur/messages");

    for old_id in old_messages {
        let old_msg_path = messages_dir.join(format!("{}.json", old_id));

        let old_msg: Value =
            serde_json::from_str(&fs::read_to_string(&old_msg_path).unwrap()).unwrap();

        let new_id = id_map.get(old_id).unwrap();

        let new_parent = remap_optional(&old_msg["parent"], id_map);
        let new_children = remap_vec(&old_msg["children"], id_map);
        let new_branches = remap_vec(&old_msg["branches"], id_map);

        // Copy markdown if exists
        let new_markdown = clone_markdown_if_any(&old_msg);

        let mut new_msg = old_msg.clone();
        new_msg["id"] = json!(new_id);
        new_msg["timestamp"] = json!(Utc::now().to_rfc3339());
        new_msg["parent"] = new_parent;
        new_msg["children"] = json!(new_children);
        new_msg["branches"] = json!(new_branches);
        new_msg["markdown"] = match new_markdown {
            Some(path) => json!(path),
            None => Value::Null,
        };

        // Write new JSON file
        let new_path = messages_dir.join(format!("{}.json", new_id));
        fs::write(new_path, serde_json::to_string_pretty(&new_msg).unwrap()).unwrap();
    }
}

pub fn remap_optional(val: &Value, map: &HashMap<String, String>) -> Value {
    match val.as_str().and_then(|v| map.get(v)) {
        Some(new) => json!(new),
        None => Value::Null,
    }
}

pub fn remap_vec(val: &Value, map: &HashMap<String, String>) -> Vec<Value> {
    val.as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v.as_str())
        .filter_map(|old| map.get(old))
        .map(|new| json!(new))
        .collect()
}

pub fn clone_markdown_if_any(old_msg: &Value) -> Option<String> {
    if let Some(md_raw) = old_msg["markdown"].as_str() {
        let old_md_path = PathBuf::from(md_raw);

        if old_md_path.exists() {
            let ts = Utc::now().format("CHAT-%Y%m%d-%H%M%S.md").to_string();
            let new_md = format!("chats/{}", ts);
            fs::copy(&old_md_path, &new_md).expect("❌ Failed to copy markdown file");
            return Some(new_md);
        }
    }
    None
}

pub fn write_new_conversation(
    new_id: &str,
    new_title: &str,
    timestamp: &str,
    id_map: &HashMap<String, String>,
    old_messages: &[String],
) {
    let threads_dir = Path::new(".fur/threads");

    let new_messages: Vec<String> = old_messages
        .iter()
        .map(|old| id_map.get(old).unwrap().clone())
        .collect();

    let convo = json!({
        "id": new_id,
        "title": new_title,
        "created_at": timestamp,
        "messages": new_messages,
        "tags": []
    });

    let new_path = threads_dir.join(format!("{}.json", new_id));
    fs::write(new_path, serde_json::to_string_pretty(&convo).unwrap()).unwrap();
}

pub fn update_index(new_id: &str) {
    let index_path = Path::new(".fur/index.json");
    let mut index: Value =
        serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();

    index["threads"]
        .as_array_mut()
        .unwrap()
        .push(json!(new_id));

    index["active_thread"] = json!(new_id);
    index["current_message"] = Value::Null;

    fs::write(index_path, serde_json::to_string_pretty(&index).unwrap()).unwrap();
}
