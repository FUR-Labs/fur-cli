use crate::commands::timeline::TimelineArgs;
use crate::commands::tree::TreeArgs;
use crate::commands::{timeline, tree};
use crate::frs::{parser, persist_frs};
use colored::*;
use std::fs;

/// Run an .frs script:
/// - Parse into Thread (in-memory)
/// - Execute inline commands (tree, timeline, status)
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

    let conversation = parser::parse_frs(path);
    let mut stored = false;

    for (lineno, line) in lines.iter().enumerate() {
        // --- Commit point
        if line == "store" {
            if !stored {
                let tid = persist_frs(&conversation);
                println!("✔️ Thread persisted at line {} → {}", lineno + 1, &tid[..8]);
                stored = true;
            } else {
                eprintln!(
                    "{}",
                    format!(
                        "⚠️ Ignoring extra `store` at line {} — already persisted",
                        lineno + 1
                    )
                    .yellow()
                    .bold()
                );
            }
            continue;
        }

        // --- Status
        if line.starts_with("status") {
            with_ephemeral(stored, &conversation, |tid_override| {
                let args = crate::commands::status::StatusArgs {
                    conversation_override: tid_override,
                };
                crate::commands::status::run_status(args);
            });
            continue;
        }

        // --- Timeline
        if line.starts_with("timeline") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let mut args = TimelineArgs {
                verbose: false,
                contents: false,
                out: None,
                conversation_override: None,
            };
            for (i, p) in parts.iter().enumerate() {
                if *p == "--out" {
                    args.out = parts.get(i + 1).map(|s| s.to_string());
                }
                if *p == "--contents" {
                    args.contents = true;
                }
            }

            with_ephemeral(stored, &conversation, |tid_override| {
                let mut args = args.clone();
                args.conversation_override = tid_override;
                timeline::run_timeline(args);
            });
            continue;
        }

        // --- Printed
        if line.starts_with("printed") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let mut out: Option<String> = None;
            let mut verbose = false;

            for (i, p) in parts.iter().enumerate() {
                if *p == "--out" {
                    out = parts.get(i + 1).map(|s| s.to_string());
                }
                if *p == "--verbose" || *p == "-v" {
                    verbose = true;
                }
            }

            with_ephemeral(stored, &conversation, |tid_override| {
                // Load index.json so we can temporarily override active thread
                let index_path = std::path::Path::new(".fur").join("index.json");
                let mut index_json: serde_json::Value =
                    serde_json::from_str(&std::fs::read_to_string(&index_path).unwrap()).unwrap();

                let original_active = index_json["active_thread"].as_str().map(|s| s.to_string());

                if let Some(tid) = &tid_override {
                    index_json["active_thread"] = tid.clone().into();
                    std::fs::write(
                        &index_path,
                        serde_json::to_string_pretty(&index_json).unwrap(),
                    )
                    .unwrap();
                }

                // Now run printed, which will read this modified active_thread
                crate::commands::printed::run_printed(out.clone(), verbose);

                // Restore original active_thread
                if let Some(orig) = original_active {
                    index_json["active_thread"] = orig.into();
                    std::fs::write(
                        &index_path,
                        serde_json::to_string_pretty(&index_json).unwrap(),
                    )
                    .unwrap();
                }
            });

            continue;
        }

        // --- Tree
        if line.starts_with("tree") {
            let args = TreeArgs {
                conversation_override: None,
            };
            with_ephemeral(stored, &conversation, |tid_override| {
                let mut args = args.clone();
                args.conversation_override = tid_override;
                tree::run_tree(args);
            });
            continue;
        }

        // Default: skip (jots already parsed by parser::parse_frs)
    }

    if !stored {
        eprintln!(
            "{}",
            "⚠️ Script finished without a `store` — nothing persisted.".yellow()
        );
    }
}

/// Run a command either with an ephemeral conversation (if not stored) or directly.
fn with_ephemeral<F>(stored: bool, conversation: &crate::frs::ast::Thread, mut f: F)
where
    F: FnMut(Option<String>),
{
    if !stored {
        let tid = crate::frs::persist::persist_ephemeral(conversation);
        f(Some(tid.clone()));
        crate::frs::persist::cleanup_ephemeral(&tid);
    } else {
        f(None);
    }
}
