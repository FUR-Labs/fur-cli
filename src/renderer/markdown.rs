use std::fs;
use std::path::Path;
use serde_json::Value;

use crate::commands::timeline::TimelineArgs;
use crate::renderer::utils::load_message;

/// Escape % inside math blocks: $ ... % ... $ → $ ... \% ... $
fn escape_percent_in_math(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    let mut in_math = false;

    while let Some(c) = chars.next() {
        if c == '$' {
            // Toggle math mode
            in_math = !in_math;
            out.push('$');
            continue;
        }

        if in_math && c == '%' {
            out.push_str("\\%");
            continue;
        }

        out.push(c);
    }

    out
}

/// Convert isolated [ ... ] math blocks into $ ... $ and then escape % inside them.
fn convert_bracket_math_block(text: &str) -> String {
    let mut out = String::new();
    let mut lines = text.lines().peekable();

    while let Some(line) = lines.next() {
        if line.trim() == "[" {
            let mut block = String::new();

            while let Some(inner) = lines.next() {
                if inner.trim() == "]" {
                    break;
                }
                block.push_str(inner);
                block.push('\n');
            }

            // Wrap with math and escape %
            let math = format!("$\n{}$", escape_percent_in_math(&block));
            out.push_str(&math);
            out.push('\n');
            continue;
        }

        out.push_str(line);
        out.push('\n');
    }

    // After processing bracket-blocks, escape % in inline math too
    escape_percent_in_math(&out)
}

pub fn render_message_to_md(
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
                // 🔥 Math-block conversion applied here
                let converted = convert_bracket_math_block(&contents);
                out.push_str(&format!("\n{}\n", converted));

            }
        }
    }

    // Branches
    for (bi, block) in msg.branches.iter().enumerate() {
        let branch_label = format!("{} - Branch {}", label, bi + 1);
        for cid in block {
            render_message_to_md(fur_dir, cid, branch_label.clone(), args, avatars, out);
        }
    }
}
