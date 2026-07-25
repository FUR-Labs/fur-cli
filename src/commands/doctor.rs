use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::Value;
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

#[derive(Parser, Debug)]
pub struct DoctorArgs {
    /// Expand search to home directory
    #[arg(long)]
    pub deep: bool,

    /// Remove unrecoverable attachment metadata
    /// (always performs deep search first)
    #[arg(long)]
    pub clean: bool,
}

pub fn run_doctor(args: DoctorArgs) {
    if crate::security::state::is_locked() {
        println!("🔒 Project locked. Run `fur unlock` first.");
        return;
    }

    let fur_dir = Path::new(".fur");

    if !fur_dir.exists() {
        eprintln!("❌ No .fur directory found.");
        return;
    }

    println!("🩺 FUR Doctor\n");

    let messages_dir = fur_dir.join("messages");

    let mut missing: HashMap<String, Vec<PathBuf>> = HashMap::new();

    for entry in fs::read_dir(&messages_dir).unwrap() {
        let msg_path = entry.unwrap().path();

        if !msg_path.is_file() {
            continue;
        }

        let content = fs::read_to_string(&msg_path).unwrap();
        let msg: Value = serde_json::from_str(&content).unwrap();

        if let Some(md_path) = msg["markdown"].as_str() {
            if !Path::new(md_path).exists() {
                missing
                    .entry(md_path.to_string())
                    .or_default()
                    .push(msg_path.clone());
            }
        }
    }

    if missing.is_empty() {
        println!("✔ No issues detected.");
        return;
    }

    println!("Missing attachments\n-------------------");

    for (p, refs) in &missing {
        println!("⚠ {} ({} references)", p, refs.len());
    }

    println!();

    let search_roots = collect_roots(args.deep);

    let pb = ProgressBar::new_spinner();

    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );

    pb.enable_steady_tick(std::time::Duration::from_millis(120));
    pb.set_message("Searching filesystem envelope...");

    let mut recovered_refs = 0usize;
    let mut unrecoverable: Vec<String> = Vec::new();
    let mut recovered_items: Vec<(String, String, usize)> = Vec::new();

    for (missing_path, msg_refs) in &missing {
        let filename = extract_filename(missing_path);
        let size = extract_size(&msg_refs[0]);
        let hash = extract_hash(&msg_refs[0]);

        if filename.is_none() || size.is_none() || hash.is_none() {
            unrecoverable.push(missing_path.clone());
            continue;
        }

        let filename = filename.unwrap();
        let size = size.unwrap();
        let hash = hash.unwrap();

        let mut found_path: Option<PathBuf> = None;

        fn check_candidate(path: &Path, filename: &str, hash: &str, found: &mut Option<PathBuf>) {
            if !path.is_file() {
                return;
            }

            if path
                .file_name()
                .and_then(|f| f.to_str())
                .map(|f| f == filename)
                .unwrap_or(false)
            {
                if let Ok(bytes) = fs::read(path) {
                    let mut hasher = Sha256::new();
                    hasher.update(&bytes);

                    let result = format!("{:x}", hasher.finalize());

                    if result == hash {
                        *found = Some(path.to_path_buf());
                    }
                }
            }
        }

        for root in &search_roots {
            if let Ok(entries) = fs::read_dir(root) {
                for e in entries.flatten() {
                    let path = e.path();
                    check_candidate(&path, filename, &hash, &mut found_path);
                    if found_path.is_some() {
                        break;
                    }
                }
            }

            if found_path.is_some() {
                break;
            }

            for entry in WalkDir::new(root)
                .max_depth(if args.deep { 10 } else { 4 })
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();

                if !path.is_file() {
                    continue;
                }

                if path
                    .file_name()
                    .and_then(|f| f.to_str())
                    .map(|f| f == filename)
                    .unwrap_or(false)
                {
                    if let Ok(meta) = fs::metadata(path) {
                        if meta.len() == size {
                            if let Ok(bytes) = fs::read(path) {
                                let mut hasher = Sha256::new();
                                hasher.update(&bytes);

                                let result = format!("{:x}", hasher.finalize());

                                if result == hash {
                                    found_path = Some(path.to_path_buf());
                                    break;
                                }
                            }
                        }
                    }
                }
            }

            if found_path.is_some() {
                break;
            }
        }

        if let Some(found) = found_path {
            for msg_file in msg_refs {
                let mut msg: Value =
                    serde_json::from_str(&fs::read_to_string(msg_file).unwrap()).unwrap();

                msg["markdown"] = Value::String(found.to_string_lossy().to_string());

                fs::write(msg_file, serde_json::to_string_pretty(&msg).unwrap()).unwrap();
            }

            recovered_refs += msg_refs.len();

            recovered_items.push((
                missing_path.clone(),
                found.display().to_string(),
                msg_refs.len(),
            ));
        } else {
            unrecoverable.push(missing_path.clone());
        }
    }

    pb.finish_and_clear();

    if !unrecoverable.is_empty() {
        println!(
            "{}",
            "\nUnrecoverable attachments"
                .bold()
                .truecolor(255, 105, 180)
        );
        println!("{}", "-------------------------".truecolor(255, 105, 180));

        for p in &unrecoverable {
            println!("✖ {}", p.truecolor(255, 105, 180));
        }

        println!();

        println!("{}", "Tip".bold().bright_yellow());
        println!(
            "  {}",
            "Run `fur doctor --deep` to search your home directory.".bold()
        );
        println!(
            "  {}",
            "Run `fur doctor --clean` only if you are sure the files are gone.".bold()
        );
        println!(
            "  {}",
            "(--clean always performs a deep search first.)".dimmed()
        );
    }

    if !recovered_items.is_empty() {
        println!("{}", "\nRecovered attachments".bold().green());
        println!("{}", "---------------------".green());

        for (orig, new, refs) in &recovered_items {
            println!("✔ {}", orig.green());
            println!("  → {}", new);
            println!("  ({} reference repaired)\n", refs);
        }
    }

    if args.clean {
        println!("\nCleaning orphan attachment metadata...\n");

        for (missing_path, msg_refs) in &missing {
            if unrecoverable.contains(missing_path) {
                for msg_file in msg_refs {
                    let mut msg: Value =
                        serde_json::from_str(&fs::read_to_string(msg_file).unwrap()).unwrap();

                    msg["markdown"] = Value::Null;
                    msg["markdown_meta"] = Value::Null;

                    fs::write(msg_file, serde_json::to_string_pretty(&msg).unwrap()).unwrap();
                }
            }
        }

        println!("✔ Orphan attachment metadata cleaned.");
    }

    println!("\nSummary");
    println!("-------");
    println!("Recovered references: {}", recovered_refs);
    println!("Unrecoverable attachments: {}", unrecoverable.len());

    println!("\nDoctor finished.");
}

fn collect_roots(deep: bool) -> Vec<PathBuf> {
    let mut roots = Vec::new();

    let mut current = std::env::current_dir().unwrap();

    for _ in 0..4 {
        roots.push(current.clone());

        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            break;
        }
    }

    if deep {
        if let Some(home) = dirs::home_dir() {
            roots.push(home);
        }
    }

    roots
}

fn extract_hash(msg_path: &Path) -> Option<String> {
    let content = fs::read_to_string(msg_path).ok()?;
    let msg: Value = serde_json::from_str(&content).ok()?;

    msg["markdown_meta"]["hash"].as_str().map(|s| s.to_string())
}

fn extract_size(msg_path: &Path) -> Option<u64> {
    let content = fs::read_to_string(msg_path).ok()?;
    let msg: Value = serde_json::from_str(&content).ok()?;

    msg["markdown_meta"]["size"].as_u64()
}

fn extract_filename(path: &str) -> Option<&str> {
    Path::new(path).file_name()?.to_str()
}
