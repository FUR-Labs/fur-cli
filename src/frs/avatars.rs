use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use rand::prelude::IndexedRandom;
use crate::frs::ast::Message;

pub fn load_avatars() -> Value {
    let path = Path::new(".fur/avatars.json");
    if path.exists() {
        let content = fs::read_to_string(path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    }
}

pub fn save_avatars(avatars: &Value) {
    let path = Path::new(".fur/avatars.json");
    if let Ok(serialized) = serde_json::to_string_pretty(avatars) {
        let _ = fs::write(path, serialized);
    }
}

pub fn collect_avatars(msgs: &[Message], acc: &mut Vec<String>) {
    for m in msgs {
        if !acc.contains(&m.avatar) {
            acc.push(m.avatar.clone());
        }
        // Walk flat children (legacy)
        collect_avatars(&m.children, acc);
        // Walk grouped branches
        for block in &m.branches {
            collect_avatars(block, acc);
        }
    }
}


pub fn get_random_emoji() -> String {
    let emojis = ["👹", "🐵", "🐧", "🐺", "🦁"];
    let mut rng = rand::rng();
    emojis.choose(&mut rng).unwrap_or(&"🐾").to_string()
}
