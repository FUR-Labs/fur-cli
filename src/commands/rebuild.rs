use std::path::Path;

use clap::Parser;
use colored::*;

use crate::schema::rebuild::{detect_state, rebuild, ProjectState, RebuildSummary};

/// Arguments for `fur rebuild`
#[derive(Parser, Debug)]
pub struct RebuildArgs {
    /// Overwrite an existing .fur/ (discards active conversation and cursor state)
    #[arg(long)]
    pub force: bool,
}

/// `fur rebuild` — reconstruct `.fur/` from the documents in `chats/`.
pub fn run_rebuild(args: RebuildArgs) {
    let root = Path::new(".");

    match detect_state(root) {
        ProjectState::Locked => {
            println!("🔒 Archive is locked. Run `fur unlock` first.");
            return;
        }
        ProjectState::Empty => {
            println!("📭 No conversation documents found under chats/.");
            println!("   Run `fur new \"Title\"` to start a diary.");
            return;
        }
        _ => {}
    }

    match rebuild(root, args.force) {
        Ok(summary) => report(&summary),
        Err(e) => eprintln!("❌ {}", e),
    }
}

/// Runs before every command. Returns false when the command must not proceed.
///
/// viceroy: six commands used to print "🚨 .fur/ not found. Run `fur new`
/// first." That message is wrong twice over on a copied or locked archive — it
/// tells someone holding a full diary to create an empty one, which on a locked
/// project would drop a fresh `.fur/` beside the ciphertext.
pub fn preflight(exempt: bool) -> bool {
    let root = Path::new(".");

    match detect_state(root) {
        ProjectState::Locked if !exempt => {
            println!(
                "{}",
                "🔒 This project is locked.".bright_yellow().bold()
            );
            println!("{}", "   Run `fur unlock` to read it.".bright_black());
            false
        }
        ProjectState::Unindexed if !exempt => {
            auto_rebuild_if_needed();
            true
        }
        _ => true,
    }
}

/// Called once at startup. Rebuilds only when `.fur/` is absent and documents
/// exist — the copied-archive case. An existing `.fur/` is never touched, so
/// this can never silently destroy live state.
pub fn auto_rebuild_if_needed() {
    let root = Path::new(".");

    if detect_state(root) != ProjectState::Unindexed {
        return;
    }

    println!(
        "{}",
        "📦 Conversation documents found, but no .fur/ index."
            .bright_yellow()
            .bold()
    );
    println!("{}", "   Rebuilding from chats/...".bright_black());

    match rebuild(root, false) {
        Ok(summary) => report(&summary),
        Err(e) => eprintln!("❌ Rebuild failed: {}", e),
    }
}

fn report(summary: &RebuildSummary) {
    println!(
        "{}",
        format!(
            "✔ Rebuilt {} conversation(s), {} message(s).",
            summary.conversations, summary.messages
        )
        .bright_green()
        .bold()
    );

    if !summary.skipped.is_empty() {
        println!("\n{}", "Skipped documents".bold().truecolor(255, 105, 180));
        for s in &summary.skipped {
            println!("  ✖ {}", s.truecolor(255, 105, 180));
        }
    }

    if !summary.avatars.is_empty() {
        println!(
            "\n{} {}",
            "Avatars found:".bright_cyan().bold(),
            summary.avatars.join(", ").bright_white().bold()
        );
    }

    // `main` and the emoji mapping are reader preferences that no document can
    // carry, so the guess is stated loudly rather than dimmed into the noise —
    // a wrong `main` silently misattributes every future bare `fur jot`.
    if let Some(main) = &summary.guessed_main {
        println!(
            "{}",
            format!("  Assuming \"{}\" is you.", main)
                .bright_yellow()
                .bold()
        );
    }

    println!(
        "{}",
        "  → Run `fur onboard` to confirm that and pick faces."
            .bright_magenta()
            .bold()
    );
}