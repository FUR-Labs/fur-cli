use colored::*;
use serde_json::{json, Value};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use uuid::Uuid;

use crate::frs::avatars::{get_random_emoji_for_name, load_avatars, save_avatars};
use crate::schema::{make_conversation_metadata, make_index_metadata};

fn init_fur_dir(fur_dir: &Path) -> io::Result<()> {
    fs::create_dir_all(fur_dir.join("threads"))?;
    fs::create_dir_all(fur_dir.join("messages"))?;

    let mut f = File::create(fur_dir.join("index.json"))?;
    let initial_index = make_index_metadata();
    f.write_all(serde_json::to_string_pretty(&initial_index)?.as_bytes())?;
    Ok(())
}

pub fn onboarding_interactive() -> (String, String) {
    // Main avatar
    println!("\n{}", "== Main Avatar ==".bright_magenta().bold());
    println!(
        "{}",
        "This is YOU (or your team). The default voice in this conversation.\n\
         Whenever you jot without specifying an avatar, it will be attributed here."
            .bright_cyan()
    );
    print!("{}", "Main avatar name [me]: ");
    io::stdout().flush().unwrap();
    let mut main_in = String::new();
    io::stdin().read_line(&mut main_in).unwrap();
    let mut main_name = main_in.trim().to_string();
    if main_name.is_empty() {
        main_name = "me".to_string();
    } else if main_name == "main" {
        println!(
            "{}",
            "[WARN] 'main' is reserved as a pointer. Using 'me' instead."
                .yellow()
                .bold()
        );
        main_name = "me".to_string();
    }
    println!(
        "{}",
        format!("[OK] Main avatar set: {}", main_name)
            .bright_green()
            .bold()
    );

    // === Secondary Avatar ===
    println!("\n{}", "== Secondary Avatar ==".bright_magenta().bold());
    println!(
        "{}",
        "You can't have a conversation with one person.\n\
         Let's log at least one other avatar. This could be an AI, your boss, your therapist, or karen_from_hr."
            .bright_cyan()
    );
    print!("{}", "Another avatar [ai]: ");
    io::stdout().flush().unwrap();
    let mut other_in = String::new();
    io::stdin().read_line(&mut other_in).unwrap();
    let other_name = {
        let trimmed = other_in.trim();
        if trimmed.is_empty() {
            "ai".to_string()
        } else {
            trimmed.to_string()
        }
    };
    println!(
        "{}",
        format!("[OK] Other avatar set: {}", other_name)
            .bright_green()
            .bold()
    );

    (main_name, other_name)
}

/// Non-interactive onboarding (useful for scripts or tests)
pub fn onboarding_auto(main: &str, other: &str) {
    let mut avatars = load_avatars();
    avatars["main"] = json!(main);
    avatars[main] = json!("🦊");
    avatars[other] = json!(get_random_emoji_for_name(other));
    save_avatars(&avatars);
}

fn run_new_internal(
    name: String,
    auto: bool,
    main_avatar: Option<String>,
    other_avatar: Option<String>,
) {
    let fur_dir = Path::new(".fur");

    if !fur_dir.exists() {
        init_fur_dir(fur_dir).expect("Failed to create .fur structure");
        println!("{}", "[INIT] .fur/ directory created".bright_green().bold());

        if auto {
            onboarding_auto(
                main_avatar.as_deref().unwrap_or("me"),
                other_avatar.as_deref().unwrap_or("ai"),
            );
        } else {
            let (main, other) = onboarding_interactive();
            onboarding_auto(&main, &other);
        }

        println!(
            "\n{}",
            "Ready! Use:\n  fur jot <your message>\n  fur jot <other avatar> <their message>"
                .bright_cyan()
        );
    }

    let conversation_id = Uuid::new_v4().to_string();
    let conversation_meta = make_conversation_metadata(&name, &conversation_id);

    // --- Write conversation ---
    let convo_path = fur_dir
        .join("threads")
        .join(format!("{}.json", conversation_id));
    fs::write(
        &convo_path,
        serde_json::to_string_pretty(&conversation_meta).unwrap(),
    )
    .expect("Could not write conversation file");

    // --- Update index ---
    let index_path = fur_dir.join("index.json");
    let mut index: Value = serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();

    if let Some(arr) = index["threads"].as_array_mut() {
        arr.push(json!(conversation_id.clone()));
    } else {
        index["threads"] = json!([conversation_id.clone()]);
    }
    index["active_thread"] = json!(conversation_id.clone());
    index["current_message"] = Value::Null;

    fs::write(&index_path, serde_json::to_string_pretty(&index).unwrap()).unwrap();

    println!(
        "{}",
        format!(
            "[NEW] Thread created: {} — \"{}\"",
            &conversation_id[..8],
            name
        )
        .bright_green()
        .bold()
    );
}

// === legacy-compatible wrapper ===
pub fn run_new(name: String) {
    run_new_internal(name, false, None, None);
}
