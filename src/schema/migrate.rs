use std::fs;
use std::path::Path;

use serde_json::Value;
use walkdir::WalkDir;

use colored::*;
use std::io::{stdin, stdout, Write};

use crate::schema::upgrade_message_schema;

use crate::schema::CURRENT_SCHEMA;

pub fn detect_old_schema() -> bool {
    let messages_dir = std::path::Path::new(".fur/messages");

    if !messages_dir.exists() {
        return false;
    }

    for entry in std::fs::read_dir(messages_dir).unwrap() {
        let path = entry.unwrap().path();

        if !path.is_file() {
            continue;
        }

        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&content) {
                let schema = msg["schema_version"].as_str().unwrap_or("0.1");

                if schema != CURRENT_SCHEMA {
                    return true;
                }
            }
        }
    }

    false
}

pub fn ask_yes_no(question: &str) -> bool {
    print!("{} [Y/n]: ", question.bold().bright_yellow());
    stdout().flush().unwrap();

    let mut input = String::new();
    stdin().read_line(&mut input).unwrap();

    let input = input.trim().to_lowercase();

    input.is_empty() || input == "y" || input == "yes"
}

pub fn run_backfill_meta() {
    let messages_dir = Path::new(".fur/messages");

    println!("\n🔧 Running metadata migration...\n");

    let mut updated = 0;

    for entry in WalkDir::new(messages_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let content = fs::read_to_string(path).unwrap();

        let mut msg: Value = serde_json::from_str(&content).unwrap();

        if upgrade_message_schema(&mut msg) {
            fs::write(path, serde_json::to_string_pretty(&msg).unwrap()).unwrap();

            updated += 1;
        }
    }

    println!("✔ Metadata backfilled for {}\n", updated);
}
