use std::fs;
use std::path::Path;
use serde_json::{Value, json};
use uuid::Uuid;
use chrono::Utc;

pub fn run_fork_from_active(title: Option<String>) {
    let index_path = Path::new(".fur").join("index.json");
    let index_data: Value = serde_json::from_str(&fs::read_to_string(index_path).unwrap()).unwrap();
    let active_conversation = index_data["active_thread"]
        .as_str()
        .expect("No active conversation set");

    run_fork(active_conversation, title);
}

pub fn run_fork(conversation_id: &str, title: Option<String>) {
    let fur_dir = Path::new(".fur");
    let threads_dir = fur_dir.join("threads");
    let index_path = fur_dir.join("index.json");

    let old_path = threads_dir.join(format!("{}.json", conversation_id));
    if !old_path.exists() {
        eprintln!("❌ Thread ID {} does not exist at path {:?}", conversation_id, old_path);
        return;
    }

    // Read old conversation
    let old_data: Value = serde_json::from_str(
        &fs::read_to_string(&old_path).unwrap()
    ).unwrap();

    let old_title = old_data["title"].as_str().unwrap_or("Untitled");
    let new_id = Uuid::new_v4().to_string();
    let timestamp = Utc::now().to_rfc3339();

    // Pick fork title
    let fork_title: String;
    let used_custom_title = title.is_some();

    fork_title = match title {
        Some(custom) => custom,
        None => format!("Fork of {}", old_title),
    };

    let messages = old_data["messages"].clone();

    let new_conversation = json!({
        "id": new_id,
        "title": fork_title,
        "created_at": timestamp,
        "forked_from": conversation_id,
        "messages": messages
    });

    let new_path = threads_dir.join(format!("{}.json", new_id));
    fs::write(&new_path, serde_json::to_string_pretty(&new_conversation).unwrap()).unwrap();

    // Update index.json
    let mut index_data: Value = serde_json::from_str(
        &fs::read_to_string(&index_path).unwrap()
    ).unwrap();

    index_data["active_thread"] = json!(new_id);
    index_data["threads"].as_array_mut().unwrap().push(json!(new_id));

    fs::write(index_path, serde_json::to_string_pretty(&index_data).unwrap()).unwrap();


    if used_custom_title {
        println!(
            "🌱 Created fork \"{}\" from {} -- {} → {}",
            fork_title, old_title, conversation_id, new_id
        );
    } else {
        println!(
            "🌱 Forked conversation from {} -- {} → {}",
            old_title, conversation_id, new_id
        );
    }
}
