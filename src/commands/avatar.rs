use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::commands::utils::input::{ask_raw, ask_string, ask_yes_no, default_yes};
use crate::frs::avatars::{get_random_emoji_for_name, load_avatars, save_avatars};
use crate::frs::emojis::{preview_emojis, search_emojis};
use crate::renderer::table::render_table;
use colored::*;
use serde_json::json;

fn count_messages_per_avatar() -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    let messages_dir = Path::new(".fur/messages");

    if !messages_dir.exists() {
        return counts;
    }

    if let Ok(entries) = fs::read_dir(messages_dir) {
        for entry in entries.flatten() {
            if let Ok(content) = fs::read_to_string(entry.path()) {
                if let Ok(json) = serde_json::from_str::<Value>(&content) {
                    if let Some(avatar) = json["avatar"].as_str() {
                        *counts.entry(avatar.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    counts
}

pub fn run_avatar_view() {
    let avatars = load_avatars();

    if let Some(map) = avatars.as_object() {
        if map.is_empty() {
            println!("(no avatars yet)");
            return;
        }

        let mut rows = Vec::new();
        let mut active_idx = None;

        let msg_counts = count_messages_per_avatar();

        for (i, (name, val)) in map.iter().enumerate() {
            if name == "main" {
                if let Some(target) = val.as_str() {
                    let count = msg_counts.get(target).copied().unwrap_or(0);
                    rows.push(vec![
                        "⭐ main".to_string(),
                        target.to_string(),
                        count.to_string(),
                    ]);
                    active_idx = Some(i);
                }
            } else {
                let emoji = val.as_str().unwrap_or("🐾");
                let count = msg_counts.get(name).copied().unwrap_or(0);
                rows.push(vec![name.to_string(), emoji.to_string(), count.to_string()]);
            }
        }

        render_table("Avatars", &["Role", "Emoji", "Messages"], rows, active_idx);
    }
}

pub fn run_avatar_onboarding() {
    let mut avatars = load_avatars();

    println!("\n{}", "== Create Avatar ==".bright_magenta().bold());
    println!(
        "{}",
        "A secondary avatar is anyone who isn’t you (the main user).\n\
         Examples: an AI, your boss, your therapist, or your cat. \n\
         If you choose [n], you’re creating or replacing the *main* avatar."
            .bright_cyan()
    );

    let is_secondary = ask_yes_no("Secondary avatar? [Y/n]: ", default_yes);

    if is_secondary {
        create_secondary_avatar(&mut avatars);
    } else {
        create_main_avatar(&mut avatars);
    }

    save_avatars(&avatars);
    println!("✅ Avatar creation complete. Use `fur avatar --view` to list all avatars.");
}

fn create_main_avatar(avatars: &mut serde_json::Value) {
    let name = ask_string("Main avatar name [me]: ", Some("me"));

    avatars["main"] = json!(name);
    avatars[name.clone()] = json!("🦊");
    println!("[OK] Main avatar set: {}", name);
}

fn create_secondary_avatar(avatars: &mut serde_json::Value) {
    let name = ask_string("Choose name [ai]: ", Some("ai"));

    let skip = ask_yes_no("Skip emoji? [Y/n]: ", default_yes);

    let emoji = if skip {
        get_random_emoji_for_name(&name)
    } else {
        choose_emoji()
    };

    avatars[name.clone()] = json!(emoji);

    println!(
        "[OK] Other avatar '{}' created with emoji '{}'",
        name, emoji
    );
}

fn choose_emoji() -> String {
    preview_emojis(50);

    loop {
        let input = ask_raw("Your choice: ");

        // numeric index from global list
        if let Ok(idx) = input.parse::<usize>() {
            if let Some(e) = emojis::iter().nth(idx) {
                return e.to_string();
            }
            println!("Index out of range.");
            continue;
        }

        // keyword search
        let matches = search_emojis(&input);
        if matches.is_empty() {
            println!("No matches for '{}'. Try again.", input);
            continue;
        }

        println!("Matches for '{}':", input);
        for (i, emoji) in matches.iter().enumerate() {
            println!("#{:<2} {:<2}  — {}", i, emoji, emoji.name());
        }

        let pick = ask_raw("Pick a hash index from these results: ");
        if let Ok(i) = pick.parse::<usize>() {
            if let Some(e) = matches.get(i) {
                return e.to_string();
            }
        }

        println!("Invalid choice, looping again.");
    }
}
