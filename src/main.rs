mod commands;
mod renderer;
mod frs;

use clap::{Parser, Subcommand, CommandFactory};
use clap_complete::{generate, shells::{Bash, Zsh, Fish}};
use std::io;
use crate::commands::{
    avatar,
    jot::{self, JotArgs},
    jump::{self, JumpArgs},
    timeline::{self, TimelineArgs},
    fork,
    status,
    tree::{self, TreeArgs},
    save::{self, SaveArgs},
    new,
    thread,
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
    /// Generate shell completions
    #[command(hide = true)]
    Completions {
        #[arg(value_parser = ["bash", "zsh", "fish"])]
        shell: String,
    },

    /// Manage avatars
    Avatar {
        /// The name of the avatar (e.g., "james", "ai", "girlfriend")
        avatar: Option<String>,

        /// Flag for creating non-main avatars
        #[arg(short, long)]
        other: bool,

        /// Emoji for the avatar
        #[arg(short, long)]
        emoji: Option<String>,

        /// View all avatars
        #[arg(long)]
        view: bool,
    },

    /// Start a new conversation
    New {
        #[arg(help = "Name for the new thread")]
        name: String,
    },

    /// Show current thread/message state
    Status {},

    /// Manage threads (list or switch)
    Thread(thread::ThreadArgs),


    /// Fork the current message into a new thread
    Fork {
        /// ID of the message to fork from (optional)
        #[arg(short, long, default_value = "")]
        id: String,

        /// Optional custom title for the new fork
        #[arg(short, long)]
        title: Option<String>,
    },

    /// Jump to another message in the thread
    Jump(JumpArgs),

    /// Add a new message or link a markdown file
    ///
    /// Use `--file` to attach a markdown document.
    Jot(JotArgs),

    /// Show the thread as a linear timeline
    Timeline(TimelineArgs),  // Pass verbose flag here

    /// Show the thread as a branching tree
    Tree(TreeArgs),

    /// Import a .frs file
    Load {
        #[arg(help = "Path to the .frs file")]
        path: String,
    },

    /// Save threads/messages
    Save(SaveArgs),
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            match shell.as_str() {
                "bash" => generate(Bash, &mut cmd, "fur", &mut io::stdout()),
                "zsh"  => generate(Zsh,  &mut cmd, "fur", &mut io::stdout()),
                "fish" => generate(Fish, &mut cmd, "fur", &mut io::stdout()),
                _ => eprintln!("Unsupported shell: {}", shell),
            }
        }

        Commands::Avatar { avatar, other, emoji, view } => {
            avatar::run_avatar(avatar, other, emoji, view);
        }

        Commands::New { name } => new::run_new(name),

        Commands::Status {} => status::run_status(),

        Commands::Thread(args) => {
            thread::run_thread(args);
        }

        Commands::Fork { id, title } => {
            if id.is_empty() {
                fork::run_fork_from_active(title.clone());
            } else {
                fork::run_fork(&id, title.clone());
            }
        }

        Commands::Jump(args) => {
            if let Err(e) = jump::run_jump(args) {
                eprintln!("Error: {}", e);
            }
        }

        Commands::Jot(args) => jot::run_jot(args),

        Commands::Timeline(args) => timeline::run_timeline(args),  // Pass args here

        Commands::Tree(args) => tree::run_tree(args),

        Commands::Load { path } => {
            let thread = frs::import_frs(&path);
            let thread_id = frs::persist_frs(&thread);
            println!("✔️ Saved as thread {}", &thread_id[..8]);
        }

        Commands::Save(args) => save::run_save(args),
    }
}
