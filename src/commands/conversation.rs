use std::fs;
use std::path::{Path};
use serde_json::{Value, json};
use clap::Parser;
use chrono::{DateTime, Local, Utc};
use crate::helpers::conversation::{resolve_target_thread_id,confirm_delete_primary,confirm_delete_destructive, perform_conversation_deletion};
use crate::renderer::table::render_table;
use crate::helpers::tags::parse_tag_list;

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
}

pub fn run_conversation(args: ThreadArgs) {
    let fur_dir = Path::new(".fur");
    let index_path = fur_dir.join("index.json");

    if !index_path.exists() {
        eprintln!("🚨 .fur/ not found. Run `fur new` first.");
        return;
    }

    let mut index: Value =
        serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();

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

fn handle_rename_thread(
    index: &mut Value,
    fur_dir: &Path,
    args: &ThreadArgs,
) {
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

    let convo_path = fur_dir.join("threads").join(format!("{}.json", target_thread_id));
    let mut conversation_json: Value =
        serde_json::from_str(&fs::read_to_string(&convo_path).unwrap()).unwrap();

    let old_title = conversation_json["title"].as_str().unwrap_or("Untitled").to_string();

    // Update title
    conversation_json["title"] = Value::String(new_title.to_string());
    fs::write(&convo_path, serde_json::to_string_pretty(&conversation_json).unwrap()).unwrap();

    println!(
        "✏️  Renamed conversation {} \"{}\" → \"{}\"",
        &target_thread_id[..8],
        old_title,
        new_title
    );
}


fn handle_delete_thread(
    index: &mut Value,
    fur_dir: &Path,
    args: &ThreadArgs,
) {
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



fn handle_view_threads(
    index: &Value,
    fur_dir: &Path,
    args: &ThreadArgs,
) {
    if !(args.view || args.id.is_none()) {
        return;
    }

    let empty_vec: Vec<Value> = Vec::new();
    let threads = index["threads"].as_array().unwrap_or(&empty_vec);
    let active = index["active_thread"].as_str().unwrap_or("");

    let mut rows = Vec::new();
    let mut active_idx = None;

    let mut total_size_bytes: u64 = 0;
    let mut conversation_info = Vec::new();

    for tid in threads {
        if let Some(tid_str) = tid.as_str() {
            let convo_path = fur_dir.join("threads").join(format!("{}.json", tid_str));

            if let Ok(content) = fs::read_to_string(&convo_path) {
                if let Ok(convo) = serde_json::from_str::<Value>(&content) {
                    let title = convo["title"].as_str().unwrap_or("Untitled").to_string();
                    let created_raw = convo["created_at"].as_str().unwrap_or("");

                    let msg_ids = convo["messages"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();

                    let msg_count = msg_ids.len();

                    let parsed = DateTime::parse_from_rfc3339(created_raw)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now());

                    let local: DateTime<Local> = DateTime::from(parsed);
                    let date_str = local.format("%Y-%m-%d").to_string();
                    let time_str = local.format("%H:%M").to_string();

                    let size_bytes = compute_conversation_size(fur_dir, tid_str, &msg_ids);
                    total_size_bytes += size_bytes;

                    let tags_str = convo["tags"]
                        .as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");

                    conversation_info.push((
                        tid_str.to_string(),
                        title,
                        date_str,
                        time_str,
                        msg_count,
                        parsed,
                        format_size(size_bytes),
                        tags_str,
                    ));
                }
            }
        }
    }

    // Sort newest first
    conversation_info.sort_by(|a, b| b.5.cmp(&a.5));

    for (i, (tid, title, date, time, msg_count, _, size_str, tags_str)) in
        conversation_info.iter().enumerate()
    {
        rows.push(vec![
            tid[..8].to_string(),
            title.to_string(),
            format!("{} | {}", date, time),
            msg_count.to_string(),
            size_str.to_string(),
            tags_str.to_string(),
        ]);

        if tid == active {
            active_idx = Some(i);
        }
    }

    // Apply truncation AFTER rows and active_idx exist
    if !args.all {
        rows = truncate_around_active(rows, active_idx);

        // Recalculate active_idx inside truncated rows
        let active_prefix = &active[..8];
        active_idx = rows
            .iter()
            .position(|row| row[0] == active_prefix);
    }

    // UPDATED HEADERS: now includes TAGS
    render_table(
        "Conversations",
        &["ID", "Title", "Created", "#Msgs", "Size", "Tags"],
        rows,
        active_idx,
    );

    println!("----------------------------");
    println!("Total Memory Used: {}", format_size(total_size_bytes));
}

fn truncate_around_active(rows: Vec<Vec<String>>, active_idx: Option<usize>) -> Vec<Vec<String>> {
    if active_idx.is_none() { return rows; }
    let i = active_idx.unwrap();
    let win = 3;

    let start = i.saturating_sub(win);
    let end   = (i + win).min(rows.len() - 1);

    let mut out = Vec::new();

    if start > 0 {
        out.push(vec!["...".into(); rows[0].len()]);
    }

    for idx in start..=end {
        out.push(rows[idx].clone());
    }

    if end < rows.len() - 1 {
        out.push(vec!["...".into(); rows[0].len()]);
    }

    out
}

fn handle_switch_thread(
    index: &mut Value,
    index_path: &Path,
    fur_dir: &Path,
    args: &ThreadArgs,
) {
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
        let matches: Vec<&String> =
            threads.iter().filter(|s| s.starts_with(tid)).collect();

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

fn handle_tagging(
    args: &ThreadArgs,
    index: &mut Value,
    fur_dir: &Path,
) {
    let empty_vec: Vec<Value> = Vec::new();
    let threads: Vec<String> = index["threads"]
        .as_array()
        .unwrap_or(&empty_vec)
        .iter()
        .filter_map(|t| t.as_str().map(|s| s.to_string()))
        .collect();

    // Determine which conversation to operate on
    let target_tid = if let Some(prefix) = &args.id {
        let matches: Vec<&String> =
            threads.iter().filter(|tid| tid.starts_with(prefix)).collect();

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
    let mut convo: Value =
        serde_json::from_str(&fs::read_to_string(&convo_path).unwrap()).unwrap();

    // -------------------------------
    // CLEAR ALL TAGS
    // -------------------------------
    if args.clear_tags {
        convo["tags"] = json!([]);
        fs::write(&convo_path, serde_json::to_string_pretty(&convo).unwrap()).unwrap();
        println!("🏷️ Cleared tags for {}", &target_tid[..8]);
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
        return;
    }
}

/// Computes total storage: conversation.json + all message JSONs + all markdown attachments.
fn compute_conversation_size(
    fur_dir: &Path,
    tid: &str,
    msg_ids: &[String],
) -> u64 {
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
