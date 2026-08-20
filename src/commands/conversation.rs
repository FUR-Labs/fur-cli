use crate::helpers::conversation::{
    confirm_delete_destructive, confirm_delete_primary, perform_conversation_deletion,
    resolve_target_thread_id,
};
use crate::helpers::tags::parse_tag_list;
use crate::renderer::table::render_table;
use crate::schema::lineage::Lineage;
use chrono::{DateTime, Local, Utc};
use clap::Parser;
use colored::*;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Arguments for the `conversation` command
#[derive(Parser)]
pub struct ThreadArgs {
    /// Thread ID or prefix to switch
    pub id: Option<String>,

    /// View all threads
    #[arg(long)]
    pub view: bool,

    /// Rename
    #[arg(long, alias = "rn")]
    pub rename: Option<String>,

    /// Add tags (comma-separated, supports spaces)
    #[arg(long)]
    pub tag: Option<String>,

    #[arg(long)]
    pub untag: Option<String>,

    /// Clear all tags from conversation
    #[arg(long)]
    pub clear_tags: bool,

    /// Delete a conversation (destructive)
    #[arg(long)]
    pub delete: bool,

    /// Show all conversations (no truncation)
    #[arg(long, short = 'a')]
    pub all: bool,

    /// List conversations newest-first, ignoring lineage
    #[arg(long)]
    pub flat: bool,
}

/// One conversation's display data, gathered once and reused by both views.
struct ConvoInfo {
    id: String,
    title: String,
    created: DateTime<Utc>,
    date_str: String,
    time_str: String,
    msg_count: usize,
    size_bytes: u64,
    tags: String,
}

pub fn run_conversation(args: ThreadArgs) {
    let fur_dir = Path::new(".fur");
    let index_path = fur_dir.join("index.json");

    if !index_path.exists() {
        eprintln!("🚨 .fur/ not found. Run `fur new` first.");
        return;
    }

    let mut index: Value = serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();

    if args.tag.is_some() || args.untag.is_some() || args.clear_tags {
        return handle_tagging(&args, &mut index, fur_dir);
    }

    if args.rename.is_some() {
        return handle_rename_thread(&mut index, fur_dir, &args);
    }

    if args.delete {
        return handle_delete_thread(&mut index, fur_dir, &args);
    }

    if args.view || args.id.is_none() {
        return handle_view_threads(&index, fur_dir, &args);
    }

    if args.id.is_some() {
        return handle_switch_thread(&mut index, &index_path, fur_dir, &args);
    }
}

fn handle_rename_thread(index: &mut Value, fur_dir: &Path, args: &ThreadArgs) {
    let new_title = match &args.rename {
        Some(t) => t,
        None => return,
    };

    let empty_vec: Vec<Value> = Vec::new();
    let threads: Vec<String> = index["threads"]
        .as_array()
        .unwrap_or(&empty_vec)
        .iter()
        .filter_map(|t| t.as_str().map(|s| s.to_string()))
        .collect();

    // CASE 1: rename current thread
    let target_thread_id = if args.id.is_none() {
        index["active_thread"].as_str().unwrap_or("").to_string()
    } else {
        // CASE 2: rename by prefix
        let prefix = args.id.as_ref().unwrap();
        let found = threads
            .iter()
            .filter(|tid| tid.starts_with(prefix))
            .collect::<Vec<_>>();

        if found.is_empty() {
            eprintln!("❌ No conversation matches prefix '{}'", prefix);
            return;
        }
        if found.len() > 1 {
            eprintln!("❌ Ambiguous prefix '{}'. Matches: {:?}", prefix, found);
            return;
        }

        found[0].to_string()
    };

    let convo_path = fur_dir
        .join("threads")
        .join(format!("{}.json", target_thread_id));
    let mut conversation_json: Value =
        serde_json::from_str(&fs::read_to_string(&convo_path).unwrap()).unwrap();

    let old_title = conversation_json["title"]
        .as_str()
        .unwrap_or("Untitled")
        .to_string();

    // Update title
    conversation_json["title"] = Value::String(new_title.to_string());
    fs::write(
        &convo_path,
        serde_json::to_string_pretty(&conversation_json).unwrap(),
    )
    .unwrap();

    println!(
        "✏️  Renamed conversation {} \"{}\" → \"{}\"",
        &target_thread_id[..8],
        old_title,
        new_title
    );

    crate::schema::bridge::sync_active();
}

