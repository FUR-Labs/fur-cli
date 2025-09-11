use std::fs;
use std::path::Path;
use serde_json::Value;

use crate::commands::timeline::TimelineArgs;
use crate::renderer::utils::load_message;

pub fn render_message_md(
    fur_dir: &Path,
    msg_id: &str,
    label: String,
    args: &TimelineArgs,
    avatars: &Value,
    out: &mut String,
) {
    let Some(msg) = load_message(fur_dir, msg_id, avatars) else { return };

    out.push_str(&format!("**{} [{}]:** {}\n", msg.name, msg.emoji, msg.text));
    out.push_str(&format!("_{} {} - {}_\n\n", msg.date_str, msg.time_str, label));

    if args.verbose || args.contents {
        if let Some(path_str) = msg.markdown {
            if let Ok(contents) = fs::read_to_string(path_str) {
                out.push_str(&format!("\n{}\n", contents));
            }
        }
    }

    // ✅ Correct branch numbering: one label per branch block
    for (bi, block) in msg.branches.iter().enumerate() {
        let branch_label = format!("{} - Branch {}", label, bi + 1);

        for cid in block {
            render_message_md(fur_dir, cid, branch_label.clone(), args, avatars, out);
        }
    }
}
