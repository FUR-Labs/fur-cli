use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use indicatif::{ProgressBar, ProgressStyle};

use serde_json::Value;
use walkdir::WalkDir;
use sha2::{Sha256, Digest};
use clap::Parser;
use colored::*;

#[derive(Parser, Debug)]
pub struct DoctorArgs {

    /// Search the entire home directory for moved attachments
    #[arg(long)]
    pub deep: bool,

    /// Remove attachment references that cannot be recovered
    /// (always performs a deep search first)
    #[arg(long)]
    pub clean: bool,
}

pub fn run_doctor(args: DoctorArgs) {

    let fur_dir = Path::new(".fur");

    if !fur_dir.exists() {
        eprintln!("❌ No .fur directory found.");
        return;
    }

    let messages_dir = fur_dir.join("messages");

    println!("🩺 FUR Doctor\n");

    let deep = args.deep || args.clean;

    let search_root: PathBuf = if deep {
        println!("Deep scan enabled.\n");
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
    } else {
        PathBuf::from(".")
    };

    let mut missing: HashMap<String, Vec<PathBuf>> = HashMap::new();
    let mut message_paths: Vec<PathBuf> = Vec::new();

    for entry in fs::read_dir(&messages_dir).unwrap() {

        let path = entry.unwrap().path();

        if !path.is_file() {
            continue;
        }

        message_paths.push(path.clone());

        let content = fs::read_to_string(&path).unwrap();

        let msg: Value = serde_json::from_str(&content).unwrap();

        if let Some(md_path) = msg["markdown"].as_str() {

            if !Path::new(md_path).exists() {

                missing
                    .entry(md_path.to_string())
                    .or_default()
                    .push(path.clone());
            }
        }
    }

    if missing.is_empty() {
        println!("✔ No issues detected.");
        return;
    }

    println!("Missing attachments\n-------------------");

    for (path, refs) in &missing {
        println!("⚠ {} ({} references)", path, refs.len());
    }

    
    let pb = ProgressBar::new_spinner();

    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
    );

    pb.enable_steady_tick(std::time::Duration::from_millis(120));

    pb.set_message("Scanning filesystem...");


    let mut recovered: usize = 0;
    let mut unrecoverable: Vec<String> = Vec::new();

    for (missing_path, msg_refs) in &missing {

        let hash = extract_hash(&msg_refs[0]);

        if hash.is_none() {
            unrecoverable.push(missing_path.clone());
            continue;
        }

        let hash = hash.unwrap();

        if let Some(found) = search_by_hash(&hash, &search_root, &pb) {

            println!("Recovered: {} → {}", missing_path, found.display());

            for msg_file in msg_refs {

                let mut msg: Value =
                    serde_json::from_str(&fs::read_to_string(msg_file).unwrap()).unwrap();

                msg["markdown"] = Value::String(found.to_string_lossy().to_string());

                fs::write(
                    msg_file,
                    serde_json::to_string_pretty(&msg).unwrap()
                ).unwrap();
            }

            recovered += msg_refs.len();

        } else {

            unrecoverable.push(missing_path.clone());
        }
    }

    if !unrecoverable.is_empty() {

        println!("\nUnrecoverable attachments\n-------------------------");

        for p in &unrecoverable {
            println!("✖ {}", p);
        }

        println!();

        println!("{}", "Tip".bold().bright_yellow());
        println!(
            "  {}",
            "Run `fur doctor --deep` to search your system for moved files."
                .bold()
        );

        println!(
            "  {}",
            "Run `fur doctor --clean` only if you are sure the files are gone."
                .bold()
        );

        println!(
            "  {}",
            "(--clean always performs a deep search first.)"
                .dimmed()
        );
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

                    fs::write(
                        msg_file,
                        serde_json::to_string_pretty(&msg).unwrap()
                    ).unwrap();
                }
            }
        }

        println!("✔ Orphan attachment metadata cleaned.");
    }

    pb.finish_with_message("Filesystem scan complete.");
    
    println!("\nSummary");
    println!("-------");
    println!("Recovered references: {}", recovered);
    println!("Unrecoverable attachments: {}", unrecoverable.len());

    println!("\nDoctor finished.");
}

fn extract_hash(msg_path: &Path) -> Option<String> {

    let content = fs::read_to_string(msg_path).ok()?;

    let msg: Value = serde_json::from_str(&content).ok()?;

    msg["markdown_meta"]["hash"]
        .as_str()
        .map(|s| s.to_string())
}

fn search_by_hash(target: &str, root: &Path, pb: &ProgressBar) -> Option<PathBuf> {

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        pb.inc(1);

        if !path.is_file() {
            continue;
        }

        if let Ok(bytes) = fs::read(path) {

            let mut hasher = Sha256::new();
            hasher.update(&bytes);

            let result = format!("{:x}", hasher.finalize());

            if result == target {
                return Some(path.to_path_buf());
            }
        }

        if pb.position() % 1000 == 0 {
            pb.set_message(format!("Scanned {} files...", pb.position()));
        }
    }

    None
}