use crate::frs::avatars::{load_avatars, save_avatars, get_random_emoji_for_name};
use crate::frs::emojis::{preview_emojis, search_emojis};
use serde_json::json;
use std::io::{self, Write};
use colored::*;
use crate::renderer::list::render_list;

pub fn run_avatar() {
    let avatars = load_avatars();

    if let Some(map) = avatars.as_object() {
        if map.is_empty() {
            println!("(no avatars yet)");
            return;
        }

        let mut rows = Vec::new();
        let mut active_idx = None;

        for (i, (name, val)) in map.iter().enumerate() {
            if name == "main" {
                if let Some(target) = val.as_str() {
                    rows.push(vec!["⭐ main".to_string(), target.to_string()]);
                    active_idx = Some(i);
                }
            } else {
                let emoji = val.as_str().unwrap_or("🐾");
                rows.push(vec![name.to_string(), emoji.to_string()]);
            }
        }

        render_list("Avatars", &["Role", "Emoji"], rows, active_idx);
    }
}



/// `fur avatar new` → onboarding wizard
pub fn run_avatar_new() {
    let mut avatars = load_avatars();

    println!("\n{}", "== Create Avatar ==".bright_magenta().bold());
    println!(
        "{}",
        "A secondary avatar is anyone who isn’t you (the main user).\n\
         Examples: an AI, your boss, your therapist, or your cat. \n\
         If you choose [n], you’re creating or replacing the *main* avatar."
            .bright_cyan()
    );
    print!("Secondary avatar? [Y/n]: ");
    io::stdout().flush().unwrap();
    let mut sec_in = String::new();
    io::stdin().read_line(&mut sec_in).unwrap();
    let sec_in = sec_in.trim().to_lowercase();
    let is_secondary = sec_in.is_empty() || sec_in == "y" || sec_in == "yes";

    if !is_secondary {
        // main avatar
        print!("Main avatar name [me]: ");
        io::stdout().flush().unwrap();
        let mut main_in = String::new();
        io::stdin().read_line(&mut main_in).unwrap();
        let main_in = main_in.trim();
        let main_name = if main_in.is_empty() { "me" } else { main_in };

        avatars["main"] = json!(main_name);
        avatars[main_name] = json!("🦊");
        println!("[OK] Main avatar set: {}", main_name);
    } else {
        // secondary avatar
        print!("Choose name [ai]: ");
        io::stdout().flush().unwrap();
        let mut other_in = String::new();
        io::stdin().read_line(&mut other_in).unwrap();
        let other_in = other_in.trim();
        let other_name = if other_in.is_empty() { "ai" } else { other_in };

        // emoji selection
        print!("Skip emoji? [Y/n]: ");
        io::stdout().flush().unwrap();
        let mut skip_in = String::new();
        io::stdin().read_line(&mut skip_in).unwrap();
        let skip_in = skip_in.trim().to_lowercase();
        let skip = skip_in.is_empty() || skip_in == "y" || skip_in == "yes";

        let emoji = if skip {
            get_random_emoji_for_name(other_name)
        } else {
            preview_emojis(50);

            loop {
                print!("Your choice: ");
                io::stdout().flush().unwrap();
                let mut input = String::new();
                io::stdin().read_line(&mut input).unwrap();
                let input = input.trim();

                // index from global preview
                if let Ok(idx) = input.parse::<usize>() {
                    if let Some(e) = emojis::iter().nth(idx) {
                        break e.to_string();
                    } else {
                        println!("Index out of range, try again.");
                        continue;
                    }
                }

                // keyword search
                let matches = search_emojis(input);
                if matches.is_empty() {
                    println!("No matches for '{}'. Try again.", input);
                    continue;
                }

                println!("Matches for '{}':", input);
                for (i, emoji) in matches.iter().enumerate() {
                    println!("#{:<2} {:<2}  — {}", i, emoji, emoji.name());
                }

                print!("Pick a hash index from these results: ");
                io::stdout().flush().unwrap();
                let mut idx_in = String::new();
                io::stdin().read_line(&mut idx_in).unwrap();
                let idx_in = idx_in.trim();

                if let Ok(i) = idx_in.parse::<usize>() {
                    if let Some(e) = matches.get(i) {
                        break e.to_string();
                    }
                }

                println!("Invalid choice, looping again.");
            }
        };

        avatars[other_name] = json!(emoji);
        println!("[OK] Other avatar '{}' created with emoji '{}'", other_name, emoji);
    }

    save_avatars(&avatars);
    println!("✅ Avatar creation complete. Use `fur avatar --view` to list all avatars.");
}


