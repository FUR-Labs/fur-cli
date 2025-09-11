use std::fs;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use serde_json::Value;
use chrono::{DateTime, FixedOffset, Local};
use clap::Parser;
use colored::*;
use crate::frs::avatars::resolve_avatar;

/// Args for timeline command
#[derive(Parser)]
pub struct TimelineArgs {
    /// Whether to show full content of Markdown files
    #[arg(short, long, alias = "contents", short_alias = 'c')]
    pub verbose: bool,

    /// Output in Markdown/script style (for docs)
    #[arg(long)]
    pub out: Option<Option<String>>, // `--out` or `--out FILE.md`
}

/// Run timeline view
pub fn run_timeline(args: TimelineArgs) {
    let fur_dir = Path::new(".fur");
    let index_path = fur_dir.join("index.json");
    if !index_path.exists() {
        eprintln!("🚨 .fur/ not found. Run `fur new` first.");
        return;
    }

    // Load avatars once
    let avatars: Value = serde_json::from_str(
        &fs::read_to_string(fur_dir.join("avatars.json")).unwrap_or_else(|_| "{}".to_string())
    ).unwrap_or(serde_json::json!({}));

    // Load index
    let index: Value = serde_json::from_str(
        &fs::read_to_string(index_path).expect("❌ Cannot read index.json")
    ).unwrap();

    let thread_id = index["active_thread"].as_str().unwrap_or("");
    if thread_id.is_empty() {
        eprintln!("❌ No active thread. Use `fur new` or `fur thread`.");
        return;
    }

    let thread_path = fur_dir.join("threads").join(format!("{}.json", thread_id));
    let thread_content = fs::read_to_string(&thread_path).expect("❌ Cannot read thread");
    let thread_json: Value = serde_json::from_str(&thread_content).unwrap();
    let thread_title = thread_json["title"].as_str().unwrap_or("Untitled");

    // Decide where to write
    let mut writer: Box<dyn Write> = if let Some(Some(path)) = &args.out {
        Box::new(File::create(path).expect("❌ Cannot create output file"))
    } else {
        Box::new(io::stdout())
    };

    if args.out.is_some() {
        writeln!(writer, "# {}", thread_title).ok();
        writeln!(writer).ok();
    } else {
        println!("{} {}", "Thread:".cyan().bold(), thread_title.green());
        println!();
    }

    // Iterate over root messages
    let empty_vec: Vec<Value> = Vec::new();
    let root_msgs = thread_json["messages"].as_array().unwrap_or(&empty_vec);
    for mid in root_msgs {
        if let Some(mid_str) = mid.as_str() {
            render_message(&fur_dir, mid_str, "Root".to_string(), &args, &avatars, &mut writer);
        }
    }
}

/// Render a message and its children/branches
fn render_message(
    fur_dir: &Path,
    msg_id: &str,
    label: String,
    args: &TimelineArgs,
    avatars: &Value,
    writer: &mut dyn Write,
) {
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
    let (date_str, time_str, branch_info) = if let Ok(dt) = raw_time.parse::<DateTime<FixedOffset>>() {
        let local_dt = dt.with_timezone(&Local); // convert to local timezone
        (
            local_dt.format("%Y-%m-%d").to_string(),
            local_dt.format("%H:%M:%S").to_string(),
            format!("— {}", label),
        )
    } else {
        (raw_time.to_string(), "".to_string(), format!("— {}", label))
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

    if args.out.is_some() {
        // Markdown/script style output
        if msg_json["markdown"].is_null() {
            writeln!(writer, "**{}:** {}", name, text).ok();
            writeln!(writer, "<sub>{} {} {}</sub>", date_str, time_str, branch_info).ok();
            writeln!(writer).ok();
        } else {
            if let Some(path_str) = msg_json["markdown"].as_str() {
                writeln!(writer, "(Attached document: `{}`)", path_str).ok();
                writeln!(writer).ok();
                if args.verbose {
                    if let Ok(contents) = fs::read_to_string(path_str) {
                        writeln!(writer, "{}", contents).ok();
                        writeln!(writer, "\n---\n").ok();
                    }
                }
            }
        }
    } else {
        // Normal terminal colored output
        println!(
            "{}  {} {} {} {} {}:",
            "🕰️".cyan().bold(),
            date_str.cyan(),
            time_str.bright_cyan().bold(),
            format!("[{}]", label).bright_green(),
            format!("{} [{}]", emoji.yellow(), name.yellow()),
            "",
        );
        println!("{}\n", text.white());

        if let Some(path_str) = msg_json["markdown"].as_str() {
            if args.verbose {
                println!("📂 Linked Markdown Content:");
                if let Ok(contents) = fs::read_to_string(path_str) {
                    println!("{}", contents);
                }
            } else {
                println!("📂 Linked Markdown file: {}", path_str);
            }
        }
    }

    // Branch recursion
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
                            render_message(fur_dir, c_id, new_label, args, avatars, writer);
                        }
                    }
                }
            }
            return;
        }
    }

    // Legacy fallback: children
    if let Some(children) = msg_json["children"].as_array() {
        for child_id in children {
            if let Some(c_id) = child_id.as_str() {
                render_message(fur_dir, c_id, label.clone(), args, avatars, writer);
            }
        }
    }
}
