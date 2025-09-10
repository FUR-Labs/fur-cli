use crate::frs::avatars::{load_avatars, save_avatars, get_random_emoji};
use serde_json::json;

pub fn run_avatar(avatar: Option<String>, other: bool, emoji: Option<String>, view: bool) {
    let mut avatars = load_avatars();

    if view {
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

    if !other {
        let e = emoji.unwrap_or_else(|| "🦊".to_string());

        // Update main mapping
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
