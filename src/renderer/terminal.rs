use std::fs;
use std::path::Path;
use serde_json::Value;
use colored::*;

use crate::commands::timeline::TimelineArgs;
use crate::renderer::utils::load_message;

pub fn render_message(
    fur_dir: &Path,
    msg_id: &str,
    label: String,
    args: &TimelineArgs,
    avatars: &Value,
) {
    let Some(msg) = load_message(fur_dir, msg_id, avatars) else { return };

    println!(
        "{} {} - {} [{}] {}:",
        msg.date_str.cyan(),
        msg.time_str.bright_cyan().bold(),
        label.bright_green(),
        msg.emoji,
        msg.name.bright_yellow(),
    );
    println!("{}\n", msg.text.white());

    if args.verbose || args.contents {
        if let Some(path_str) = msg.markdown {
            if let Ok(contents) = fs::read_to_string(path_str) {
                println!("{}", contents);
            }
        }
    }

    // ✅ Branch-aware recursion: one label per branch block
    for (bi, block) in msg.branches.iter().enumerate() {
        let branch_label = format!("{} - Branch {}", label, bi + 1);

        for cid in block {
            render_message(fur_dir, cid, branch_label.clone(), args, avatars);
        }
    }
}
