use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "fur", version = "0.1", author = "You", about = "Recursive chat versioning tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start a new conversation
    New {
        #[arg(short, long)]
        title: Option<String>,
    },

    /// Fork an existing thread
    Fork {
        #[arg(short, long)]
        id: String,
    },

    /// Add message to current thread
    Scribe {
        #[arg(short, long)]
        message: String,
    },

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
        Commands::New { title } => {
            println!("Starting new thread: {:?}", title.unwrap_or_else(|| "untitled".into()));
        }
        Commands::Fork { id } => {
            println!("Forking thread ID: {}", id);
        }
        Commands::Scribe { message } => {
            println!("Appending message: {}", message);
        }
        Commands::Timeline {} => {
            println!("Displaying thread timeline...");
        }
        Commands::Frostmark {} => {
            println!("Embedding frostmark...");
        }
        Commands::Unearth { id } => {
            println!("Unearthing fork ID: {}", id);
        }
    }
}
