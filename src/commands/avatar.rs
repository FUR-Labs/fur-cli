use crate::frs::avatars::{load_avatars, save_avatars, get_random_emoji_for_name};
use serde_json::json;

pub fn run_avatar(avatar: Option<String>, other: bool, emoji: Option<String>, view: bool) {
    let mut avatars = load_avatars();

    // ✅ View mode (explicit --view OR no avatar name given)
    if view || avatar.is_none() {
        println!("📇 Avatars:");
        if let Some(map) = avatars.as_object() {
            if map.is_empty() {
                println!("(no avatars yet)");
            } else {
                for (name, emoji) in map {
                    if name == "main" {
                        println!("⭐ main → {}", emoji.as_str().unwrap_or("?"));
                    } else {
                        println!("{} {}", emoji.as_str().unwrap_or("🐾"), name);
                    }
                }
            }
        } else {
            println!("(avatars.json is invalid or empty)");
        }
        return;
    }

    // ✅ Set main or other avatar
    let avatar = avatar.unwrap();
    if !other {
        // Main avatar
        let e = emoji.unwrap_or_else(|| "🦊".to_string());
        avatars["main"] = json!(avatar);
        avatars[&avatar] = json!(e);
        println!("✔️ Main avatar set: {} [{}]", e, avatar);
    } else {
        // Other avatar
        let e = emoji.unwrap_or_else(|| get_random_emoji_for_name(&avatar));
        avatars[&avatar] = json!(e);
        println!("✔️ Other avatar '{}' created with emoji '{}'", avatar, e);
    }

    // ✅ Always persist changes
    save_avatars(&avatars);
}
