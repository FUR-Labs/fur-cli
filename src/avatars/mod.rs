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

/// Model families distinctive enough to match anywhere in a name, so `gpt5`,
/// `claude-opus` and `mi-gemini` all resolve.
///
/// This is a first guess, not a registry. `fur avatar <name> --kind ai` is the
/// authoritative answer and is recorded in the archive; the list only spares
/// people the flag in the common case. It therefore does not need to be
/// exhaustive, and is deliberately not user-configurable — a second, editable
/// source of truth would silently reclassify existing avatars when edited.
pub const AI_NAME_FRAGMENTS: &[&str] = &[
    "gpt",
    "claude",
    "gemini",
    "bard",
    "grok",
    "copilot",
    "llama",
    "mistral",
    "deepseek",
    "qwen",
];

/// Generic words that mean "not a person", matched as whole tokens.
///
/// these are substring-unsafe — `ai` appears in `claire` and `raina`,
/// `bot` in `robotics` — so they are split on non-alphanumerics and compared
/// whole. Anything added here must survive that test.
pub const AI_NAME_TOKENS: &[&str] = &[
    "ai", "ia", "bot", "llm", "agent", "assistant", "asistente", "model", "modelo",
];

/// Names that clearly belong to a model or agent.
pub fn is_bot_name(name: &str) -> bool {
    let n = name.to_lowercase();

    if AI_NAME_FRAGMENTS.iter().any(|m| n.contains(m)) {
        return true;
    }

    n.split(|c: char| !c.is_alphanumeric())
        .any(|tok| AI_NAME_TOKENS.contains(&tok))
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

/// Optional per-avatar metadata, stored under the reserved `meta` key.
///
/// ```json
/// { "main": "andrew", "andrew": "🦊", "claude": "✨",
///   "meta": { "andrew": {"role": "Investigador Principal"},
///             "claude": {"kind": "ai"} } }
/// ```
///
/// viceroy: deliberately additive rather than a schema bump. `load_avatars`,
/// `resolve_avatar` and all six call sites keep working untouched, and a
/// project that never sets a role writes the same bytes it always did.
/// `role` is absent when unset — never null, never "", never "?".
pub const META_KEY: &str = "meta";

/// Role for an avatar, when one has been set.
pub fn role_of(avatars: &Value, name: &str) -> Option<String> {
    avatars
        .get(META_KEY)?
        .get(name)?
        .get("role")?
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
}

/// "human" or "ai". Explicit metadata wins; otherwise inferred from the name,
/// which is what `is_bot_name` already exists to answer.
pub fn kind_of(avatars: &Value, name: &str) -> String {
    let explicit = avatars
        .get(META_KEY)
        .and_then(|m| m.get(name))
        .and_then(|entry| entry.get("kind"))
        .and_then(|v| v.as_str());

    match explicit {
        Some("ai") => "ai".to_string(),
        Some("human") => "human".to_string(),
        _ if is_bot_name(name) => "ai".to_string(),
        _ => "human".to_string(),
    }
}

/// Set or clear one metadata field. Clearing removes the key entirely, and an
/// avatar left with no metadata is removed from `meta` so the file does not
/// accumulate empty objects.
pub fn set_meta(avatars: &mut Value, name: &str, field: &str, value: Option<&str>) {
    if !avatars.is_object() {
        *avatars = json!({});
    }
    if avatars.get(META_KEY).map(|m| !m.is_object()).unwrap_or(true) {
        avatars[META_KEY] = json!({});
    }

    match value {
        Some(v) if !v.trim().is_empty() => {
            if avatars[META_KEY].get(name).is_none() {
                avatars[META_KEY][name] = json!({});
            }
            avatars[META_KEY][name][field] = json!(v.trim());
        }
        _ => {
            if let Some(entry) = avatars[META_KEY].get_mut(name) {
                if let Some(map) = entry.as_object_mut() {
                    map.remove(field);
                }
            }
        }
    }

    let empty = avatars[META_KEY]
        .get(name)
        .and_then(|e| e.as_object())
        .map(|m| m.is_empty())
        .unwrap_or(false);

    if empty {
        if let Some(meta) = avatars[META_KEY].as_object_mut() {
            meta.remove(name);
        }
    }

    if avatars[META_KEY]
        .as_object()
        .map(|m| m.is_empty())
        .unwrap_or(false)
    {
        if let Some(map) = avatars.as_object_mut() {
            map.remove(META_KEY);
        }
    }
}

/// True for keys that are not avatar names.
pub fn is_reserved_key(key: &str) -> bool {
    key == "main" || key == META_KEY
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
    fn recognises_spanish_and_current_models() {
        assert!(is_bot_name("asistente"));
        assert!(is_bot_name("mi-ia"));
        assert!(is_bot_name("llama3"));
        assert!(is_bot_name("deepseek-r1"));
        assert!(!is_bot_name("maria"));
        assert!(!is_bot_name("diana"));
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