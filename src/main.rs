mod commands;

use clap::{Parser, Subcommand};
use crate::commands::{/* new, */ scribe::ScribeArgs, scribe, timeline, fork};
use crate::commands::{jump, JumpArgs};

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

    /// Fork an existing thread
    Fork {
        #[arg(short, long, default_value = "")]
        id: String,
    },

    /// Jump to another message (past, child, or ID)
    Jump(JumpArgs),

    /// Add message to current thread
    Scribe(ScribeArgs),

    /// Show thread timeline
    Timeline {},

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

        Commands::Scribe(args) => scribe::run_scribe(args),

        Commands::Timeline {} => timeline::run_timeline(),

        Commands::Frostmark {} => {
            println!("Embedding frostmark...");
        }
        Commands::Unearth { id } => {
            println!("Unearthing fork ID: {}", id);
        }
    }
}
