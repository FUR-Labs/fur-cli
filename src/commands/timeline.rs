use std::fs;
use std::path::Path;
use serde_json::{Value, json};
use chrono::{DateTime, FixedOffset, Local};
use colored::*;
use clap::Parser;
use genpdf::{Document, Element, elements, fonts, style};

use crate::frs::avatars::resolve_avatar;

/// Args for timeline command
#[derive(Parser)]
pub struct TimelineArgs {
    /// Whether to show full content of Markdown files
    #[arg(short, long)]
    pub verbose: bool,

    /// Alias for --verbose
    #[arg(long)]
    pub contents: bool,

    /// Write output to a file (Markdown or PDF, depending on extension)
    #[arg(long)]
    pub out: Option<String>,
}

fn embedded_font_family() -> fonts::FontFamily<fonts::FontData> {
    let try_load = |bytes: &'static [u8]| {
        fonts::FontData::new(bytes.to_vec(), None).expect("Failed to load embedded font")
    };

    fonts::FontFamily {
        regular: try_load(include_bytes!("../../fonts/LiberationSans-Regular.ttf")),
        bold: try_load(include_bytes!("../../fonts/LiberationSans-Bold.ttf")),
        italic: try_load(include_bytes!("../../fonts/LiberationSans-Italic.ttf")),
        bold_italic: try_load(include_bytes!("../../fonts/LiberationSans-BoldItalic.ttf")),
    }
}


/// Main entry for timeline
pub fn run_timeline(args: TimelineArgs) {
    let fur_dir = Path::new(".fur");
    let index_path = fur_dir.join("index.json");
    if !index_path.exists() {
        eprintln!("🚨 .fur/ not found. Run `fur new` first.");
        return;
    }

    let index: Value = serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();
    let thread_id = index["active_thread"].as_str().unwrap_or("");
    let thread_path = fur_dir.join("threads").join(format!("{}.json", thread_id));
    let thread_json: Value = serde_json::from_str(&fs::read_to_string(&thread_path).unwrap()).unwrap();

    let thread_title = thread_json["title"].as_str().unwrap_or("Untitled");

    let avatars: Value = serde_json::from_str(
        &fs::read_to_string(fur_dir.join("avatars.json")).unwrap_or_else(|_| "{}".to_string())
    ).unwrap_or(json!({}));

    let empty_vec: Vec<Value> = Vec::new();
    let root_msgs = thread_json["messages"].as_array().unwrap_or(&empty_vec);

    if let Some(path) = &args.out {
        if path.ends_with(".pdf") {
            // ✅ PDF MODE with embedded fonts
            let font_family = embedded_font_family();
            let mut doc = Document::new(font_family);
            doc.set_title(thread_title);

            doc.push(
                elements::Paragraph::new(format!("# {}", thread_title))
                    .aligned(genpdf::Alignment::Center)
                    .styled(style::Style::new().bold()),
            );

            for mid in root_msgs {
                if let Some(mid_str) = mid.as_str() {
                    render_message_pdf(&fur_dir, mid_str, "Root".to_string(), &args, &avatars, &mut doc);
                }
            }

            doc.render_to_file(path).expect("❌ Failed writing PDF");
            println!("✔️ Timeline exported to {}", path);
            return;
        }

        // Markdown mode
        let mut out_content = String::new();
        out_content.push_str(&format!("# {}\n\n", thread_title));

        for mid in root_msgs {
            if let Some(mid_str) = mid.as_str() {
                render_message_md(&fur_dir, mid_str, "Root".to_string(), &args, &avatars, &mut out_content);
            }
        }

        fs::write(path, out_content).expect("❌ Failed writing Markdown file");
        println!("✔️ Timeline exported to {}", path);
        return;
    }

    // Terminal mode
    println!("Thread: {}", thread_title);
    for mid in root_msgs {
        if let Some(mid_str) = mid.as_str() {
            render_message(&fur_dir, mid_str, "Root".to_string(), &args, &avatars);
        }
    }
}

