use std::fs;
use std::path::{Path};
use serde_json::{Value, json};
use std::io::{self, Write};
use crate::commands::conversation::ThreadArgs;
use colored::*;

pub fn resolve_target_thread_id(
    index: &Value,
    args: &ThreadArgs,
) -> Option<String> {
    let empty_vec: Vec<Value> = Vec::new();
    let threads: Vec<String> = index["threads"]
        .as_array()
        .unwrap_or(&empty_vec)
        .iter()
        .filter_map(|t| t.as_str().map(|s| s.to_string()))
        .collect();

    // If ID prefix provided
    if let Some(prefix) = &args.id {
        let matches: Vec<&String> = threads
            .iter()
            .filter(|tid| tid.starts_with(prefix))
            .collect();

        return match matches.as_slice() {
            [] => {
                eprintln!("❌ No conversation matches '{}'", prefix);
                None
            }
            [single] => Some((*single).clone()),
            _ => {
                eprintln!("❌ Ambiguous prefix '{}': {:?}", prefix, matches);
                None
            }
        };
    }

    // Otherwise use active thread
    let active = index["active_thread"].as_str().unwrap_or("").to_string();
    if active.is_empty() {
        eprintln!("❌ No active conversation to delete.");
        return None;
    }

    Some(active)
}

pub fn confirm_delete_primary() -> bool {

    println!("Are you sure you want to delete this conversation? (y/N)");
    print!("> ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    input.trim().to_lowercase() == "y"
}

pub fn confirm_delete_destructive() -> bool {

    println!();
    println!(
        "{}",
        "⚠️  Reminder: deleting a conversation is a destructive action.\n\
         It cannot be reversed unless the project is version-controlled (git)."
            .color(Color::BrightRed)
            .bold()
    );
    println!();
    println!("Type DELETE to confirm:");
    print!("> ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    input.trim() == "DELETE"
}

pub fn perform_conversation_deletion(
    index: &mut Value,
    fur_dir: &Path,
    target_tid: &str,
    threads: &[String],
) {
    let convo_path = fur_dir.join("threads").join(format!("{}.json", target_tid));

    // Load convo to extract message IDs + title
    let convo_content = fs::read_to_string(&convo_path)
        .expect("Failed to load conversation JSON.");
    let convo: Value = serde_json::from_str(&convo_content).unwrap();

    let title = convo["title"].as_str().unwrap_or("Untitled");
    let msg_ids: Vec<String> = convo["messages"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    println!(
        "🗑️  Deleting conversation {} \"{}\"...",
        &target_tid[..8],
        title
    );

    // 1. Delete conversation JSON
    let _ = fs::remove_file(&convo_path);

    // 2. Delete message files and markdown attachments
    for mid in msg_ids {
        let msg_path = fur_dir.join("messages").join(format!("{}.json", mid));

        if let Ok(content) = fs::read_to_string(&msg_path) {
            if let Ok(msg_json) = serde_json::from_str::<Value>(&content) {
                if let Some(md_raw) = msg_json["markdown"].as_str() {
                    let md_path = Path::new(md_raw);
                    if md_path.is_absolute() {
                        let _ = fs::remove_file(md_path);
                    } else {
                        let _ = fs::remove_file(Path::new(".").join(md_raw));
                    }
                }
            }
        }

        let _ = fs::remove_file(&msg_path);
    }

    // 3. Update index.json (remove thread)
    let new_threads: Vec<String> = threads
        .iter()
        .filter(|tid| tid.as_str() != target_tid)
        .cloned()
        .collect();

    index["threads"] = json!(new_threads);

    // 4. Clear active thread if it matches deleted
    if index["active_thread"].as_str() == Some(target_tid) {
        index["active_thread"] = Value::Null;
        index["current_message"] = Value::Null;
    }

    // 5. Save index.json
    let index_path = fur_dir.join("index.json");
    fs::write(&index_path, serde_json::to_string_pretty(&index).unwrap()).unwrap();

    println!("✔️ Conversation deleted successfully.");
}
