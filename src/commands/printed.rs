use std::fs;
use std::path::Path;
use serde_json::Value;
use colored::*;
use crate::commands::timeline::{run_timeline, TimelineArgs};

/// `fur printed` — exports the active conversation to Markdown or PDF
pub fn run_printed(out: Option<String>, verbose: bool) {
    let fur_dir = Path::new(".fur");
    let index_path = fur_dir.join("index.json");

    if !index_path.exists() {
        eprintln!("🚨 .fur/ not found. Run `fur new` first.");
        return;
    }

    // --- Load index and get active conversation ID ---
    let index: Value = serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();
    let active_id = index["active_thread"].as_str().unwrap_or_default();
    if active_id.is_empty() {
        eprintln!("❌ No active conversation found.");
        return;
    }

    // --- Load active conversation metadata ---
    let convo_path = fur_dir.join("threads").join(format!("{}.json", active_id));
    let conversation_json: Value =
        serde_json::from_str(&fs::read_to_string(&convo_path).unwrap()).unwrap();

    let title = conversation_json["title"].as_str().unwrap_or("untitled");
    let id = conversation_json["id"].as_str().unwrap_or("unknown");

    // --- Determine output file path ---
    let out_path = match out {
        Some(p) => p,
        None => {
            // Default: ALLCAPS_TITLE_SHORTID.md
            let all_caps = title.to_uppercase().replace(' ', "_");

            // Grab only first part of the UUID before '-'
            let short_id = id.split('-').next().unwrap_or(id);

            format!("{}_{}.md", all_caps, short_id)
        }
    };


    // --- Auto-detect output type ---
    let lower_out = out_path.to_lowercase();
    let is_pdf = lower_out.ends_with(".pdf");

    // --- Build timeline args ---
    let args = TimelineArgs {
        verbose,
        contents: true,
        out: Some(out_path.clone()),
        conversation_override: None,
    };

    // --- Logging ---
    println!(
        "{}",
        format!(
            "🖨️  Printing conversation: {} ({}) → {}",
            title, id, out_path
        )
        .bright_green()
        .bold()
    );

    // --- Run export ---
    run_timeline(args);

    println!(
        "{}",
        if is_pdf {
            format!("✔️  Exported PDF: {}", out_path)
        } else {
            format!("✔️  Exported Markdown: {}", out_path)
        }
        .bright_green()
        .bold()
    );
}
