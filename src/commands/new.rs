use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;
use uuid::Uuid;
use chrono::Utc;
use serde_json::json;
use colored::*;

use crate::frs::avatars::{load_avatars, save_avatars, get_random_emoji_for_name};

/// Creates a new thread with a user-provided name.
pub fn run_new(name: String) {
    let fur_dir = Path::new(".fur");

    // Create .fur/ directory if it doesn't exist
    if !fur_dir.exists() {
        fs::create_dir_all(fur_dir.join("threads")).expect("Failed to create .fur/threads");
        fs::create_dir_all(fur_dir.join("messages")).expect("Failed to create .fur/messages");

        let index_path = fur_dir.join("index.json");
        let initial_index = json!({
            "threads": [],
            "active_thread": null,
            "created_at": Utc::now().to_rfc3339(),
            "schema_version": "0.2"
        });

        let mut file = File::create(index_path).expect("Failed to write .fur/index.json");
        file.write_all(initial_index.to_string().as_bytes()).unwrap();

        println!("{}", "[INIT] .fur/ directory created".bright_green().bold());

        // === Interactive onboarding for avatars ===

        // Prompt for main avatar
        println!("\n{}", "== Main Avatar ==".bright_magenta().bold());
        println!(
            "{}",
            "This is YOU (or your team). The default voice in this thread.\n\
            Whenever you jot without specifying an avatar, it will be attributed here."
                .bright_cyan()
        );
        print!("{}", "Main avatar name [me]: ");
        io::stdout().flush().unwrap();
        let mut main_in = String::new();
        io::stdin().read_line(&mut main_in).unwrap();
        let main_in = main_in.trim();

        let mut main_name = if main_in.is_empty() { "me" } else { main_in };
        if main_name == "main" {
            println!(
                "{}",
                "[WARN] 'main' is reserved as a pointer. Using 'me' instead."
                    .yellow()
                    .bold()
            );
            main_name = "me";
        }

        println!(
            "{}",
            format!("[OK] Main avatar set: {}", main_name).bright_green().bold()
        );

        // Prompt for secondary avatar
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
        let other_in = other_in.trim();
        let other_name = if other_in.is_empty() { "ai" } else { other_in };

        println!(
            "{}",
            format!("[OK] Other avatar set: {}", other_name)
                .bright_green().bold()
        );

        // Load current avatars (probably empty at this point)
        let mut avatars = load_avatars();

        // Set main avatar pointer + emoji
        avatars["main"] = json!(main_name);
        avatars[main_name] = json!("🦊");

        // Set secondary avatar + auto emoji
        let e = get_random_emoji_for_name(other_name);
        avatars[other_name] = json!(e);

        save_avatars(&avatars);

        println!(
            "\n{}",
            "Ready! Use: \n  fur jot <your message>\n  fur jot <other avatar> <their message>"
                .bright_cyan()
        );
    }

    // === Create new thread ===
    let thread_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let thread_metadata = json!({
        "id": thread_id,
        "created_at": now,
        "messages": [],
        "tags": [],
        "title": name,
    });

    let thread_path = fur_dir.join("threads").join(format!("{}.json", thread_id));
    let mut thread_file = File::create(thread_path).expect("Could not create thread file");
    thread_file
        .write_all(thread_metadata.to_string().as_bytes())
        .expect("Could not write thread file");

    // Update index
    let index_path = fur_dir.join("index.json");
    let mut index_data: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();

    index_data["threads"]
        .as_array_mut()
        .unwrap()
        .push(thread_id.clone().into());
    index_data["active_thread"] = thread_id.clone().into();
    index_data["current_message"] = serde_json::Value::Null;

    let mut index_file = File::create(index_path).unwrap();
    index_file
        .write_all(index_data.to_string().as_bytes())
        .unwrap();

    println!(
        "{}",
        format!("[NEW] Thread created: {} — \"{}\"", &thread_id[..8], name)
            .bright_green()
            .bold()
    );
}
