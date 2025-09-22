use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use chrono::Utc;
use colored::*;

use crate::commands::jot::{self, JotArgs};

/// Interactive chat-style jot for long / structured messages
pub fn run_chat(avatar: Option<String>) {
    println!("{}", "💬 Write / Copy-Paste your Markdown or text below.".bright_cyan());
    println!("{}", "↪ Finish with Ctrl+D (Linux/macOS) or Ctrl+Z then Enter (Windows).".white());
    println!("{}", "↪ Press Ctrl+C to cancel.".white());

    // --- Capture multi-line input
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer).unwrap();

    if buffer.trim().is_empty() {
        println!("⚠️ No content provided. Aborting.");
        return;
    }

    // --- Confirm
    print!("You have finished writing. Continue? [Y/n]: ");
    io::stdout().flush().unwrap();
    let mut confirm = String::new();
    io::stdin().read_line(&mut confirm).unwrap();
    if confirm.trim().eq_ignore_ascii_case("n") {
        println!("❌ Cancelled.");
        return;
    }

    // --- Filename suggestion
    let default_name = format!("chats/CHAT-{}.md", Utc::now().format("%Y%m%d-%H%M%S"));
    println!("Save as? (default: {})", default_name);
    print!("> ");
    io::stdout().flush().unwrap();
    let mut fname = String::new();
    io::stdin().read_line(&mut fname).unwrap();
    let fname = fname.trim();
    let path = if fname.is_empty() { default_name } else { fname.to_string() };

    // Ensure chats dir exists
    if let Some(parent) = Path::new(&path).parent() {
        fs::create_dir_all(parent).ok();
    }

    fs::write(&path, &buffer).expect("❌ Failed to write file");
    println!("💾 Saved to {}", path.green());

    // --- Reuse jot logic to attach to thread
    let args = JotArgs {
        avatar,
        positional_text: None,
        text: None,
        markdown: Some(path),
        img: None,
        parent: None,
    };
    jot::run_jot(args);
}
