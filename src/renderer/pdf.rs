use std::fs;
use std::path::Path;
use serde_json::Value;
use genpdf::{Document, elements, style, Element};

use crate::commands::timeline::TimelineArgs;
use crate::renderer::utils::load_message;

pub fn render_message_pdf(
    fur_dir: &Path,
    msg_id: &str,
    label: String,
    args: &TimelineArgs,
    avatars: &Value,
    doc: &mut Document,
) {
    let Some(msg) = load_message(fur_dir, msg_id, avatars) else { return };

    doc.push(
        elements::Paragraph::new(format!("{}: {}", msg.name, msg.text))
            .styled(style::Style::new().bold().with_font_size(12)),
    );
    doc.push(
        elements::Paragraph::new(format!("{} {} - {}", msg.date_str, msg.time_str, label))
            .styled(style::Style::new().italic().with_color(style::Color::Rgb(120, 120, 120))),
    );
    doc.push(elements::Break::new(1));

    if args.verbose || args.contents {
        if let Some(path_str) = msg.markdown {
            if let Ok(contents) = fs::read_to_string(path_str) {
                doc.push(elements::Paragraph::new(contents));
            }
        }
    }

    for cid in msg.children {
        render_message_pdf(fur_dir, &cid, label.clone(), args, avatars, doc);
    }
}
