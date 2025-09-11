use crate::frs::avatars::{load_avatars, save_avatars, get_random_emoji_for_name};
use serde_json::json;

pub fn run_avatar(avatar: Option<String>, other: bool, emoji: Option<String>, view: bool) {
    let mut avatars = load_avatars();

    // ✅ Default to view if --view flag OR no avatar name provided
    if view || avatar.is_none() {
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

    let avatar = avatar.unwrap();

    if !other {
        let e = emoji.unwrap_or_else(|| "🦊".to_string());

        // Update main mapping
        avatars["main"] = json!(avatar);
        avatars[&avatar] = json!(e);

        println!("✔️ Main avatar set: {} [{}]", e, avatar);
    } else {
        let e = emoji.unwrap_or_else(|| get_random_emoji_for_name(&avatar));
        avatars[&avatar] = json!(e);
        println!("✔️ Other avatar '{}' created with emoji '{}'", avatar, e);
    }

    save_avatars(&avatars);
}
