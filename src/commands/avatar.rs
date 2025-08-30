use std::fs::{self};
use std::path::Path;
use serde_json::{json, Value};
use rand::prelude::*;
use rand::rng;

pub fn run_avatar(avatar: Option<String>, other: bool, emoji: Option<String>, view: bool) {
    if view {
        let avatars = load_avatars();
        println!("📇 Avatars:");
        for (name, emoji) in avatars.as_object().unwrap_or(&serde_json::Map::new()) {
            if name == "main" {
                println!("⭐ main → {}", emoji.as_str().unwrap_or("?"));
            } else {
                println!("{} {}", emoji.as_str().unwrap_or("🐾"), name);
            }
        }
        return;
    }

    let avatar = match avatar {
        Some(a) => a,
        None => {
            eprintln!("❌ No avatar name provided. Use `fur avatar <name>` or `--view`.");
            return;
        }
    };

    let mut avatars = load_avatars();

    if !other {
        let e = emoji.unwrap_or_else(|| "🦊".to_string());

        // Remove old main if different
        if let Some(old_main) = avatars.get("main").and_then(|v| v.as_str()).map(|s| s.to_string()) {
            if old_main != avatar {
                if let Some(map) = avatars.as_object_mut() {
                    map.remove(&old_main);
                }
            }
        }

        avatars["main"] = json!(avatar);
        avatars[&avatar] = json!(e);

        println!("✔️ Main avatar set: {} [{}]", e, avatar);
    } else {
        let e = emoji.unwrap_or_else(|| get_random_emoji());
        avatars[&avatar] = json!(e);
        println!("✔️ Other avatar '{}' created with emoji '{}'", avatar, e);
    }

    save_avatars(&avatars);
}

fn load_avatars() -> Value {
    let avatars_path = Path::new(".fur/avatars.json");
    if avatars_path.exists() {
        let content = fs::read_to_string(avatars_path).unwrap();
        serde_json::from_str(&content).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    }
}

fn save_avatars(avatars: &Value) {
    let avatars_path = Path::new(".fur/avatars.json");
    fs::write(
        avatars_path,
        serde_json::to_string_pretty(avatars).unwrap(),
    )
    .unwrap();
}

fn get_random_emoji() -> String {
    let emojis = ["👹", "🐵", "🐧", "🐺", "🦁"];
    let mut rng = rng();
    emojis.choose(&mut rng).unwrap_or(&"🐾").to_string()
}
