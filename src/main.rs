pub mod avatars;
mod commands;
mod frs;
mod git;
mod helpers;
mod renderer;
mod schema;
mod security;
mod utils;
use colored::Colorize;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{
    generate,
    shells::{Bash, Fish, Zsh},
};
use serde_json::Value;
use std::fs;
use std::io;
use std::path::Path;

use crate::commands::{
    avatar, chat, clone, conversation,
    doctor::{self, DoctorArgs},
    export::{self, ExportArgs},
    jot::{self, JotArgs},
    jump::{self, JumpArgs},
    message::{self, MsgArgs},
    new,
    onboard::{self, OnboardArgs},
    printed,
    rebuild::{self, RebuildArgs},
    run,
    save::{self, SaveArgs},
    search::{self, SearchArgs},
    status,
    sweep::{self, SweepArgs},
    timeline::{self, TimelineArgs},
    tree::{self, TreeArgs},
    xclone,
};

#[derive(Parser)]
#[command(
    name = "fur",
    version,
    about = "FUR: AI Conversation Control System",
    long_about = "FUR turns your AI conversations into a digital diary. A solution to the long-term memory problem of AI chats."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    // Hidden completions
    #[command(hide = true)]
    Completions {
        #[arg(value_parser = ["bash", "zsh", "fish"])]
        shell: String,
    },

    // Git passthrough
    Status {},
    Add {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    Commit {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    Push {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    Pull {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    #[command(about = "Security:: Encrypt the current fur diary")]
    Lock(security::lock::LockArgs),

    #[command(about = "Security:: Decrypt the current fur diary")]
    Unlock(security::unlock::UnlockArgs),

    #[command(about = "Security:: Generate a Diceware passphrase for encryption")]
    Keygen {
        #[arg(short, long, default_value_t = 6)]
        words: usize,
    },

    // Everyday
    #[command(about = "Everyday:: Start a new conversation (new convo)")]
    New {
        name: String,
    },

    #[command(about = "Everyday:: Jot something out (write short form in quotes)")]
    Jot(JotArgs),

    #[command(about = "Everyday:: Paste a full chat (write long form)")]
    Chat {
        avatar: Option<String>,
    },

    #[command(about = "Everyday:: Edit or Delete a jotted message")]
    Msg(MsgArgs),

    #[command(about = "Everyday:: Show full conversation (replaces timeline --verbose)")]
    Show(TimelineArgs),

    #[command(about = "Everyday:: Print timeline of full conversation.")]
    Timeline(TimelineArgs),

    #[command(about = "Everyday:: Export full conversation as Markdown")]
    Printed {
        out: Option<String>,
        #[arg(short, long)]
        verbose: bool,
    },

    #[command(about = "Everyday:: Search all conversations for text or markdown matches")]
    Search(SearchArgs),

    #[command(about = "Everyday:: Export conversations as canonical Markdown into chats/")]
    Export(ExportArgs),

    #[command(about = "Management:: Repair missing or moved attachments")]
    Doctor(DoctorArgs),

    #[command(about = "Management:: Rebuild .fur/ from the documents in chats/")]
    Rebuild(RebuildArgs),

    #[command(about = "Management:: Set which avatar is you, and pick faces")]
    Onboard(OnboardArgs),

    // Management
    #[command(about = "Management:: See or create new conversation avatars/personas")]
    Avatar {
        #[command(subcommand)]
        action: Option<AvatarAction>,
        #[arg(long)]
        view: bool,
    },

    #[command(about = "Management:: Clone a conversation safely (deep copy)")]
    Clone {
        /// Optional: conversation ID or prefix
        #[arg(short, long, default_value = "")]
        id: String,

        /// Optional: custom title for the new conversation
        #[arg(short, long)]
        title: Option<String>,
    },

    #[command(about = "Management:: Deep clone a conversation into another .fur project", visible_aliases = ["xc"])]
    Xclone {
        /// Path to the target project (folder containing .fur/)
        #[arg(long)]
        to: String,

        /// Conversation ID or prefix (defaults to active thread)
        #[arg(short, long, default_value = "")]
        id: String,

        /// Optional custom title for the cloned conversation
        #[arg(short, long)]
        title: Option<String>,
    },

    #[command(
        about = "Management:: See or jump to any of your conversations",
        visible_alias = "conversation"
    )]
    Convo(conversation::ThreadArgs),

    #[command(about = "Management:: Tree of full conversation")]
    Tree(TreeArgs),
    #[command(about = "Management:: Scan for FUR projects beneath the current directory")]
    Scan {
        /// Maximum recursion depth
        #[arg(long, default_value_t = 5)]
        depth: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Suppress warnings
        #[arg(long)]
        silent: bool,
    },

    #[command(
        about = "Management:: Global search of all your FUR projects (starting from your home directory)"
    )]
    Gsearch(SweepArgs),

    // Scripting
    #[command(about = "Scripting:: Run a .frs (FurScript) script with `fur <script.frs>`)")]
    Run {
        path: String,
    },

    #[command(about = "Scripting:: Save your conversation as a .frs script")]
    Save(SaveArgs),

    // Experimental
    #[command(about = "Under development:: Jump to specific chat within convo")]
    Jump(JumpArgs),
}

#[derive(Subcommand)]
enum AvatarAction {
    New,
}

