use std::fs;
use std::path::Path;
use clap::Args;
use crate::utils::load_json;

#[derive(Args)]
pub struct CatArgs {
    /// Optional ID to show a specific message
    #[arg(short, long)]
    pub id: Option<String>,
}

pub fn run_cat(args: CatArgs) {
    let index = load_json(".fur/index.json");
    let message_id = args.id.unwrap_or_else(|| {
        index["current_message"].as_str().unwrap().to_string()
    });

    let msg_path = Path::new(".fur/messages").join(format!("{}.json", message_id));
    if !msg_path.exists() {
        eprintln!("❌ No such message found: {}", message_id);
        return;
    }

    let msg = load_json(msg_path.to_str().unwrap());

    if let Some(path_str) = msg["markdown"].as_str() {
        if let Ok(contents) = fs::read_to_string(path_str) {
            println!("📄 Contents of {}\n{}", path_str, contents);
        } else {
            eprintln!("⚠️ Could not read linked markdown file: {}", path_str);
        }
    } else {
        eprintln!("❌ This message has no linked markdown file.");
    }
}
