use std::fs;
use std::path::Path;
use serde_json::{Value, json};
use clap::Parser;
use genpdf::{Document, elements, fonts, style, Element};

use crate::renderer::{
    terminal::render_message,
    markdown::render_message_md,
    pdf::render_message_pdf,
};

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

/// Load embedded LiberationSans font family
fn embedded_font_family() -> fonts::FontFamily<fonts::FontData> {
    let try_load = |bytes: &'static [u8]| {
        fonts::FontData::new(bytes.to_vec(), None)
            .expect("Failed to load embedded font")
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

    // Load thread metadata
    let index: Value = serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();
    let thread_id = index["active_thread"].as_str().unwrap_or("");
    let thread_path = fur_dir.join("threads").join(format!("{}.json", thread_id));
    let thread_json: Value = serde_json::from_str(&fs::read_to_string(&thread_path).unwrap()).unwrap();

    let thread_title = thread_json["title"].as_str().unwrap_or("Untitled");

    // Load avatars
    let avatars: Value = serde_json::from_str(
        &fs::read_to_string(fur_dir.join("avatars.json"))
            .unwrap_or_else(|_| "{}".to_string())
    ).unwrap_or(json!({}));

    // Root messages (ids only)
    let empty_vec: Vec<Value> = Vec::new();
    let root_msgs = thread_json["messages"].as_array().unwrap_or(&empty_vec);

    // --- PDF mode
    if let Some(path) = &args.out {
        if path.ends_with(".pdf") {
            let font_family = embedded_font_family();
            let mut doc = Document::new(font_family);
            doc.set_title(thread_title);

            doc.push(
                elements::Paragraph::new(thread_title)
                    .aligned(genpdf::Alignment::Center)
                    .styled(style::Style::new().bold().with_font_size(20)),
            );
            doc.push(elements::Break::new(1));

            for mid in root_msgs {
                if let Some(mid_str) = mid.as_str() {
                    // delegate rendering
                    render_message_pdf(&fur_dir, mid_str, "Root".to_string(), &args, &avatars, &mut doc);
                }
            }

            doc.render_to_file(path).expect("❌ Failed writing PDF");
            println!("✔️ Timeline exported to {}", path);
            return;
        }

        // --- Markdown mode
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

    // --- Terminal mode
    println!("Thread: {}", thread_title);
    for mid in root_msgs {
        if let Some(mid_str) = mid.as_str() {
            render_message(&fur_dir, mid_str, "Root".to_string(), &args, &avatars);
        }
    }
}
