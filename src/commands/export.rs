use std::path::Path;

use clap::Parser;
use colored::*;
use serde_json::Value;

use crate::schema::bridge::{document_from_thread, linked_sources, write_conversation_folder};

/// Arguments for `fur export`
#[derive(Parser, Debug)]
pub struct ExportArgs {
    /// Conversation ID or prefix (defaults to the active conversation)
    #[arg(short, long)]
    pub id: Option<String>,

    /// Export every conversation in the project
    #[arg(long)]
    pub all: bool,

    /// Overwrite an existing convo.md
    #[arg(long)]
    pub force: bool,
}

/// `fur export` — write conversations out as canonical Markdown documents.
///
/// Phase A: this is additive. `.fur/` remains the source of truth and is left
/// untouched; the export exists so the document format can be checked against
/// real conversations before anything depends on it.
pub fn run_export(args: ExportArgs) {
    let fur_dir = Path::new(".fur");
    let index_path = fur_dir.join("index.json");

    if !index_path.exists() {
        eprintln!("🚨 .fur/ not found. Run `fur new` first.");
        return;
    }

    let Some(content) = crate::security::io::read_text_file(&index_path) else {
        return;
    };

    let index: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(e) => return eprintln!("❌ Invalid index.json: {}", e),
    };

    let threads: Vec<String> = index["threads"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let targets = match select_targets(&index, &threads, &args) {
        Some(t) => t,
        None => return,
    };

    let project_root = Path::new(".");
    let mut exported = 0usize;

    for tid in &targets {
        match export_one(fur_dir, project_root, tid, args.force) {
            Ok(folder) => {
                println!("📄 {}", folder.display().to_string().green());
                exported += 1;
            }
            Err(e) => eprintln!("❌ {} — {}", &short(tid).bright_black(), e),
        }
    }

    println!(
        "\n✔️ Exported {} of {} conversation(s).",
        exported,
        targets.len()
    );
}

fn export_one(
    fur_dir: &Path,
    project_root: &Path,
    tid: &str,
    force: bool,
) -> Result<std::path::PathBuf, String> {
    let doc = document_from_thread(fur_dir, tid)?;
    let linked = linked_sources(fur_dir, tid)?;

    write_conversation_folder(project_root, &doc, &linked, force)
}

/// Resolve which conversations to export: an explicit prefix, everything, or
/// the active conversation.
fn select_targets(index: &Value, threads: &[String], args: &ExportArgs) -> Option<Vec<String>> {
    if args.all {
        if threads.is_empty() {
            eprintln!("⚠️ No conversations to export.");
            return None;
        }
        return Some(threads.to_vec());
    }

    if let Some(prefix) = &args.id {
        let matches: Vec<&String> = threads.iter().filter(|t| t.starts_with(prefix)).collect();

        return match matches.as_slice() {
            [] => {
                eprintln!("❌ No conversation matches '{}'", prefix);
                None
            }
            [single] => Some(vec![(*single).clone()]),
            _ => {
                eprintln!("❌ Ambiguous prefix '{}': {:?}", prefix, matches);
                None
            }
        };
    }

    match index["active_thread"].as_str() {
        Some(active) if !active.is_empty() => Some(vec![active.to_string()]),
        _ => {
            eprintln!("❌ No active conversation. Use --id or --all.");
            None
        }
    }
}

fn short(id: &str) -> &str {
    if id.len() >= 8 {
        &id[..8]
    } else {
        id
    }
}