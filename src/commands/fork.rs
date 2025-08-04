use std::fs;
use std::path::Path;
use serde_json::{Value, json};
use uuid::Uuid;
use chrono::Utc;

pub fn run_fork_from_active() {
    let index_path = Path::new(".fur").join("index.json");

    let index_data: Value = serde_json::from_str(&fs::read_to_string(index_path).unwrap()).unwrap();
    let active_thread = index_data["active_thread"]
        .as_str()
        .expect("No active thread set");

    run_fork(active_thread);
}

pub fn run_fork(thread_id: &str) {
    let fur_dir = Path::new(".fur");
    let threads_dir = fur_dir.join("threads");
    let index_path = fur_dir.join("index.json");

    let old_path = threads_dir.join(format!("{}.json", thread_id));
    if !old_path.exists() {
    eprintln!("❌ Thread ID {} does not exist at path {:?}", thread_id, old_path);
    return;
}


    // Read old thread
    let old_data: Value = serde_json::from_str(
        &fs::read_to_string(&old_path).unwrap()
    ).unwrap();

    let new_id = Uuid::new_v4().to_string();
    let timestamp = Utc::now().to_rfc3339();

    // Copy messages and create new thread metadata
    let messages = old_data["messages"].clone();

    let new_thread = json!({
        "id": new_id,
        "title": format!("Fork of {}", thread_id),
        "created_at": timestamp,
        "forked_from": thread_id,
        "messages": messages
    });

    let new_path = threads_dir.join(format!("{}.json", new_id));
    fs::write(&new_path, serde_json::to_string_pretty(&new_thread).unwrap()).unwrap();

    // Update index.json
    let mut index_data: Value = serde_json::from_str(
        &fs::read_to_string(&index_path).unwrap()
    ).unwrap();

    index_data["active_thread"] = json!(new_id);
    index_data["threads"].as_array_mut().unwrap().push(json!(new_id));

    fs::write(index_path, serde_json::to_string_pretty(&index_data).unwrap()).unwrap();

    println!("🌱 Forked thread {} → {}", thread_id, new_id);
}
