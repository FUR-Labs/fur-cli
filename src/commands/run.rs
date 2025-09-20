use crate::frs::{import_frs, persist_frs};
use crate::frs::ast::{ScriptItem, Command};
use crate::commands::{timeline, tree, status, save};
use std::path::Path;

/// Run an .frs script: import, persist, and execute commands
pub fn run_frs(path: &str) {
    if !Path::new(path).exists() {
        eprintln!("❌ File not found: {}", path);
        return;
    }

    // Step 1: parse script
    let thread = import_frs(path);

    // Step 2: persist messages (like `fur load`)
    let thread_id = persist_frs(&thread);
    println!("✔️ Imported as thread {}", &thread_id[..8]);

    // Step 3: execute embedded commands
    for item in thread.items {
        match item {
            ScriptItem::Message(_) => {
                // already persisted in step 2
            }
            ScriptItem::Command(cmd) => {
                dispatch_command(cmd);
            }
        }
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
