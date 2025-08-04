use std::fs;
use std::path::Path;
use serde_json::Value;

pub fn run_timeline() {
    let fur_dir = Path::new(".fur");
    let index_path = fur_dir.join("index.json");

    if !index_path.exists() {
        eprintln!("🚨 .fur/ not found. Run `fur new` first.");
        return;
    }

    let index_data: Value = serde_json::from_str(
        &fs::read_to_string(&index_path).expect("Cannot read index.json")
    ).unwrap();

    let thread_id = match index_data["active_thread"].as_str() {
        Some(id) => id,
        None => {
            eprintln!("⚠️ No active thread.");
            return;
        }
    };

    let thread_path = fur_dir.join("threads").join(format!("{}.json", thread_id));
    let thread_data: Value = serde_json::from_str(
        &fs::read_to_string(&thread_path).expect("Cannot read thread")
    ).unwrap();

    let empty = vec![];
    let messages = thread_data["messages"].as_array().unwrap_or(&empty);

    if messages.is_empty() {
        println!("🕳️ Thread is empty.");
        return;
    }

    println!("🧵 Thread: {}\n", &thread_data["title"]);

    for msg_id in messages {
        let msg_id_str = msg_id.as_str().unwrap();
        let msg_path = fur_dir.join("messages").join(format!("{}.json", msg_id_str));
        if let Ok(msg_content) = fs::read_to_string(&msg_path) {
            if let Ok(msg_json) = serde_json::from_str::<Value>(&msg_content) {
                let time = msg_json["created_at"].as_str().unwrap_or("???");
                let role = msg_json["role"].as_str().unwrap_or("unknown");
                let text = msg_json["text"].as_str().unwrap_or("???");

                println!("🕰️  {} [{}]:\n{}\n", time, role, text);
            }
        }
    }
}
