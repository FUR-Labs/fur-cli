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

/// Return true if the name clearly looks like a bot/LLM.
fn is_bot_name(name: &str) -> bool {
    let n = name.to_lowercase();

    // lean markers (substring is fine for most)
    const MARKERS: [&str; 8] = [
        "gpt", "claude", "gemini", "bard", "grok", "bot", "ai", "llm",
    ];

    // fast path: any simple substring marker
    if MARKERS.iter().any(|m| n.contains(m)) {
        return true;
    }

    // safer whole-word match for "agent" (avoid hitting "management")
    if n.split(|c: char| !c.is_alphanumeric())
        .any(|tok| tok == "agent")
    {
        return true;
    }

    false
}

/// Emoji selection for new/unknown avatars:
/// - Fox stays reserved for main via your `save_avatars_with_main`
/// - Bots → 🤖
pub fn get_random_emoji_for_name(name: &str) -> String {
    if is_bot_name(name) {
        return "🤖".to_string();
    }
    let pool = ["👤"];
    pool.choose(&mut rand::rng()).unwrap_or(&"👔").to_string()
}

// Back-compat wrapper (if you still call `get_random_emoji()` without a name)
pub fn get_random_emoji() -> String {
    get_random_emoji_for_name("")
}