fn handle_delete_thread(index: &mut Value, fur_dir: &Path, args: &ThreadArgs) {
    let target_tid = match resolve_target_thread_id(index, args) {
        Some(tid) => tid,
        None => return,
    };

    // extract all thread IDs for later index update
    let empty_vec: Vec<Value> = Vec::new();
    let threads: Vec<String> = index["threads"]
        .as_array()
        .unwrap_or(&empty_vec)
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    if !confirm_delete_primary() {
        println!("❌ Deletion aborted.");
        return;
    }

    if !confirm_delete_destructive() {
        println!("❌ Deletion aborted.");
        return;
    }

    perform_conversation_deletion(index, fur_dir, &target_tid, &threads);
}

// ======================================================
//  VIEW
// ======================================================

fn handle_view_threads(index: &Value, fur_dir: &Path, args: &ThreadArgs) {
    if !(args.view || args.id.is_none()) {
        return;
    }

    let active = index["active_thread"].as_str().unwrap_or("");
    let infos = collect_infos(index, fur_dir);

    if infos.is_empty() {
        println!("📭 No conversations yet. Run `fur new \"Title\"`.");
        return;
    }

    let total_size_bytes: u64 = infos.iter().map(|i| i.size_bytes).sum();

    // Neither view truncates. `--all` is kept as an accepted no-op so existing
    // muscle memory and scripts do not break.
    let _ = args.all;

    if args.flat {
        render_flat(&infos, active);
    } else {
        render_tree(&infos, active, fur_dir);
    }

    println!("----------------------------");
    println!("Total Memory Used: {}", format_size(total_size_bytes));
}

/// Read every conversation named by the index, newest first.
fn collect_infos(index: &Value, fur_dir: &Path) -> Vec<ConvoInfo> {
    let empty_vec: Vec<Value> = Vec::new();
    let threads = index["threads"].as_array().unwrap_or(&empty_vec);

    let mut infos = Vec::new();

    for tid in threads {
        let Some(tid_str) = tid.as_str() else {
            continue;
        };

        let convo_path = fur_dir.join("threads").join(format!("{}.json", tid_str));
        let Ok(content) = fs::read_to_string(&convo_path) else {
            continue;
        };
        let Ok(convo) = serde_json::from_str::<Value>(&content) else {
            continue;
        };

        let msg_ids: Vec<String> = convo["messages"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let created = DateTime::parse_from_rfc3339(convo["created_at"].as_str().unwrap_or(""))
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let local: DateTime<Local> = DateTime::from(created);

        infos.push(ConvoInfo {
            id: tid_str.to_string(),
            title: convo["title"].as_str().unwrap_or("Untitled").to_string(),
            created,
            date_str: local.format("%Y-%m-%d").to_string(),
            time_str: local.format("%H:%M").to_string(),
            msg_count: msg_ids.len(),
            size_bytes: compute_conversation_size(fur_dir, tid_str, &msg_ids),
            tags: convo["tags"]
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        });
    }

    infos.sort_by(|a, b| b.created.cmp(&a.created));
    infos
}

fn row_for(info: &ConvoInfo, title: String) -> Vec<String> {
    vec![
        info.id[..8].to_string(),
        title,
        format!("{} | {}", info.date_str, info.time_str),
        info.msg_count.to_string(),
        format_size(info.size_bytes),
        info.tags.clone(),
    ]
}

const HEADERS: [&str; 6] = ["ID", "Title", "Created", "#Msgs", "Size", "Tags"];

/// Newest-first list with no nesting — the pre-lineage view.
fn render_flat(infos: &[ConvoInfo], active: &str) {
    let rows: Vec<Vec<String>> = infos
        .iter()
        .map(|info| row_for(info, info.title.clone()))
        .collect();

    let active_idx = infos.iter().position(|i| i.id == active);

    render_table("Conversations", &HEADERS, rows, active_idx);
}

/// Lineage view: parents at the margin, children indented beneath them.
///
/// Conversations with no edges sit flat at the margin exactly as they do in the
/// flat view, so an archive that never uses linking looks unchanged.
fn render_tree(infos: &[ConvoInfo], active: &str, fur_dir: &Path) {
    let lineage = match Lineage::load(fur_dir) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("⚠ lineage unavailable ({}); showing a flat list", e);
            return render_flat(infos, active);
        }
    };

    if lineage.is_empty() {
        return render_flat(infos, active);
    }

    let by_id: HashMap<&str, &ConvoInfo> = infos.iter().map(|i| (i.id.as_str(), i)).collect();
    let order: Vec<String> = infos.iter().map(|i| i.id.clone()).collect();

    let mut rows = Vec::new();
    let mut active_idx = None;
    let mut any_repeat = false;
    let mut any_orphan = false;

    for entry in lineage.forest(&order) {
        let Some(info) = by_id.get(entry.id.as_str()) else {
            continue;
        };

        let indent = if entry.depth == 0 {
            String::new()
        } else {
            format!("{}└─ ", "   ".repeat(entry.depth - 1))
        };

        let mut title = format!("{}{}", indent, info.title);

        if entry.orphan_parent {
            any_orphan = true;
            title.push_str(" ↑");
        }

        if entry.repeat {
            any_repeat = true;
            title.push_str(" (above)");
        }

        if entry.id == active && !entry.repeat {
            active_idx = Some(rows.len());
        }

        rows.push(row_for(info, title));
    }

    render_table("Conversations", &HEADERS, rows, active_idx);

    if any_orphan {
        println!(
            "{}",
            "↑ linked to a conversation that is not in this project".bright_black()
        );
    }
    if any_repeat {
        println!(
            "{}",
            "(above) listed under another parent as well".bright_black()
        );
    }
}

