use std::fs;
use std::path::Path;
use serde_json::{Value, json};
use clap::Parser;
use chrono::{DateTime, Local, Utc};
use crate::renderer::list::render_list;

/// Arguments for the `thread` command
#[derive(Parser)]
pub struct ThreadArgs {
    /// Thread ID or prefix to switch
    pub id: Option<String>,

    /// View all threads
    #[arg(long)]
    pub view: bool,
}

/// Main entry point for the `thread` command
pub fn run_thread(args: ThreadArgs) {
    let fur_dir = Path::new(".fur");
    let index_path = fur_dir.join("index.json");

    if !index_path.exists() {
        eprintln!("🚨 .fur/ not found. Run `fur new` first.");
        return;
    }

    let mut index: Value =
        serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();

    // ------------------------
    // VIEW ALL THREADS
    // ------------------------
    if args.view || args.id.is_none() {
        let empty_vec: Vec<Value> = Vec::new();
        let threads = index["threads"].as_array().unwrap_or(&empty_vec);
        let active = index["active_thread"].as_str().unwrap_or("");

        let mut rows = Vec::new();
        let mut active_idx = None;

        // Collect thread metadata first
        let mut thread_info = Vec::new();
        for tid in threads {
            if let Some(tid_str) = tid.as_str() {
                let thread_path = fur_dir.join("threads").join(format!("{}.json", tid_str));
                if let Ok(content) = fs::read_to_string(thread_path) {
                    if let Ok(thread_json) = serde_json::from_str::<Value>(&content) {
                        let title = thread_json["title"].as_str().unwrap_or("Untitled").to_string();
                        let created_raw = thread_json["created_at"].as_str().unwrap_or("");
                        let msg_count = thread_json["messages"]
                            .as_array()
                            .map(|a| a.len())
                            .unwrap_or(0);

                        // Parse created_at safely
                        let parsed_time = DateTime::parse_from_rfc3339(created_raw)
                            .map(|dt| dt.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now());
                        let local_time: DateTime<Local> = DateTime::from(parsed_time);
                        let date_str = local_time.format("%Y-%m-%d").to_string();
                        let time_str = local_time.format("%H:%M").to_string();

                        thread_info.push((
                            tid_str.to_string(),
                            title,
                            date_str,
                            time_str,
                            msg_count,
                            parsed_time,
                        ));
                    }
                }
            }
        }

        // Sort newest → oldest
        thread_info.sort_by(|a, b| b.5.cmp(&a.5));

        // Build rows and track active index
        for (i, (tid, title, date, time, msg_count, _)) in thread_info.iter().enumerate() {
            let short_id = &tid[..8];
            rows.push(vec![
                short_id.to_string(),
                title.to_string(),
                format!("{} | {}", date, time),
                msg_count.to_string(),
            ]);
            if tid == active {
                active_idx = Some(i);
            }
        }

        render_list("Threads", &["ID", "Title", "Created", "#Msgs"], rows, active_idx);
        return;
    }

    // ------------------------
    // SWITCH ACTIVE THREAD
    // ------------------------
    if let Some(tid) = args.id {
        let empty_vec: Vec<Value> = Vec::new();
        let threads: Vec<String> = index["threads"]
            .as_array()
            .unwrap_or(&empty_vec)
            .iter()
            .filter_map(|t| t.as_str().map(|s| s.to_string()))
            .collect();

        let mut found = threads.iter().find(|&s| s == &tid);
        if found.is_none() {
            let matches: Vec<&String> = threads.iter().filter(|s| s.starts_with(&tid)).collect();
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
        fs::write(&index_path, serde_json::to_string_pretty(&index).unwrap()).unwrap();

        let thread_path = fur_dir.join("threads").join(format!("{}.json", tid_full));
        let content = fs::read_to_string(thread_path).unwrap();
        let thread_json: Value = serde_json::from_str(&content).unwrap();
        let title = thread_json["title"].as_str().unwrap_or("Untitled");

        println!("✔️ Switched active thread to {} \"{}\"", &tid_full[..8], title);
    }
}
