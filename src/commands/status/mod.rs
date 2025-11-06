pub mod core;
pub mod render;

use std::fs;
use std::path::Path;
use std::collections::HashMap;

use colored::Colorize;
use serde_json::{Value, json};

use self::core::*;
use self::render::*;

use clap::Parser;


#[derive(Parser, Debug)]
pub struct StatusArgs {
    /// Optional thread override (used by `fur run` for ephemeral runs)
    #[clap(skip)]
    pub thread_override: Option<String>,
}

pub fn run_status(args: StatusArgs) {
    let fur_dir = Path::new(".fur");
    let index_path = fur_dir.join("index.json");

    if !index_path.exists() {
        eprintln!("{}", "🚨 .fur/ not found. Run `fur new` first.".red().bold());
        return;
    }

    let avatars: Value = serde_json::from_str(
        &fs::read_to_string(fur_dir.join("avatars.json")).unwrap_or_else(|_| "{}".to_string())
    ).unwrap_or(json!({}));

    let (index, mut thread, mut current_msg_id) =
        load_index_and_thread(&fur_dir);

    if let Some(ref tid) = args.thread_override {
        let thread_path = fur_dir.join("tmp").join(format!("{}.json", tid));
        if let Ok(content) = fs::read_to_string(&thread_path) {
            if let Ok(tmp_thread) = serde_json::from_str::<Value>(&content) {
                thread = tmp_thread;
            }
        }
    }

    let id_to_message = build_id_to_message(&fur_dir, &thread);

    if current_msg_id.is_empty() {
        current_msg_id = first_message_fallback(&thread);
    }

    render_status_ui(
        &index,
        &thread,
        &id_to_message,
        &current_msg_id,
        &avatars
    );
}


fn render_status_ui(
    index: &Value,
    thread: &Value,
    id_to_message: &HashMap<String, Value>,
    current_msg_id: &str,
    avatars: &Value
) {
    // Active thread
    print_active_thread(index);

    // Current message
    print_current_message(current_msg_id);

    println!("{}", "─────────────────────────────".bright_black());

    // Lineage (ancestors)
    print_lineage(
        id_to_message,
        current_msg_id,
        avatars
    );

    println!("{}", "─────────────────────────────".bright_black());
    println!("{}", "Next messages from here:".bright_magenta().bold());

    // Children + siblings
    print_next_messages(
        id_to_message,
        thread,
        current_msg_id,
        avatars
    );
}