// ======================================================
//  SWITCH / TAG
// ======================================================

fn handle_switch_thread(index: &mut Value, index_path: &Path, fur_dir: &Path, args: &ThreadArgs) {
    let tid = match &args.id {
        Some(id) => id,
        None => return,
    };

    let empty_vec: Vec<Value> = Vec::new();
    let threads: Vec<String> = index["threads"]
        .as_array()
        .unwrap_or(&empty_vec)
        .iter()
        .filter_map(|t| t.as_str().map(|s| s.to_string()))
        .collect();

    let mut found = threads.iter().find(|&s| s == tid);

    if found.is_none() {
        let matches: Vec<&String> = threads.iter().filter(|s| s.starts_with(tid)).collect();

        if matches.len() == 1 {
            found = Some(matches[0]);
        } else if matches.len() > 1 {
            eprintln!("❌ Ambiguous prefix '{}'. Matches: {:?}", tid, matches);
            return;
        }
    }

    let tid_full = match found {
        Some(s) => s,
        None => {
            eprintln!("❌ Thread not found: {}", tid);
            return;
        }
    };

    index["active_thread"] = json!(tid_full);
    index["current_message"] = serde_json::Value::Null;

    fs::write(index_path, serde_json::to_string_pretty(&index).unwrap()).unwrap();

    let convo_path = fur_dir.join("threads").join(format!("{}.json", tid_full));
    let content = fs::read_to_string(convo_path).unwrap();
    let conversation_json: Value = serde_json::from_str(&content).unwrap();
    let title = conversation_json["title"].as_str().unwrap_or("Untitled");

    println!(
        "✔️ Switched active conversation to {} \"{}\"",
        &tid_full[..8],
        title
    );
}

