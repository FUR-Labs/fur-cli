use std::fs;
use std::path::Path;
use colored::*;
use crate::frs::{parser, persist_frs};
use crate::commands::{timeline, tree};
use crate::commands::timeline::TimelineArgs;
use crate::commands::tree::TreeArgs;

/// Run an .frs script:
/// - Parse into Thread (in-memory)
/// - Execute inline commands (tree, timeline, etc.)
/// - Persist once at first `store`
/// - Ignore later `store`s
pub fn run_frs(path: &str) {
    let raw = fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("❌ Could not read .frs file: {}", path));

    let lines: Vec<String> = raw
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    let mut thread = parser::parse_frs(path);
    let mut stored = false;

    for (lineno, line) in lines.iter().enumerate() {
        // Handle special commands
        if line == "store" {
            if !stored {
                let tid = persist_frs(&thread);
                println!("✔️ Thread persisted at line {} → {}", lineno + 1, &tid[..8]);
                stored = true;
            } else {
                eprintln!(
                    "{}",
                    format!("⚠️ Ignoring extra `store` at line {} — already persisted", lineno + 1)
                        .yellow()
                        .bold()
                );
            }
            continue;
        }

        if line.starts_with("timeline") {
            // crude arg split, you may want to Clap-ify later
            let parts: Vec<&str> = line.split_whitespace().collect();
            let mut args = TimelineArgs {
                verbose: false,
                contents: false,
                out: None,
            };
            for (i, p) in parts.iter().enumerate() {
                if *p == "--out" {
                    args.out = parts.get(i + 1).map(|s| s.to_string());
                }
                if *p == "--contents" {
                    args.contents = true;
                }
            }
            timeline::run_timeline(args);
            continue;
        }

        if line.starts_with("tree") {
            let args = TreeArgs {};
            tree::run_tree(args);
            continue;
        }

        // Default: skip, because parse_frs already consumed jots/branches
    }

    if !stored {
        eprintln!("{}", "⚠️ Script finished without a `store` — nothing persisted.".yellow());
    }
}


/// Dispatch a script-level command (timeline, tree, etc.)
fn dispatch_command(cmd: Command) {
    match cmd.name.as_str() {
        "timeline" => {
            // Minimal: timeline --out foo.md
            let args = parse_timeline_args(&cmd.args);
            timeline::run_timeline(args);
        }
        "tree" => {
            let args = crate::commands::tree::TreeArgs {};
            tree::run_tree(args);
        }
        "status" => {
            status::run_status();
        }
        "save" | "store" => {
            let args = parse_save_args(&cmd.args);
            save::run_save(args);
        }
        other => {
            eprintln!("⚠️ Unknown script command at line {}: {}", cmd.line_number, other);
        }
    }
}

// TODO: implement simple arg parsing bridges
fn parse_timeline_args(args: &[String]) -> crate::commands::timeline::TimelineArgs {
    use clap::Parser;
    crate::commands::timeline::TimelineArgs::parse_from(
        std::iter::once("timeline".to_string()).chain(args.to_owned())
    )
}

fn parse_save_args(args: &[String]) -> crate::commands::save::SaveArgs {
    use clap::Parser;
    crate::commands::save::SaveArgs::parse_from(
        std::iter::once("save".to_string()).chain(args.to_owned())
    )
}
