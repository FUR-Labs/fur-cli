use clap::Parser;
use std::fs;
use std::path::Path;
use serde_json::Value;

/// Arguments for the `save` subcommand
#[derive(Parser)]
pub struct SaveArgs {
    /// Output path for the .frs file
    #[arg(short, long)]
    pub out: Option<String>,
}

/// Save the active thread back into a .frs file
pub fn run_save(args: SaveArgs) {
    let fur_dir = Path::new(".fur");
    let index_path = fur_dir.join("index.json");

    if !index_path.exists() {
        eprintln!("🚨 .fur/ not found. Run `fur new` first.");
        return;
    }

    let index: Value =
        serde_json::from_str(&fs::read_to_string(&index_path).expect("❌ Cannot read index.json"))
            .unwrap();

    let thread_id = match index["active_thread"].as_str() {
        Some(id) => id,
        None => {
            eprintln!("⚠️ No active thread.");
            return;
        }
    };

    let thread_path = fur_dir.join("threads").join(format!("{}.json", thread_id));
    let thread: Value =
        serde_json::from_str(&fs::read_to_string(&thread_path).expect("❌ Cannot read thread"))
            .unwrap();

    let title = thread["title"].as_str().unwrap_or("Untitled");
    let safe_title = title.replace(" ", "_");

    let output_path = args
        .out
        .unwrap_or_else(|| format!("{}.frs", safe_title));

    let mut out = String::new();

    // ---- header
    out.push_str(&format!("new \"{}\"\n", title));
    if let Some(tags) = thread["tags"].as_array() {
        if !tags.is_empty() {
            let tags_str = tags
                .iter()
                .filter_map(|t| t.as_str())
                .map(|t| format!("\"{}\"", t))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("tags = [{}]\n\n", tags_str));
        }
    }

    // ---- messages (recursive)
    for msg_id in thread["messages"].as_array().unwrap_or(&vec![]) {
        if let Some(mid) = msg_id.as_str() {
            out.push_str(&render_message(mid, 0, fur_dir));
        }
    }

    fs::write(&output_path, out).expect("❌ Could not write .frs file");
    println!("💾 Saved thread \"{}\" to {}", title, output_path);
}

fn render_message(msg_id: &str, indent: usize, fur_dir: &Path) -> String {
    let msg_path = fur_dir.join("messages").join(format!("{}.json", msg_id));
    let content = match fs::read_to_string(&msg_path) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    let msg: Value = serde_json::from_str(&content).unwrap();

    let mut out = String::new();
    let pad = "    ".repeat(indent);

    let avatar = msg["avatar"].as_str().unwrap_or("anon");

    if let Some(text) = msg["text"].as_str() {
        out.push_str(&format!("{}jot {} \"{}\"\n", pad, avatar, text));
    } else if let Some(file) = msg["markdown"].as_str() {
        out.push_str(&format!("{}jot {} --file \"{}\"\n", pad, avatar, file));
    } else if let Some(att) = msg["attachment"].as_str() {
        out.push_str(&format!("{}jot {} --img \"{}\"\n", pad, avatar, att));
    }

    if let Some(branches) = msg["branches"].as_array() {
        for block in branches {
            if let Some(arr) = block.as_array() {
                out.push_str(&format!("{}branch {{\n", pad));
                for child in arr {
                    if let Some(cid) = child.as_str() {
                        out.push_str(&render_message(cid, indent + 1, fur_dir));
                    }
                }
                out.push_str(&format!("{}}}\n", pad));
            }
        }
    }

    out
}
