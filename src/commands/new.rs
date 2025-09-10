use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use uuid::Uuid;
use chrono::Utc;
use serde_json::json;

/// Creates a new thread with a user-provided name.
pub fn run_new(name: String) {
    let fur_dir = Path::new(".fur");

    // Create .fur/ directory if it doesn't exist
    if !fur_dir.exists() {
        fs::create_dir_all(fur_dir.join("threads")).expect("Failed to create .fur/threads");
        fs::create_dir_all(fur_dir.join("messages")).expect("Failed to create .fur/messages");

        let index_path = fur_dir.join("index.json");
        let initial_index = json!({
            "threads": [],
            "active_thread": null,
            "created_at": Utc::now().to_rfc3339(),
            "schema_version": "0.2"
        });

        let mut file = File::create(index_path).expect("Failed to write .fur/index.json");
        file.write_all(initial_index.to_string().as_bytes()).unwrap();

        println!("🧱 Initialized .fur/ directory");
    }

    // Create new thread
    let thread_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let thread_metadata = json!({
        "id": thread_id,
        "created_at": now,
        "messages": [],
        "tags": [],
        "title": name,  // 👈 custom name instead of auto title
    });

    let thread_path = fur_dir.join("threads").join(format!("{}.json", thread_id));
    let mut thread_file = File::create(thread_path).expect("Could not create thread file");
    thread_file
        .write_all(thread_metadata.to_string().as_bytes())
        .expect("Could not write thread file");

    // Update index
    let index_path = fur_dir.join("index.json");
    let mut index_data: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();

    index_data["threads"]
        .as_array_mut()
        .unwrap()
        .push(thread_id.clone().into());
    index_data["active_thread"] = thread_id.clone().into();
    index_data["current_message"] = serde_json::Value::Null;

    let mut index_file = File::create(index_path).unwrap();
    index_file
        .write_all(index_data.to_string().as_bytes())
        .unwrap();

    println!("🌱 New thread created: {} — \"{}\"", &thread_id[..8], name);
}
