use std::fs;
use std::path::Path;
use serde_json::{Value, json};
use clap::Parser;

use crate::renderer::{
    terminal::render_message,
    markdown::render_message_to_md,
    pdf::export_convo_to_pdf,
};


/// Args for timeline command
#[derive(Parser, Clone)]
pub struct TimelineArgs {
    #[arg(short, long)]
    pub verbose: bool,
    #[arg(long)]
    pub contents: bool,
    #[arg(long)]
    pub out: Option<String>,

    #[clap(skip)]
    pub conversation_override: Option<String>,
}


/// Main entry for timeline
pub fn run_timeline(args: TimelineArgs) {
    let fur_dir = Path::new(".fur");
    let index_path = fur_dir.join("index.json");
    if !index_path.exists() {
        eprintln!("🚨 .fur/ not found. Run `fur new` first.");
        return;
    }

    // Load conversation metadata
    let index: Value = serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();
    let conversation_id = if let Some(ref override_id) = args.conversation_override {
        override_id
    } else {
        index["active_thread"].as_str().unwrap_or("")
    };

    let conversation_path = fur_dir.join("threads").join(format!("{}.json", conversation_id));
    let conversation_json: Value = serde_json::from_str(&fs::read_to_string(&conversation_path).unwrap()).unwrap();

    let conversation_title = conversation_json["title"].as_str().unwrap_or("Untitled");

    // Load avatars
    let avatars: Value = serde_json::from_str(
        &fs::read_to_string(fur_dir.join("avatars.json"))
            .unwrap_or_else(|_| "{}".to_string())
    ).unwrap_or(json!({}));

    // Root messages (ids only)
    let empty_vec: Vec<Value> = Vec::new();
    let root_msgs = conversation_json["messages"].as_array().unwrap_or(&empty_vec);

    // --- PDF mode
    if let Some(path) = &args.out {
        if path.ends_with(".pdf") {
            export_convo_to_pdf(&fur_dir, conversation_title, root_msgs, &args, &avatars, path);
            return;
        }


        // --- Markdown mode
        let mut out_content = String::new();
        out_content.push_str(&format!("# {}\n\n", conversation_title));

        for mid in root_msgs {
            if let Some(mid_str) = mid.as_str() {
                render_message_to_md(&fur_dir, mid_str, "Root".to_string(), &args, &avatars, &mut out_content);
            }
        }

        let word_count = out_content.split_whitespace().count();
        let token_est = word_count * 4 / 3;

        let mut final_output = String::new();
        final_output.push_str(&format!(
            "> ✍️ Words: {}\n> 🪙 Tokens (est.): {}\n\n",
            word_count, token_est
        ));
        final_output.push_str(&out_content);

        fs::write(path, final_output).expect("❌ Failed writing Markdown file");

        println!("✔️ Timeline exported to {}", path);
        return;
    }

    // --- Terminal mode
    println!("Thread: {}", conversation_title);
    for mid in root_msgs {
        if let Some(mid_str) = mid.as_str() {
            render_message(&fur_dir, mid_str, "Root".to_string(), &args, &avatars);
        }
    }
}
