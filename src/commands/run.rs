use std::fs;
use colored::*;
use crate::frs::{parser, persist_frs};
use crate::commands::{timeline, tree};
use crate::commands::timeline::TimelineArgs;
use crate::commands::tree::TreeArgs;

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

    let thread = parser::parse_frs(path);
    let mut stored = false;

    for (lineno, line) in lines.iter().enumerate() {
        // --- Commit point
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

        // --- Status
        if line.starts_with("status") {
            with_ephemeral(stored, &thread, |tid_override| {
                let args = crate::commands::status::StatusArgs {
                    thread_override: tid_override,
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
                thread_override: None,
            };
            for (i, p) in parts.iter().enumerate() {
                if *p == "--out" {
                    args.out = parts.get(i + 1).map(|s| s.to_string());
                }
                if *p == "--contents" {
                    args.contents = true;
                }
            }

            with_ephemeral(stored, &thread, |tid_override| {
                args.thread_override = tid_override;
                timeline::run_timeline(args);
            });
            continue;
        }

        // --- Tree
        if line.starts_with("tree") {
            let mut args = TreeArgs { thread_override: None };
            with_ephemeral(stored, &thread, |tid_override| {
                args.thread_override = tid_override;
                tree::run_tree(args);
            });
            continue;
        }

        // Default: skip (jots already parsed by parser::parse_frs)
    }

    if !stored {
        eprintln!("{}", "⚠️ Script finished without a `store` — nothing persisted.".yellow());
    }
}

/// Run a command either with an ephemeral thread (if not stored) or directly.
fn with_ephemeral<F>(stored: bool, thread: &crate::frs::ast::Thread, mut f: F)
where
    F: FnMut(Option<String>),
{
    if !stored {
        let tid = crate::frs::persist::persist_ephemeral(thread);
        f(Some(tid.clone()));
        crate::frs::persist::cleanup_ephemeral(&tid);
    } else {
        f(None);
    }
}
