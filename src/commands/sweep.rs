use std::fs;
use std::path::PathBuf;
use chrono::{DateTime, Local};
use serde_json::{Value, json};
use clap::Parser;
use walkdir::WalkDir;
use dirs;
use colored::*;

/// Arguments for `fur sweep`
#[derive(Parser)]
pub struct SweepArgs {
    /// Directory to start scan (default: home)
    #[arg(long)]
    pub dir: Option<String>,

    /// Maximum recursion depth
    #[arg(long, default_value_t = 5)]
    pub depth: usize,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Suppress warnings
    #[arg(long)]
    pub silent: bool,
}

pub fn run_sweep(args: SweepArgs) {
    let start_dir = args.dir
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")));

    let mut results = Vec::new();

    for entry in WalkDir::new(&start_dir)
        .follow_links(false)
        .max_depth(args.depth)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_dir() && path.ends_with(".fur") {
            let index_path = path.join("index.json");
            if index_path.exists() {
                if let Ok(content) = fs::read_to_string(&index_path) {
                    if let Ok(json) = serde_json::from_str::<Value>(&content) {
                        let threads_vec = json["threads"]
                            .as_array()
                            .cloned()
                            .unwrap_or_else(|| Vec::new());
                        let active = json["active_thread"].as_str().unwrap_or("-");
                        let msg_count = threads_vec.iter().filter_map(|tid| {
                            tid.as_str().map(|t| {
                                let tp = path.join("threads").join(format!("{}.json", t));
                                if let Ok(tc) = fs::read_to_string(tp) {
                                    serde_json::from_str::<Value>(&tc)
                                        .ok()
                                        .and_then(|v| v["messages"].as_array().map(|a| a.len()))
                                        .unwrap_or(0)
                                } else { 0 }
                            })
                        }).sum::<usize>();

                        let modified = fs::metadata(&index_path)
                            .and_then(|m| m.modified())
                            .ok()
                            .map(|t| {
                                let dt: DateTime<Local> = DateTime::from(t);
                                dt.format("%Y-%m-%d %H:%M").to_string()
                            })
                            .unwrap_or("-".into());

                        results.push(json!({
                            "path": path.parent().unwrap_or(path).display().to_string(),
                            "active_thread": active,
                            "threads": threads_vec.len(),
                            "messages": msg_count,
                            "modified": modified
                        }));
                    }
                }
            }
        }
    }

    let mut total_projects = 0;
    let mut total_threads = 0;
    let mut total_messages = 0;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&results).unwrap());
    } else {
        println!("{}", "=== 🧭 FUR Project Sweep ===\n".bold().bright_cyan());
        for r in &results {
            total_projects += 1;
            total_threads += r["threads"].as_u64().unwrap_or(0);
            total_messages += r["messages"].as_u64().unwrap_or(0);

            let path = r["path"].as_str().unwrap_or("-").bright_yellow().bold();
            let active = r["active_thread"].as_str().unwrap_or("-").bright_cyan();
            let threads = r["threads"].as_u64().unwrap_or(0).to_string().green();
            let messages = r["messages"].as_u64().unwrap_or(0).to_string().blue();
            let modified = r["modified"].as_str().unwrap_or("-").bright_black();

            println!(
                "{}\n  {} {}\n  {} {}\n  {} {}\n  {} {}\n",
                path,
                "Active Thread :".dimmed(),
                active,
                "Threads       :".dimmed(),
                threads,
                "Messages      :".dimmed(),
                messages,
                "Last Modified :".dimmed(),
                modified,
            );
        }

        // Summary footer
        println!("{}", "─".repeat(50).dimmed());
        println!(
            "🌍  Found {} {} | {} {} | {} {}\n",
            total_projects.to_string().bold().bright_yellow(),
            if total_projects == 1 { "project" } else { "projects" },
            total_threads.to_string().bold().green(),
            if total_threads == 1 { "thread" } else { "threads" },
            total_messages.to_string().bold().blue(),
            if total_messages == 1 { "message" } else { "messages" },
        );
    }
}