/// Terminal renderer
fn render_message(fur_dir: &Path, msg_id: &str, label: String, args: &TimelineArgs, avatars: &Value) {
    let msg_path = fur_dir.join("messages").join(format!("{}.json", msg_id));
    let Ok(msg_content) = fs::read_to_string(&msg_path) else { return };
    let Ok(msg_json) = serde_json::from_str::<Value>(&msg_content) else { return };

    // Timestamp → local
    let raw_time = msg_json["timestamp"].as_str().unwrap_or("???");
    let (date_str, time_str) = if let Ok(dt) = raw_time.parse::<DateTime<FixedOffset>>() {
        let local_dt = dt.with_timezone(&Local);
        (local_dt.format("%Y-%m-%d").to_string(), local_dt.format("%H:%M:%S").to_string())
    } else {
        (raw_time.to_string(), "".to_string())
    };

    let avatar_key = msg_json["avatar"].as_str().unwrap_or("???");
    let (name, emoji) = resolve_avatar(avatars, avatar_key);
    let text = msg_json["text"].as_str().unwrap_or("<no content>");

    println!(
        "{} {} - {} [{}] {}:",
        date_str.cyan(),
        time_str.bright_cyan().bold(),
        label.green(),
        emoji,
        name.yellow(),
    );
    println!("{}\n", text.white());

    if args.verbose || args.contents {
        if let Some(path_str) = msg_json["markdown"].as_str() {
            if let Ok(contents) = fs::read_to_string(path_str) {
                println!("{}", contents);
            }
        }
    }

    if let Some(children) = msg_json["children"].as_array() {
        for c in children {
            if let Some(cid) = c.as_str() {
                render_message(fur_dir, cid, label.clone(), args, avatars);
            }
        }
    }
}

/// Markdown renderer
fn render_message_md(
    fur_dir: &Path,
    msg_id: &str,
    label: String,
    args: &TimelineArgs,
    avatars: &Value,
    out: &mut String,
) {
    let msg_path = fur_dir.join("messages").join(format!("{}.json", msg_id));
    let Ok(msg_content) = fs::read_to_string(&msg_path) else { return };
    let Ok(msg_json) = serde_json::from_str::<Value>(&msg_content) else { return };

    let raw_time = msg_json["timestamp"].as_str().unwrap_or("???");
    let (date_str, time_str) = if let Ok(dt) = raw_time.parse::<DateTime<FixedOffset>>() {
        let local_dt = dt.with_timezone(&Local);
        (local_dt.format("%Y-%m-%d").to_string(), local_dt.format("%H:%M:%S").to_string())
    } else {
        (raw_time.to_string(), "".to_string())
    };

    let avatar_key = msg_json["avatar"].as_str().unwrap_or("???");
    let (name, _emoji) = resolve_avatar(avatars, avatar_key);
    let text = msg_json["text"].as_str().unwrap_or("<no content>");

    out.push_str(&format!("**{}:** {}\n", name, text));
    out.push_str(&format!("_{} {} - {}_\n\n", date_str, time_str, label));

    if args.verbose || args.contents {
        if let Some(path_str) = msg_json["markdown"].as_str() {
            if let Ok(contents) = fs::read_to_string(path_str) {
                out.push_str(&format!("\n{}\n", contents));
            }
        }
    }

    if let Some(children) = msg_json["children"].as_array() {
        for c in children {
            if let Some(cid) = c.as_str() {
                render_message_md(fur_dir, cid, label.clone(), args, avatars, out);
            }
        }
    }
}

/// PDF renderer
fn render_message_pdf(
    fur_dir: &Path,
    msg_id: &str,
    label: String,
    args: &TimelineArgs,
    avatars: &Value,
    doc: &mut Document,
) {
    let msg_path = fur_dir.join("messages").join(format!("{}.json", msg_id));
    let Ok(msg_content) = fs::read_to_string(&msg_path) else { return };
    let Ok(msg_json) = serde_json::from_str::<Value>(&msg_content) else { return };

    let raw_time = msg_json["timestamp"].as_str().unwrap_or("???");
    let (date_str, time_str) = if let Ok(dt) = raw_time.parse::<DateTime<FixedOffset>>() {
        let local_dt = dt.with_timezone(&Local);
        (local_dt.format("%Y-%m-%d").to_string(), local_dt.format("%H:%M:%S").to_string())
    } else {
        (raw_time.to_string(), "".to_string())
    };

    let avatar_key = msg_json["avatar"].as_str().unwrap_or("???");
    let (name, _emoji) = resolve_avatar(avatars, avatar_key);
    let text = msg_json["text"].as_str().unwrap_or("<no content>");

    doc.push(
        elements::Paragraph::new(format!("{}: {}", name, text))
            .styled(style::Style::new().bold()),
    );
    doc.push(
        elements::Paragraph::new(format!("{} {} - {}", date_str, time_str, label))
            .styled(
                style::Style::new()
                    .italic()
                    .with_color(style::Color::Rgb(150, 150, 150)),
            ),
    );

    if args.verbose || args.contents {
        if let Some(path_str) = msg_json["markdown"].as_str() {
            if let Ok(contents) = fs::read_to_string(path_str) {
                doc.push(elements::Paragraph::new(contents));
            }
        }
    }

    if let Some(children) = msg_json["children"].as_array() {
        for c in children {
            if let Some(cid) = c.as_str() {
                render_message_pdf(fur_dir, cid, label.clone(), args, avatars, doc);
            }
        }
    }
}
