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

    if let Some(att) = msg.attachment {
        if att.ends_with(".png")
            || att.ends_with(".jpg")
            || att.ends_with(".jpeg")
            || att.ends_with(".gif")
        {
            out.push_str(&format!("\n![attachment]({})\n\n", att));
        } else if att.ends_with(".pdf") {
            // Markdown can't inline PDFs → make it a link
            out.push_str(&format!(
                "\n[Attached PDF: {}]({})\n\n",
                Path::new(&att).file_name().unwrap().to_string_lossy(),
                att
            ));
        } else {
            out.push_str(&format!("\n[Attachment: {}]\n\n", att));
        }
    }

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
