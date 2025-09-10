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

/// Save avatars, ensuring `main` is always set
pub fn save_avatars_with_main(avatars: &mut Value, main: &str) {
    // assign emoji if missing
    if avatars.get(main).is_none() {
        let emoji = get_random_emoji();
        avatars[main] = json!(emoji);
        println!("✨ Assigned emoji {} to main avatar \"{}\"", emoji, main);
    }
    // set main explicitly
    avatars["main"] = json!(main);
    avatars[main] = json!("🦊");  // force fox

    let path = Path::new(".fur/avatars.json");
    if let Ok(serialized) = serde_json::to_string_pretty(avatars) {
        let _ = fs::write(path, serialized);
    }
}

pub fn resolve_avatar(avatars: &Value, key: &str) -> (String, String) {
    // If key matches a known avatar name → return (name, emoji)
    if let Some(emoji) = avatars.get(key).and_then(|v| v.as_str()) {
        return (key.to_string(), emoji.to_string());
    }

    // If key looks like an emoji already → reverse-lookup name
    if let Some((name, _)) = avatars.as_object()
        .and_then(|map| map.iter().find(|(_, v)| v.as_str() == Some(key)))
    {
        return (name.clone(), key.to_string());
    }

    (key.to_string(), "🐾".to_string()) // fallback
}


pub fn collect_avatars(msgs: &[Message], acc: &mut Vec<String>) {
    for m in msgs {
        if !acc.contains(&m.avatar) {
            acc.push(m.avatar.clone());
        }
        collect_avatars(&m.children, acc);
        for block in &m.branches {
            collect_avatars(block, acc);
        }
    }
}

pub fn get_random_emoji() -> String {
    let emojis = ["👹", "🐵", "🐧", "🐺", "🦁", "🦊"];
    let mut rng = rand::rng();
    emojis.choose(&mut rng).unwrap_or(&"🐾").to_string()
}