enum GitCmd {
    Status,
    Add(Vec<String>),
    Commit(Vec<String>),
    Push(Vec<String>),
    Pull(Vec<String>),
}

fn dispatch_git(cmd: GitCmd) {
    match cmd {
        GitCmd::Status => {
            let args = status::StatusArgs {
                conversation_override: None,
            };
            status::run_status(args);
            if let Some(repo) = utils::git::find_git_root() {
                git::status::run_git_status(&repo);
            }
        }
        GitCmd::Add(args) => git::passthrough::passthrough("add", &args),
        GitCmd::Commit(args) => git::passthrough::passthrough("commit", &args),
        GitCmd::Push(args) => git::passthrough::passthrough("push", &args),
        GitCmd::Pull(args) => git::passthrough::passthrough("pull", &args),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // .frs shortcut
    if args.len() == 2 && args[1].ends_with(".frs") {
        run::run_frs(&args[1]);
        return;
    }

    let cli = Cli::parse();

    // Locked archives, and copied archives with no .fur/ index.
    let exempt = matches!(
        cli.command,
        Commands::Unlock(_) | Commands::Keygen { .. } | Commands::Completions { .. }
    );
    if !commands::rebuild::preflight(exempt) {
        return;
    }

    let skip_schema_migration = std::env::var_os("FUR_SKIP_SCHEMA_MIGRATION").is_some();

    if !skip_schema_migration && schema::detect_old_schema() {
        println!("{}", "⚠ Older FUR schema detected.".bright_yellow().bold());

        if schema::ask_yes_no("Run metadata migration now?") {
            schema::run_backfill_meta();
        } else {
            println!("Skipping migration.\n");
        }
    }

    match cli.command {
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();

            match shell.as_str() {
                "bash" => generate(Bash, &mut cmd, "fur", &mut io::stdout()),
                "zsh" => generate(Zsh, &mut cmd, "fur", &mut io::stdout()),
                "fish" => generate(Fish, &mut cmd, "fur", &mut io::stdout()),
                _ => eprintln!("Unsupported shell: {}", shell),
            }
        }

        Commands::Lock(args) => security::lock::run_lock(args),
        Commands::Unlock(args) => security::unlock::run_unlock(args),
        Commands::Keygen { words } => {
            let (pass, entropy) = security::crypto::generate_password(words);

            println!("\nGenerated passphrase:\n{}\n", pass);

            println!("Entropy ≈ {:.1} bits\n", entropy);

            if entropy < 70.0 {
                println!("⚠ Warning: low entropy passphrase (recommended ≥ 6 words)");
            }
        }
        Commands::Status {} => dispatch_git(GitCmd::Status),
        Commands::Add { args } => dispatch_git(GitCmd::Add(args)),
        Commands::Commit { args } => dispatch_git(GitCmd::Commit(args)),
        Commands::Push { args } => dispatch_git(GitCmd::Push(args)),
        Commands::Pull { args } => dispatch_git(GitCmd::Pull(args)),

        Commands::New { name } => new::run_new(name),

        Commands::Jot(a) => jot::run_jot(a),

        Commands::Chat { avatar } => chat::run_chat(avatar),

        Commands::Msg(a) => message::run_msg(a),

        Commands::Timeline(a) => timeline::run_timeline(a),

        Commands::Show(a) => {
            let mut a = a;
            a.verbose = true; // force verbose mode
            timeline::run_timeline(a);
        }

        Commands::Printed { out, verbose } => printed::run_printed(out, verbose),

        Commands::Doctor(args) => doctor::run_doctor(args),

        Commands::Rebuild(args) => rebuild::run_rebuild(args),

        Commands::Onboard(args) => onboard::run_onboard(args),

        Commands::Avatar { action, .. } => match action {
            Some(AvatarAction::New) => avatar::run_avatar_onboarding(),
            None => avatar::run_avatar_view(),
        },

        Commands::Convo(a) => conversation::run_conversation(a),
        Commands::Xclone { to, id, title } => {
            // If no id given, fall back to active thread
            let tid_to_clone = if id.is_empty() {
                let index_path = Path::new(".fur").join("index.json");
                let index: Value = serde_json::from_str(&fs::read_to_string(&index_path).unwrap())
                    .expect("❌ Failed to read index.json");
                index["active_thread"]
                    .as_str()
                    .expect("❌ No active thread set")
                    .to_string()
            } else {
                id
            };

            xclone::run_xclone(&to, &tid_to_clone, title);
        }

        Commands::Tree(a) => tree::run_tree(a),
        Commands::Search(a) => search::run_search(a),
        Commands::Export(a) => export::run_export(a),

        Commands::Scan {
            depth,
            json,
            silent,
        } => {
            sweep::run_sweep(sweep::SweepArgs {
                dir: Some(".".to_string()),
                depth,
                json,
                silent,
            });
        }

        Commands::Gsearch(a) => sweep::run_sweep(a),

        Commands::Run { path } => run::run_frs(&path),

        Commands::Save(a) => save::run_save(a),

        Commands::Clone { id, title } => {
            if id.is_empty() {
                clone::run_clone_from_active(title);
            } else {
                clone::run_clone(&id, title);
            }
        }

        Commands::Jump(a) => {
            if let Err(e) = jump::run_jump(a) {
                eprintln!("Error: {}", e);
            }
        }
    }
}
