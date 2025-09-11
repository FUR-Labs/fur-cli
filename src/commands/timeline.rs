use std::fs;
use std::path::Path;
use serde_json::{Value, json};
use clap::Parser;
use colored::*; // 🎨 for styling
use crate::frs::avatars::resolve_avatar;
use chrono::{DateTime, FixedOffset, Local};

/// Timeline command structure with verbose flag
#[derive(Parser)]
pub struct TimelineArgs {
    /// Whether to show full content of Markdown files
    #[arg(short, long)]
    pub verbose: bool,
}

pub fn run_timeline(args: TimelineArgs) {
    let fur_dir = Path::new(".fur");
    let index_path = fur_dir.join("index.json");

    if !index_path.exists() {
        eprintln!("{}", "🚨 .fur/ not found. Run `fur new` first.".red().bold());
        return;
    }

    let index_data: Value = serde_json::from_str(
        &fs::read_to_string(&index_path).expect("Cannot read index.json")
    ).unwrap();

    let thread_id = match index_data["active_thread"].as_str() {
        Some(id) => id,
        None => {
            eprintln!("{}", "⚠️ No active thread.".yellow().bold());
            return;
        }
    };

    let thread_path = fur_dir.join("threads").join(format!("{}.json", thread_id));
    let thread_data: Value = serde_json::from_str(
        &fs::read_to_string(&thread_path).expect("Cannot read thread")
    ).unwrap();

    // load avatars.json once
    let avatars: Value = serde_json::from_str(
        &fs::read_to_string(fur_dir.join("avatars.json")).unwrap_or_else(|_| "{}".to_string())
    ).unwrap_or(json!({}));

    let empty = vec![];
    let messages = thread_data["messages"].as_array().unwrap_or(&empty);

    if messages.is_empty() {
        println!("{}", "🕳️ Thread is empty.".bright_black());
        return;
    }

    println!(
        "{} {}",
        "🧵 Thread:".bold().cyan(),
        thread_data["title"].as_str().unwrap_or("Untitled").bright_green().bold().italic()
    );
    println!();

    // Root messages (stem level)
    for msg_id in messages {
        if let Some(id) = msg_id.as_str() {
            render_message(&fur_dir, id, "Root".to_string(), args.verbose, &avatars);
        }
    }
}

fn render_message(fur_dir: &Path, msg_id: &str, label: String, verbose: bool, avatars: &Value) {
    let msg_path = fur_dir.join("messages").join(format!("{}.json", msg_id));
    let msg_content = match fs::read_to_string(&msg_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let msg_json: Value = match serde_json::from_str(&msg_content) {
        Ok(v) => v,
        Err(_) => return,
    };

    // --- Timestamp formatting (localized) ---
    let raw_time = msg_json["timestamp"].as_str().unwrap_or("???");
    let (date_str, time_str, micros_str) = if let Ok(dt) = raw_time.parse::<DateTime<FixedOffset>>() {
        let local_dt = dt.with_timezone(&Local); // convert to local timezone
        (
            local_dt.format("%Y-%m-%d").to_string(),   // date
            local_dt.format("%H:%M:%S").to_string(),   // time
            format!("{}", local_dt.format(".%f%:z")),  // micros + offset
        )
    } else {
        (raw_time.to_string(), "".to_string(), "".to_string())
    };

    // --- Avatar + name ---
    let avatar_key = msg_json["avatar"].as_str().unwrap_or("???");
    let (name, emoji) = resolve_avatar(avatars, avatar_key);

    // --- Message text ---
    let text = msg_json["text"].as_str().unwrap_or_else(|| {
        if msg_json["markdown"].is_null() {
            "No comment"
        } else {
            "No comment, just a file:"
        }
    });

    // --- Print divider line ---
    println!("{}", "─────────────────────────────".bright_black());

    // --- Print header line (no clock emoji) ---
    println!(
        "{} {}{} {} {}:",
        date_str.cyan(),                          // date
        time_str.bright_cyan().bold(),            // time
        micros_str.bright_black(),                // micros dim
        format!("[{}]", label).bright_green(),    // branch/label
        format!("{} [{}]", emoji.yellow(), name.bright_yellow()), // avatar + name
    );

    // --- Print message text ---
    println!("{}\n", text.white());

    // --- Markdown linked file (if any) ---
    if let Some(path_str) = msg_json["markdown"].as_str() {
        println!("🔍 Resolving markdown file at: {}", path_str);
        if verbose {
            if let Ok(contents) = fs::read_to_string(path_str) {
                println!("📄 Linked Markdown Content:\n{}", contents);
            } else {
                println!("⚠️ Could not read linked markdown file at: {}", path_str);
            }
        } else {
            println!("📂 Linked Markdown file: {}", path_str);
        }
    }

    // Branch-aware recursion
    if let Some(branches) = msg_json["branches"].as_array() {
        if !branches.is_empty() {
            for (b_idx, branch) in branches.iter().enumerate() {
                if let Some(branch_arr) = branch.as_array() {
                    for child_id in branch_arr {
                        if let Some(c_id) = child_id.as_str() {
                            let new_label = if label == "Root" {
                                format!("Branch {}", b_idx + 1)
                            } else {
                                format!("{}.{}", label.replace("Branch ", ""), b_idx + 1)
                            };
                            render_message(fur_dir, c_id, new_label, verbose, avatars);
                        }
                    }
                }
            }
            return; // ✅ don’t fall back to children if branches exist
        }
    }

    // Legacy fallback: use children if no branches
    if let Some(children) = msg_json["children"].as_array() {
        for child_id in children {
            if let Some(c_id) = child_id.as_str() {
                render_message(fur_dir, c_id, label.clone(), verbose, avatars);
            }
        }
    }
}
