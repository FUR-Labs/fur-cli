mod commands;
pub mod utils;

use clap::{Parser, Subcommand};
use crate::commands::{/* new, */ jot::JotArgs, jot, timeline, fork};
use crate::commands::{jump, JumpArgs};
use crate::commands::{status, tree};

#[derive(Parser)]
#[command(name = "fur")]
#[command(about = "FUR — Forkable, Unearthable, Recursive memory tracker")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a new conversation
    New,

    /// Show current thread/message state
    Status {},

    /// Fork an existing thread
    Fork {
        #[arg(short, long, default_value = "")]
        id: String,
    },

    /// Jump to another message (past, child, or ID)
    Jump(JumpArgs),

    /// Add message to current thread
    Jot(JotArgs),

    /// Show thread timeline
    Timeline {},

    /// Show tree of message ancestry & children
    Tree {},

    /// Embed a compressed summary (like a frostmark)
    Frostmark {},

    /// Unearth a sacrificed thread
    Unearth {
        #[arg(short, long)]
        id: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::New => commands::new::run_new(),

        Commands::Status {} => status::run_status(),

        Commands::Fork { id } => {
            if id.is_empty() {
                fork::run_fork_from_active()
            } else {
                fork::run_fork(&id)
            }
        }

        Commands::Jump(args) => {
            if let Err(e) = jump::run_jump(args) {
                eprintln!("Error: {}", e);
            }
        }

        Commands::Jot(args) => jot::run_jot(args),

        Commands::Timeline {} => timeline::run_timeline(),

        Commands::Tree {} => tree::run_tree(),

        Commands::Frostmark {} => {
            println!("Embedding frostmark...");
        }
        Commands::Unearth { id } => {
            println!("Unearthing fork ID: {}", id);
        }
    }
}
