use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::avatars::emojis::pick_emoji;
use crate::avatars::{
    get_random_emoji_for_name, kind_of, load_avatars, role_of, save_avatars, set_meta, MAIN_EMOJI,
};
use crate::commands::utils::input::{ask_string, ask_yes_no, default_yes};
use crate::renderer::table::render_table;
use colored::*;
use serde_json::json;

pub fn count_messages_per_avatar() -> HashMap<String, usize> {
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

    let Some(map) = avatars.as_object() else {
        return;
    };
    if map.is_empty() {
        println!("(no avatars yet)");
        return;
    }

    let msg_counts = count_messages_per_avatar();

    // The Role column appears only when something occupies it, so a project
    // that never sets roles sees the table it has always seen.
    let any_role = map
        .keys()
        .any(|name| !crate::avatars::is_reserved_key(name) && role_of(&avatars, name).is_some());

    let mut rows = Vec::new();
    let mut active_idx = None;

    for (name, value) in map.iter() {
        // `main` is a pointer, not an avatar: it renders as its own starred row
        // naming the avatar it points at.
        if name == "main" {
            let Some(target) = value.as_str() else {
                continue;
            };
            let count = msg_counts.get(target).copied().unwrap_or(0);
            let mut row = vec![
                "⭐ main".to_string(),
                target.to_string(),
                count.to_string(),
                kind_of(&avatars, target),
            ];
            if any_role {
                row.push(role_of(&avatars, target).unwrap_or_default());
            }
            active_idx = Some(rows.len());
            rows.push(row);
            continue;
        }

        if crate::avatars::is_reserved_key(name) {
            continue;
        }

        let emoji = value.as_str().unwrap_or("🐾").to_string();
        let count = msg_counts.get(name).copied().unwrap_or(0);

        // Kind is its own column, not a suffix on the name: the name is the
        // identifier that appears in every message marker, and decorating it
        // makes the table disagree with the documents.
        let mut row = vec![
            name.to_string(),
            emoji,
            count.to_string(),
            kind_of(&avatars, name),
        ];
        if any_role {
            row.push(role_of(&avatars, name).unwrap_or_default());
        }
        rows.push(row);
    }

    if any_role {
        render_table(
            "Avatars",
            &["Role", "Emoji", "Messages", "Kind", "Function"],
            rows,
            active_idx,
            true,
        );
    } else {
        render_table(
            "Avatars",
            &["Role", "Emoji", "Messages", "Kind"],
            rows,
            active_idx,
            true,
        );
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
    avatars[name.clone()] = json!(MAIN_EMOJI);
    println!("[OK] Main avatar set: {}", name);
}

fn create_secondary_avatar(avatars: &mut serde_json::Value) {
    let name = ask_string("Choose name [ai]: ", Some("ai"));

    let suggested = get_random_emoji_for_name(&name);

    let skip = ask_yes_no(
        &format!("Use suggested emoji {}? [Y/n]: ", suggested),
        default_yes,
    );

    // viceroy: the picker moved to `avatars::emojis::pick_emoji` so onboarding
    // can use the same one instead of a hardcoded lookup table.
    let emoji = if skip {
        suggested
    } else {
        pick_emoji("Your choice: ", None)
    };

    avatars[name.clone()] = json!(emoji);

    println!(
        "[OK] Other avatar '{}' created with emoji '{}'",
        name, emoji
    );
}

/// `fur avatar <name> --role "…" --kind ai|human --clear-role`
pub fn run_avatar_meta(name: &str, role: Option<&str>, kind: Option<&str>, clear_role: bool) {
    let mut avatars = load_avatars();

    if avatars.get(name).is_none() {
        let emoji = get_random_emoji_for_name(name);
        avatars[name] = json!(emoji);
        println!("[OK] Avatar '{}' created {}", name, emoji);
    }

    if clear_role {
        set_meta(&mut avatars, name, "role", None);
    } else if let Some(value) = role {
        set_meta(&mut avatars, name, "role", Some(value));
    }

    if let Some(value) = kind {
        set_meta(&mut avatars, name, "kind", Some(value));
    }

    save_avatars(&avatars);

    let shown_role = role_of(&avatars, name).unwrap_or_else(|| "—".to_string());
    println!(
        "🏷️  {} · {} · {}",
        name.bright_yellow(),
        kind_of(&avatars, name),
        shown_role
    );
}