use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use chrono::Utc;
use clap::Args;
use serde_json::{json, Value};

#[derive(Args)]
pub struct JotArgs {
    #[arg(short, long)]
    pub role: String,

    #[arg(short, long)]
    pub text: Option<String>,

    #[arg(short = 'f', long)]
    pub file: Option<PathBuf>,
}


pub fn run_jot(args: JotArgs) {
    if args.text.is_none() && args.file.is_none() {
        eprintln!("🚨 Provide at least --text or --file");
        return;
    }

    let fur_dir = Path::new(".fur");
    let index_path = fur_dir.join("index.json");

    if !index_path.exists() {
        eprintln!("🚨 No .fur/ directory found. Run `fur new` first.");
        return;
    }

    // Load index + thread
    let index_data: Value = serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();
    let thread_id = index_data["active_thread"].as_str().unwrap();
    let parent_id = index_data["current_message"].as_str().unwrap_or("null");

    // Generate new message
    let message_id = Uuid::new_v4().to_string();
    let timestamp = Utc::now().to_rfc3339();

    let mut message = json!({
        "id": message_id,
        "role": args.role,
        "timestamp": timestamp,
        "parent": if parent_id == "null" { Value::Null } else { Value::String(parent_id.to_string()) },
    });

    if let Some(text) = args.text {
        message["text"] = Value::String(text);
    }

    if let Some(file_path) = args.file {
        match fs::canonicalize(&file_path) {
            Ok(abs_path) => {
                message["markdown"] = Value::String(abs_path.to_string_lossy().to_string());
            }
            Err(_) => {
                eprintln!("❌ Could not resolve file path: {:?}", file_path);
                return;
            }
        }
    }

    // Save message to .fur/messages/
    let message_path = fur_dir.join("messages").join(format!("{}.json", message_id));
    let mut file = File::create(&message_path).unwrap();
    file.write_all(message.to_string().as_bytes()).unwrap();

    // Append to thread
    let thread_path = fur_dir.join("threads").join(format!("{}.json", thread_id));
    let mut thread_data: Value = serde_json::from_str(&fs::read_to_string(&thread_path).unwrap()).unwrap();
    thread_data["messages"].as_array_mut().unwrap().push(Value::String(message_id.clone()));
    fs::write(&thread_path, serde_json::to_string_pretty(&thread_data).unwrap()).unwrap();

    // Update index
    let mut index_data: Value = serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();
    index_data["current_message"] = Value::String(message_id.clone());
    fs::write(index_path, serde_json::to_string_pretty(&index_data).unwrap()).unwrap();

    // Add child link to parent
    if parent_id != "null" {
        let parent_path = fur_dir.join("messages").join(format!("{}.json", parent_id));
        if let Ok(parent_str) = fs::read_to_string(&parent_path) {
            if let Ok(mut parent_json) = serde_json::from_str::<Value>(&parent_str) {
                if parent_json["children"].is_null() {
                    parent_json["children"] = json!([]);
                }
                let children = parent_json["children"].as_array_mut().unwrap();
                children.push(Value::String(message_id.clone()));
                fs::write(parent_path, serde_json::to_string_pretty(&parent_json).unwrap()).unwrap();
            }
        }
    }

    println!("✍️ Message jotted down to thread {}: {}", &thread_id[..8], &message_id[..8]);
}
