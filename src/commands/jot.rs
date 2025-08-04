use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use uuid::Uuid;
use chrono::Utc;
use clap::Args;
use serde_json::json;
use serde_json::Value;

#[derive(Args)]
pub struct JotArgs {
    #[arg(short, long)]
    pub role: String,

    #[arg(short, long)]
    pub text: String,

    #[arg(long)]
    pub md: Option<String>, // Name of the markdown file to link
}


pub fn run_jot(args: JotArgs) {
    let fur_dir = Path::new(".fur");
    let index_path = fur_dir.join("index.json");

    if !index_path.exists() {
        eprintln!("🚨 No .fur/ directory found. Run `fur new` first.");
        return;
    }

    // Load current index
    let index_data: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();

    let thread_id = index_data["active_thread"]
        .as_str()
        .expect("No active thread set.");

    // Create new message
    let message_id = Uuid::new_v4().to_string();
    let timestamp = Utc::now().to_rfc3339();

    let old_index: Value = serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();
    let parent_id = old_index["current_message"].as_str().unwrap_or("null");


    let message = json!({
        "id": message_id,
        "role": args.role,
        "text": args.text,
        "timestamp": timestamp,
        "parent": if parent_id == "null" {
            Value::Null
        } else {
            Value::String(parent_id.to_string())
        }
    });


    let message_path = fur_dir
        .join("messages")
        .join(format!("{}.json", message_id));
    let mut file = File::create(&message_path).expect("Failed to write message");
    file.write_all(message.to_string().as_bytes()).unwrap();

    // Append message to thread
    let thread_path = fur_dir.join("threads").join(format!("{}.json", thread_id));
    let mut thread_data: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&thread_path).unwrap()).unwrap();

    thread_data["messages"]
        .as_array_mut()
        .unwrap()
        .push(message_id.clone().into());

    let mut thread_file = File::create(&thread_path).unwrap();
    thread_file
        .write_all(thread_data.to_string().as_bytes())
        .unwrap();

    println!("✍️ Message jotted down to thread {}: {}", &thread_id[..8], &message_id[..8]);

    // Update index.json with new current_message
    let mut index_data: Value = serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();
    index_data["current_message"] = Value::String(message_id.clone());
    fs::write(index_path, serde_json::to_string_pretty(&index_data).unwrap()).unwrap();

    // Update parent message with new child
    if parent_id != "null" {
        let parent_path = fur_dir.join("messages").join(format!("{}.json", parent_id));
        if let Ok(parent_content) = fs::read_to_string(&parent_path) {
            if let Ok(mut parent_json) = serde_json::from_str::<Value>(&parent_content) {
                if !parent_json.get("children").is_some() {
                    parent_json["children"] = json!([]);
                }

                if let Some(children) = parent_json["children"].as_array_mut() {
                    children.push(Value::String(message_id.clone()));
                }
                fs::write(&parent_path, serde_json::to_string_pretty(&parent_json).unwrap()).unwrap();
            }
        }
    }


}
