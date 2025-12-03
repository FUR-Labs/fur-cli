mod commands;
mod renderer;
mod frs;
mod schema;
mod utils;
mod git;
mod helpers;

use clap::{Parser, Subcommand, CommandFactory};
use clap_complete::{generate, shells::{Bash, Zsh, Fish}};
use std::io;
use crate::commands::{
    avatar,
    jot::{self, JotArgs},
    chat,
    jump::{self, JumpArgs},
    timeline::{self, TimelineArgs},
    printed,
    fork,
    status,
    tree::{self, TreeArgs},
    save::{self, SaveArgs},
    search::{self, SearchArgs},
    sweep::{self, SweepArgs},
    new,
    conversation,
    run,
    message::{self, MsgArgs},
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


    // Everyday
    #[command(about = "Everyday:: Start a new conversation (new convo)")]
    New { name: String },
    
    #[command(about = "Everyday:: Jot something out (write short form in quotes)")]
    Jot(JotArgs),

    #[command(about = "Everyday:: Paste a full chat (write long form)")]
    Chat { avatar: Option<String> },

    #[command(about = "Everyday:: Edit or Delete a jotted message")]
    Msg(MsgArgs),

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

    // Management
    #[command(about = "Management:: See or create new conversation avatars/personas")]

    Avatar {
        #[command(subcommand)]
        action: Option<AvatarAction>,
        #[arg(long)]
        view: bool,
    },

    #[command(about = "Management:: See or jump to any of your conversations", visible_alias = "conversation")]
    Convo(conversation::ThreadArgs),
    
    #[command(about = "Management:: Tree of full conversation")]
    Tree(TreeArgs),
    #[command(visible_aliases = ["scan", "sweep"])]

    #[command(about = "Management:: Global search of all your FUR projects (full PC, blazing fast)")]
    Gsearch(SweepArgs),

    // Scripting
    #[command(about = "Scripting:: Run a .frs (FurScript) script with `fur <script.frs>`)")]
    Run { path: String },
    
    #[command(about = "Scripting:: Save your conversation as a .frs script")]
    Save(SaveArgs),


    // Experimental
    #[command(about = "Under development:: Fork / Copy")]
    Fork {
        #[arg(short, long, default_value = "")]
        id: String,
        #[arg(short, long)]
        title: Option<String>,
    },
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
            let args = status::StatusArgs { conversation_override: None };
            status::run_status(args);
            if let Some(repo) = utils::git::find_git_root() {
                git::status::run_git_status(&repo);
            }
        }
        GitCmd::Add(args)    => git::passthrough::passthrough("add", &args),
        GitCmd::Commit(args) => git::passthrough::passthrough("commit", &args),
        GitCmd::Push(args)   => git::passthrough::passthrough("push", &args),
        GitCmd::Pull(args)   => git::passthrough::passthrough("pull", &args),
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
        Commands::Printed { out, verbose } => printed::run_printed(out, verbose),

        Commands::Avatar { action, view: _ } => {
            match action {
                Some(AvatarAction::New) => avatar::run_avatar_onboarding(),
                None => avatar::run_avatar_view(),
            }
        }

        Commands::Convo(a) => conversation::run_conversation(a),
        Commands::Tree(a) => tree::run_tree(a),
        Commands::Search(a) => search::run_search(a),
        Commands::Gsearch(a) => sweep::run_sweep(a),

        Commands::Run { path } => run::run_frs(&path),
        Commands::Save(a) => save::run_save(a),

        Commands::Fork { id, title } => {
            if id.is_empty() {
                fork::run_fork_from_active(title);
            } else {
                fork::run_fork(&id, title);
            }
        }

        Commands::Jump(a) => {
            if let Err(e) = jump::run_jump(a) {
                eprintln!("Error: {}", e);
            }
        }

    }
}
