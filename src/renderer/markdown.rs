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

    out.push_str(&format!("**{}:** {}\n", msg.name, msg.text));
    out.push_str(&format!("_{} {} - {}_\n\n", msg.date_str, msg.time_str, label));

    if args.verbose || args.contents {
        if let Some(path_str) = msg.markdown {
            if let Ok(contents) = fs::read_to_string(path_str) {
                out.push_str(&format!("\n{}\n", contents));
            }
        }
    }

    for cid in msg.children {
        render_message_md(fur_dir, &cid, label.clone(), args, avatars, out);
    }
}
