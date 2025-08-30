use std::fs;
use std::path::Path;
use serde_json::Value;

pub fn run_status() {
    let fur_dir = Path::new(".fur");
    let index_path = fur_dir.join("index.json");

    if !index_path.exists() {
        eprintln!("🚨 .fur/ not found. Run `fur new` first.");
        return;
    }

    let index: Value =
        serde_json::from_str(&fs::read_to_string(&index_path).expect("❌ Cannot read index.json"))
            .unwrap();

    let thread_id = index["active_thread"].as_str().unwrap_or("❓");
    let current_msg_id = index["current_message"].as_str().unwrap_or("");

    let thread_path = fur_dir.join("threads").join(format!("{}.json", thread_id));
    let thread: Value =
        serde_json::from_str(&fs::read_to_string(&thread_path).expect("❌ Cannot read thread"))
            .unwrap();

    println!("🧠 Current FUR Status");
    println!("─────────────────────────────");
    println!(
        "📌 Active thread: {} ({})",
        thread["title"].as_str().unwrap_or("Untitled"),
        thread_id
    );
    println!("🧭 Current message: {}", current_msg_id);
    println!("─────────────────────────────");

    let fallback = vec![];
    let messages = thread["messages"].as_array().unwrap_or(&fallback);

    let mut id_to_message = std::collections::HashMap::new();

    for msg_id in messages {
        if let Some(id) = msg_id.as_str() {
            let path = fur_dir.join("messages").join(format!("{}.json", id));
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(json) = serde_json::from_str::<Value>(&content) {
                    id_to_message.insert(id.to_string(), json);
                }
            }
        }
    }

    let mut lineage = vec![];
    let mut current = current_msg_id;
    while let Some(msg) = id_to_message.get(current) {
        lineage.push(current.to_string());
        match msg["parent"].as_str() {
            Some(parent_id) => current = parent_id,
            None => break,
        }
    }
    lineage.reverse();

    for id in &lineage {
        if let Some(msg) = id_to_message.get(id) {
            let avatar = msg["avatar"].as_str().unwrap_or("🐾");
            let name = msg["name"].as_str().unwrap_or("???");
            let text = msg
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| {
                    msg.get("markdown")
                        .and_then(|v| v.as_str())
                        .unwrap_or("<no content>")
                });

            let preview = text.lines().next().unwrap_or("").chars().take(40).collect::<String>();
            let marker = if id == current_msg_id { "🧭 (current)" } else { "✅" };
            let id_display = &id[..8];

            if msg.get("markdown").is_some() {
                println!("{preview} {avatar} [{name}] 📄 {id_display} {marker}");
            } else {
                println!("{preview} {avatar} [{name}] {id_display} {marker}");
            }
        }
    }

    println!("─────────────────────────────");
    println!("Children of current:");
    if let Some(curr_msg) = id_to_message.get(current_msg_id) {
        let empty_children = vec![];
        let children = curr_msg["children"].as_array().unwrap_or(&empty_children);
        for child in children {
            if let Some(child_id) = child.as_str() {
                if let Some(child_msg) = id_to_message.get(child_id) {
                    let avatar = child_msg["avatar"].as_str().unwrap_or("🐾");
                    let name = child_msg["name"].as_str().unwrap_or("???");
                    let text = child_msg
                        .get("text")
                        .and_then(|v| v.as_str())
                        .unwrap_or_else(|| {
                            child_msg
                                .get("markdown")
                                .and_then(|v| v.as_str())
                                .unwrap_or("<no content>")
                        });

                    let preview =
                        text.lines().next().unwrap_or("").chars().take(40).collect::<String>();
                    let id_display = &child_id[..8];

                    if child_msg.get("markdown").is_some() {
                        println!("🔹 {preview} {avatar} [{name}] 📄 {id_display}");
                    } else {
                        println!("🔹 {preview} {avatar} [{name}] {id_display}");
                    }
                }
            }
        }
    } else {
        println!("(No current message found.)");
    }
}
