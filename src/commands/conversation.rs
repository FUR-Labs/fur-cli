use std::fs;
use std::path::Path;

use clap::Parser;
use serde_json::{Value, json};

use crate::schema::upgrade_conversation_schema;

/* --------------------- HELPERS --------------------- */

use crate::helpers::conversation::columns::handle_column_ops;
use crate::helpers::conversation::sort::handle_sorting_ops;
use crate::helpers::conversation::view::view_conversations;
use crate::helpers::conversation::tid::resolve_tid;
use crate::helpers::conversation::utils::{
    confirm_delete_primary,
    confirm_delete_destructive,
    perform_conversation_deletion,
};

/* --------------------------------------------------------------
   CLI ARGUMENT STRUCT
-------------------------------------------------------------- */

#[derive(Parser, Clone)]
pub struct ThreadArgs {

    /// Thread ID or prefix
    pub id: Option<String>,

    /// View conversation table
    #[arg(long)]
    pub view: bool,

    /// Rename a conversation
    #[arg(long, alias = "rn")]
    pub rename: Option<String>,

    /* -------------- LEGACY TAGS -------------- */
    #[arg(long)]
    pub tag: Option<String>,
    #[arg(long)]
    pub untag: Option<String>,
    #[arg(long)]
    pub clear_tags: bool,

    /* -------------- NEW COLUMN ENGINE -------------- */
    #[arg(long)]
    pub col_new: Option<String>,
    #[arg(long)]
    pub col_rename: Option<String>,
    #[arg(long)]
    pub col_add: Option<String>,
    #[arg(long)]
    pub col_remove: Option<String>,
    #[arg(long)]
    pub col_clear: Option<String>,

    /* -------------- SORTING -------------- */
    #[arg(long)]
    pub sort_by: Option<String>,
    #[arg(long)]
    pub asc: bool,
    #[arg(long)]
    pub desc: bool,

    /* -------------- MGMT -------------- */
    #[arg(long)]
    pub delete: bool,

    #[arg(long, short = 'a')]
    pub all: bool,
}

/* --------------------------------------------------------------
   ENTRYPOINT
-------------------------------------------------------------- */

pub fn run_conversation(args: ThreadArgs) {
    let fur_dir = Path::new(".fur");
    let index_path = fur_dir.join("index.json");

    if !index_path.exists() {
        eprintln!("🚨 .fur not initialized. Run `fur new`.");
        return;
    }

    let raw = fs::read_to_string(&index_path).unwrap();
    let mut index: Value = serde_json::from_str(&raw).unwrap();

    /* ============================================================
       DISPATCH IN STRICT PRIORITY ORDER
    ============================================================ */

    // 1. legacy tag system
    if args.tag.is_some() || args.untag.is_some() || args.clear_tags {
        return handle_legacy_tags(&args, &mut index, fur_dir);
    }

    // 2. GLOBAL COLUMN SYSTEM
    if args.col_new.is_some()
        || args.col_rename.is_some()
        || args.col_add.is_some()
        || args.col_remove.is_some()
        || args.col_clear.is_some()
    {
        crate::helpers::conversation::columns::handle_column_ops(
            args,
            &mut index,
            fur_dir,
        );

        let index_path = fur_dir.join("index.json");
        fs::write(&index_path, serde_json::to_string_pretty(&index).unwrap()).unwrap();
        return;
    }

    // 3. sorting
    if args.sort_by.is_some() || args.asc || args.desc {
        return handle_sorting_ops(&args, &mut index, &index_path);
    }

    // 4. rename
    if args.rename.is_some() {
        return handle_thread_rename(&args, &mut index, fur_dir);
    }

    // 5. delete
    if args.delete {
        return handle_thread_delete(&args, &mut index, fur_dir);
    }

    // 6. view table
    if args.view || args.id.is_none() {
        return handle_view_threads(&args, &mut index, fur_dir);
    }

    // 7. switch thread
    if args.id.is_some() {
        return handle_thread_switch(&args, &mut index, &index_path, fur_dir);
    }
}

/* --------------------------------------------------------------
   RENAME THREAD
-------------------------------------------------------------- */

fn handle_thread_rename(
    args: &ThreadArgs,
    index: &mut Value,
    fur_dir: &Path,
) {
    let new_title = args.rename.as_ref().unwrap().trim();

    let tid = match resolve_tid(index, &args.id) {
        Some(t) => t,
        None => return,
    };

    let convo_path = fur_dir.join("threads").join(format!("{}.json", tid));
    let raw = fs::read_to_string(&convo_path).unwrap();
    let mut convo: Value = serde_json::from_str(&raw).unwrap();

    convo = upgrade_conversation_schema(convo, index);

    let old_title = convo["title"]
        .as_str()
        .unwrap_or("Untitled")
        .to_string();  // <-- convert to String, no borrow

    convo["title"] = json!(new_title);


    fs::write(&convo_path, serde_json::to_string_pretty(&convo).unwrap()).unwrap();
    println!("✏️  Renamed \"{}\" → \"{}\"", old_title, new_title);
}

/* --------------------------------------------------------------
   DELETE THREAD
-------------------------------------------------------------- */

fn handle_thread_delete(
    args: &ThreadArgs,
    index: &mut Value,
    fur_dir: &Path,
) {
    let tid = match resolve_tid(index, &args.id) {
        Some(t) => t,
        None => return,
    };

    if !confirm_delete_primary() { return; }
    if !confirm_delete_destructive() { return; }

    let threads: Vec<String> = index["threads"]
        .as_array().unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    perform_conversation_deletion(index, fur_dir, &tid, &threads);
}

/* --------------------------------------------------------------
   VIEW TABLE
-------------------------------------------------------------- */

fn handle_view_threads(
    args: &ThreadArgs,
    index: &mut Value,
    fur_dir: &Path,
) {
    view_conversations(index, fur_dir, args.all);
}

/* --------------------------------------------------------------
   SWITCH THREAD
-------------------------------------------------------------- */

fn handle_thread_switch(
    args: &ThreadArgs,
    index: &mut Value,
    index_path: &Path,
    fur_dir: &Path,
) {
    let tid = match resolve_tid(index, &args.id) {
        Some(t) => t,
        None => return,
    };

    index["active_thread"] = json!(tid.clone());
    index["current_message"] = Value::Null;

    fs::write(index_path, serde_json::to_string_pretty(index).unwrap()).unwrap();

    let convo_path = fur_dir.join("threads").join(format!("{}.json", tid));
    let raw = fs::read_to_string(&convo_path).unwrap();
    let convo: Value = serde_json::from_str(&raw).unwrap();
    let title = convo["title"].as_str().unwrap_or("Untitled");

    println!("✔️ Switched to {} \"{}\"", &tid[..8], title);
}


fn handle_legacy_tags(
    args: &ThreadArgs,
    index: &mut Value,
    fur_dir: &Path,
) {
    // Legacy flags → convert into column ops on "tags"

    if let Some(raw) = &args.tag {
        let mut a = args.clone();
        a.col_add = Some(format!("tags={}", raw));
        return handle_column_ops(a, index, fur_dir);
    }

    if let Some(raw) = &args.untag {
        let mut a = args.clone();
        a.col_remove = Some(format!("tags={}", raw));
        return handle_column_ops(a, index, fur_dir);
    }

    if args.clear_tags {
        let mut a = args.clone();
        a.col_clear = Some("tags".into());
        return handle_column_ops(a, index, fur_dir);
    }
}
