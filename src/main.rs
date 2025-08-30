mod commands;
mod utils;

use clap::{Parser, Subcommand};
use crate::commands::{
    avatar,
    jot::{self, JotArgs},
    jump::{self, JumpArgs},
    timeline::{self, TimelineArgs},
    fork,
    status,
    tree,
    cat::{self, CatArgs},
};

#[derive(Parser)]
#[command(
    name = "fur",
    about = "FUR — Forkable, Unearthable, Recursive memory tracker",
    long_about = "Track, branch, and link your conversations, especially AI chats, using local files and JSON. Think of it like git for your ideas."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Set or modify avatars for users
    Avatar {
        /// The name of the avatar (e.g., "james", "ai", "girlfriend")
        avatar: String,

        /// Flag for creating non-main avatars (e.g., "ai", "girlfriend")
        #[arg(short, long)]
        other: bool,

        /// Emoji for the avatar
        #[arg(short, long)]
        emoji: String,
    },

    /// Start a new conversation
    New {
        #[arg(help = "Name for the new thread")]
        name: String,
    },

    /// Show current thread/message state
    Status {},

    /// Fork the current message into a new thread
    Fork {
        /// ID of the message to fork from (optional)
        #[arg(short, long, default_value = "")]
        id: String,
    },

    /// Jump to another message in the thread
    ///
    /// You can go back (past), forward (child), or jump to a specific ID.
    Jump(JumpArgs),

    /// Add a new message or link a markdown file
    ///
    /// Use `--file` to attach a markdown document.
    Jot(JotArgs),

    /// Show the thread as a linear timeline
    Timeline(TimelineArgs),  // Add verbose flag here

    /// Show the thread as a branching tree
    Tree {},

    /// Print full contents of a markdown-linked message
    Cat(CatArgs),
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Avatar { avatar, other, emoji } => {
            avatar::run_avatar(avatar, other, emoji);
        }

        Commands::New { name } => commands::new::run_new(name),

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

        Commands::Timeline(args) => timeline::run_timeline(args),  // Pass args here

        Commands::Tree {} => tree::run_tree(),

        Commands::Cat(args) => cat::run_cat(args),
    }
}
