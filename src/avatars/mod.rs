//! Avatar identity: names, emoji, and the `main` pointer.
//!
//! viceroy: this lived in `src/frs/` but is not FurScript. `resolve_avatar` is
//! called by `tree`, `status`, and the renderers; `load_avatars` by `new`,
//! `jot`, `avatar`, and `onboard`. Only `frs::parser` uses it from inside the
//! DSL. `frs::avatars` still resolves here via a re-export, so no existing
//! import path breaks.

pub mod emojis;

use rand::prelude::IndexedRandom;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

/// Shown when an avatar has no emoji assigned, and written by `fur rebuild`
/// for every avatar it recovers from documents.
pub const PLACEHOLDER: &str = "🐾";

/// Reserved for the main avatar.
pub const MAIN_EMOJI: &str = "🦊";

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

pub fn resolve_avatar(avatars: &Value, key: &str) -> (String, String) {
    // If key matches a known avatar name → return (name, emoji)
    if let Some(emoji) = avatars.get(key).and_then(|v| v.as_str()) {
        return (key.to_string(), emoji.to_string());
    }

    // If key looks like an emoji already → reverse-lookup name
    if let Some((name, _)) = avatars
        .as_object()
        .and_then(|map| map.iter().find(|(_, v)| v.as_str() == Some(key)))
    {
        return (name.clone(), key.to_string());
    }

    (key.to_string(), PLACEHOLDER.to_string()) // fallback
}

/// Names that clearly belong to a model or agent.
///
/// viceroy: the short markers used to be plain substring tests, so `ai` matched
/// `claire`, `raina`, and `mail`, and `bot` matched `robotics`. Short markers
/// are matched as whole tokens now; only the distinctive long ones stay
/// substring matches, so `gpt5` and `claude-3` still resolve.
pub fn is_bot_name(name: &str) -> bool {
    let n = name.to_lowercase();

    const SUBSTRING_MARKERS: [&str; 6] = ["gpt", "claude", "gemini", "bard", "grok", "copilot"];

    if SUBSTRING_MARKERS.iter().any(|m| n.contains(m)) {
        return true;
    }

    const TOKEN_MARKERS: [&str; 7] = ["ai", "bot", "llm", "agent", "assistant", "model", "llama"];

    n.split(|c: char| !c.is_alphanumeric())
        .any(|tok| TOKEN_MARKERS.contains(&tok))
}

/// A sensible starting emoji for a name, so suggestions look considered rather
/// than arbitrary. Falls back to a neutral person glyph.
pub fn get_random_emoji_for_name(name: &str) -> String {
    if is_bot_name(name) {
        return "🤖".to_string();
    }

    if let Some(emoji) = role_emoji(name) {
        return emoji.to_string();
    }

    let pool = ["👤", "🧑", "👔", "🙂"];
    pool.choose(&mut rand::rng()).unwrap_or(&"👤").to_string()
}

/// Roles that recur often enough to be worth naming directly.
fn role_emoji(name: &str) -> Option<&'static str> {
    const ROLES: &[(&str, &str)] = &[
        ("me", MAIN_EMOJI),
        ("self", MAIN_EMOJI),
        ("user", MAIN_EMOJI),
        ("boss", "💼"),
        ("manager", "💼"),
        ("hr", "📋"),
        ("therapist", "🛋"),
        ("lawyer", "⚖"),
        ("doctor", "🩺"),
        ("client", "🤝"),
        ("team", "👥"),
        ("cat", "🐱"),
        ("dog", "🐶"),
    ];

    let key = name.trim().to_lowercase();

    ROLES
        .iter()
        .find(|(candidate, _)| key == *candidate)
        .map(|(_, emoji)| *emoji)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_models() {
        assert!(is_bot_name("gpt5"));
        assert!(is_bot_name("claude"));
        assert!(is_bot_name("my-ai"));
        assert!(is_bot_name("research agent"));
    }

    #[test]
    fn does_not_mistake_people_for_bots() {
        // Each of these contains "ai" or "bot" as a substring.
        assert!(!is_bot_name("claire"));
        assert!(!is_bot_name("raina"));
        assert!(!is_bot_name("robotics_lab"));
        assert!(!is_bot_name("management"));
    }

    #[test]
    fn roles_get_stable_emoji() {
        assert_eq!(get_random_emoji_for_name("hr"), "📋");
        assert_eq!(get_random_emoji_for_name("me"), MAIN_EMOJI);
    }
}