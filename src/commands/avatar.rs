use std::fs::{self};
use std::path::{Path};
use serde_json::{json, Value};

pub fn run_avatar(avatar: String, other: bool, emoji: String) {
    // If it's not the main user, ensure the --other flag is provided
    if avatar == "main" && other {
        eprintln!("❌ 'main' cannot be set with the --other flag. It is the default avatar.");
        return;
    }

    // If it's not the main user, ensure the --other flag is provided
    if avatar != "main" && !other {
        eprintln!("❌ To create an avatar for '{}', please use the --other flag.", avatar);
        return;
    }

    // If defining a new avatar, rename 'main' to the new avatar name
    if avatar != "main" {
        rename_main_avatar(&avatar, &emoji);
    }

    // Create or update the avatar
    create_avatar(&avatar, &emoji);

    println!("✔️ Avatar '{}' created with emoji '{}'.", avatar, emoji);
}

fn rename_main_avatar(new_avatar_name: &str, emoji: &str) {
    let mut avatars = load_avatars();

    // Convert Value to Map<String, Value> to modify it
    if let Some(map) = avatars.as_object_mut() {
        // Remove "main" if it exists and add the new avatar
        if let Some(_) = map.remove("main") {
            map.insert(new_avatar_name.to_string(), json!(emoji));
            save_avatars(&avatars);
        } else {
            eprintln!("❌ 'main' avatar not found. Cannot rename.");
        }
    } else {
        eprintln!("❌ Failed to load avatars as a map.");
    }
}

fn create_avatar(avatar_name: &str, emoji: &str) {
    let mut avatars = load_avatars();

    // Convert Value to Map<String, Value> to modify it
    if let Some(map) = avatars.as_object_mut() {
        // Add or update the avatar with the provided emoji
        map.insert(avatar_name.to_string(), json!(emoji));

        // Save updated avatars to file
        save_avatars(&avatars);
    } else {
        eprintln!("❌ Failed to load avatars as a map.");
    }
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
    fs::write(avatars_path, serde_json::to_string_pretty(avatars).unwrap()).unwrap();
}