fn handle_tagging(args: &ThreadArgs, index: &mut Value, fur_dir: &Path) {
    let empty_vec: Vec<Value> = Vec::new();
    let threads: Vec<String> = index["threads"]
        .as_array()
        .unwrap_or(&empty_vec)
        .iter()
        .filter_map(|t| t.as_str().map(|s| s.to_string()))
        .collect();

    // Determine which conversation to operate on
    let target_tid = if let Some(prefix) = &args.id {
        let matches: Vec<&String> = threads
            .iter()
            .filter(|tid| tid.starts_with(prefix))
            .collect();

        if matches.is_empty() {
            eprintln!("❌ No conversation matches '{}'", prefix);
            return;
        }
        if matches.len() > 1 {
            eprintln!("❌ Ambiguous prefix '{}': {:?}", prefix, matches);
            return;
        }
        matches[0].clone()
    } else {
        index["active_thread"].as_str().unwrap_or("").to_string()
    };

    let convo_path = fur_dir.join("threads").join(format!("{}.json", target_tid));
    let mut convo: Value = serde_json::from_str(&fs::read_to_string(&convo_path).unwrap()).unwrap();

    // -------------------------------
    // CLEAR ALL TAGS
    // -------------------------------
    if args.clear_tags {
        convo["tags"] = json!([]);
        fs::write(&convo_path, serde_json::to_string_pretty(&convo).unwrap()).unwrap();
        println!("🏷️ Cleared tags for {}", &target_tid[..8]);
        crate::schema::bridge::sync_active();
        return;
    }

    // Load existing tags
    let mut existing: Vec<String> = convo["tags"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    // -------------------------------
    // REMOVE TAGS
    // -------------------------------
    if let Some(raw) = &args.untag {
        let remove_list = parse_tag_list(raw);

        existing.retain(|t| !remove_list.contains(t));

        convo["tags"] = json!(existing);
        fs::write(&convo_path, serde_json::to_string_pretty(&convo).unwrap()).unwrap();

        println!(
            "🏷️ Removed tag(s) [{}] from {}",
            remove_list.join(", "),
            &target_tid[..8]
        );
        crate::schema::bridge::sync_active();
        return;
    }

    // -------------------------------
    // ADD TAGS
    // -------------------------------
    if let Some(raw) = &args.tag {
        let add_list = parse_tag_list(raw);

        for t in add_list {
            if !existing.contains(&t) {
                existing.push(t);
            }
        }

        convo["tags"] = json!(existing);
        fs::write(&convo_path, serde_json::to_string_pretty(&convo).unwrap()).unwrap();

        println!("🏷️ Updated tags for {}", &target_tid[..8]);
        crate::schema::bridge::sync_active();
    }
}

// ======================================================
//  SIZE
// ======================================================

/// Computes total storage: conversation.json + all message JSONs + all markdown attachments.
fn compute_conversation_size(fur_dir: &Path, tid: &str, msg_ids: &[String]) -> u64 {
    let mut total: u64 = 0;

    // Add main conversation JSON
    let convo_path = fur_dir.join("threads").join(format!("{}.json", tid));
    total += file_size(&convo_path);

    // Add all messages + markdowns
    total += get_message_file_sizes(fur_dir, msg_ids);

    total
}

fn get_message_file_sizes(fur_dir: &Path, msg_ids: &[String]) -> u64 {
    let mut total = 0;

    for mid in msg_ids {
        let msg_path = fur_dir.join("messages").join(format!("{}.json", mid));
        total += file_size(&msg_path);

        // Parse JSON to find ONLY message["markdown"]
        if let Ok(content) = fs::read_to_string(&msg_path) {
            if let Ok(json) = serde_json::from_str::<Value>(&content) {
                if let Some(md_raw) = json["markdown"].as_str() {
                    // CASE 1: absolute path -> use as-is
                    let md_path = Path::new(md_raw);
                    if md_path.is_absolute() {
                        total += file_size(md_path);
                        continue;
                    }

                    // CASE 2: relative path -> resolve relative to project root
                    let project_root_path = Path::new(".").join(md_raw);
                    total += file_size(&project_root_path);
                }
            }
        }
    }

    total
}

fn file_size(path: &Path) -> u64 {
    fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

pub fn format_size(bytes: u64) -> String {
    if bytes < 1_048_576 {
        format!("{} KB", (bytes as f64 / 1024.0).round() as u64)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}