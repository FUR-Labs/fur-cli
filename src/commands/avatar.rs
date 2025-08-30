use std::fs::{self};
use std::path::Path;
use serde_json::{json, Value};
use rand::prelude::*; // Import prelude to bring in `choose`
use rand::rng;

pub fn run_avatar(avatar: String, other: bool, emoji: Option<String>) {
    let mut avatars = load_avatars();

    if !other {
        // Set main avatar
        let e = emoji.unwrap_or_else(|| "🦊".to_string());
        avatars["main"] = json!(avatar);       // main pointer
        avatars[&avatar] = json!(e);           // actual avatar entry
        println!("✔️ Main avatar set: {} [{}]", e, avatar);
    } else {
        // Set secondary avatar
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

// Get a random emoji for secondary avatars
fn get_random_emoji() -> String {
    let emojis = ["👹", "👧", "👤", "🐺", "🤖"];
    let mut rng = rng(); // Create a random number generator
    emojis.choose(&mut rng).unwrap_or(&"🐾").to_string()
}
