use clap::Parser;
use serde_json::{Value, json};
use std::fs;
use std::io::{Write};
use std::path::{Path};

/// Subcommand: `fur msg`
#[derive(Parser, Debug)]
pub struct MsgArgs {
    /// Delete a message by prefix or full ID
    #[arg(long, alias = "rem")]
    pub delete: Option<String>,

    /// Edit a message by prefix or full ID
    #[arg(long)]
    pub edit: Option<String>,

    /// New text content (for editing)
    #[arg(long)]
    pub text: Option<String>,

    /// New markdown file (for editing)
    #[arg(long, alias = "file")]
    pub file: Option<String>,

    /// Change avatar (for editing)
    #[arg(long)]
    pub avatar: Option<String>,

    /// Open in $EDITOR
    #[arg(long)]
    pub interactive: bool,
}

/// Entry point
pub fn run_msg(args: MsgArgs) {
    if let Some(prefix) = &args.delete {
        return delete_entry(Some(prefix.clone()));
    }

    if let Some(prefix) = &args.edit {
        return edit_entry(Some(prefix.clone()), args);
    }

    // No hash provided → use current or last (delete)
    if args.text.is_none() && args.file.is_none() && args.avatar.is_none() {
        return delete_entry(None);
    }

    // No hash provided → editing default target
    edit_entry(None, args);
}

//
// ======================================================
//  ID RESOLUTION
// ======================================================
//

fn resolve_active_conversation() -> (Value, String) {
    let index_path = Path::new(".fur/index.json");
    let index: Value =
        serde_json::from_str(&fs::read_to_string(index_path).unwrap()).unwrap();

    let tid = index["active_thread"]
        .as_str()
        .unwrap_or("")
        .to_string();

    (index, tid)
}

fn resolve_target_message(prefix: Option<String>) -> String {
    let fur_dir = Path::new(".fur");

    let (index, convo_id) = resolve_active_conversation();

    let convo_path = fur_dir.join("threads").join(format!("{}.json", convo_id));
    let convo: Value =
        serde_json::from_str(&fs::read_to_string(&convo_path).unwrap()).unwrap();

    let root = convo["messages"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect::<Vec<String>>();

    // If prefix provided → resolve prefix
    if let Some(p) = prefix {
        return resolve_prefix(&root, &p);
    }

    // No prefix → try current_message
    if let Some(cur) = index["current_message"].as_str() {
        if !cur.is_empty() {
            return cur.to_string();
        }
    }

    // Else → use last root message
    root.last()
        .expect("❌ No messages in this conversation.")
        .to_string()
}

fn resolve_prefix(root_ids: &Vec<String>, prefix: &str) -> String {
    let matches: Vec<&String> = root_ids
        .iter()
        .filter(|id| id.starts_with(prefix))
        .collect();

    if matches.is_empty() {
        eprintln!("❌ No message matches prefix '{}'", prefix);
        std::process::exit(1);
    }

    if matches.len() > 1 {
        eprintln!("❌ Ambiguous prefix '{}': {:?}", prefix, matches);
        std::process::exit(1);
    }

    matches[0].to_string()
}

//
// ======================================================
//  DELETE MESSAGE
// ======================================================
//

fn delete_entry(prefix: Option<String>) {
    let target_id = resolve_target_message(prefix);

    print!("Delete message {}? [y/N]: ", &target_id[..8]);
    std::io::stdout().flush().unwrap();

    let mut confirm = String::new();
    std::io::stdin().read_line(&mut confirm).unwrap();
    if !["y", "Y", "yes", "YES"].contains(&confirm.trim()) {
        println!("❌ Cancelled.");
        return;
    }

    recursive_delete(&target_id);
    remove_from_parent_or_root(&target_id);
    update_current_after_delete(&target_id);

    println!("🗑️ Deleted message {}", &target_id[..8]);
}

fn recursive_delete(mid: &str) {
    let fur_dir = Path::new(".fur");
    let msg_path = fur_dir.join("messages").join(format!("{}.json", mid));

    let content = match fs::read_to_string(&msg_path) {
        Ok(c) => c,
        Err(_) => return, // Message already deleted, nothing to do
    };

    let msg: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return, // Malformed or partially deleted; ignore
    };


    // children only (no branches)
    if let Some(children) = msg["children"].as_array() {
        for c in children {
            if let Some(cid) = c.as_str() {
                recursive_delete(cid);
            }
        }
    }

    // Delete this JSON
    let _ = fs::remove_file(&msg_path);
}

fn remove_from_parent_or_root(mid: &str) {
    let fur_dir = Path::new(".fur");

    // Load message to find parent
    let msg_path = fur_dir.join("messages").join(format!("{}.json", mid));
    let msg_raw = fs::read_to_string(&msg_path).unwrap_or("{}".into());
    let msg: Value = serde_json::from_str(&msg_raw).unwrap_or(json!({}));

    // If parent: remove from parent's children
    if let Some(pid) = msg["parent"].as_str() {
        let p_path = fur_dir.join("messages").join(format!("{}.json", pid));
        if let Ok(content) = fs::read_to_string(&p_path) {
            let mut parent: Value = serde_json::from_str(&content).unwrap();
            if let Some(arr) = parent["children"].as_array_mut() {
                arr.retain(|v| v.as_str() != Some(mid));
            }
            write_json(&p_path, &parent);
        }
        return;
    }

    // Else: remove from conversation root
    let (_index, tid) = resolve_active_conversation();
    let convo_path = fur_dir.join("threads").join(format!("{}.json", tid));

    let mut convo: Value =
        serde_json::from_str(&fs::read_to_string(&convo_path).unwrap()).unwrap();

    if let Some(arr) = convo["messages"].as_array_mut() {
        arr.retain(|v| v.as_str() != Some(mid));
    }

    write_json(&convo_path, &convo);
}

fn update_current_after_delete(mid: &str) {
    let fur_dir = Path::new(".fur");
    let index_path = fur_dir.join("index.json");
    let mut index: Value =
        serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();

    if let Some(cur) = index["current_message"].as_str() {
        if cur == mid {
            index["current_message"] = json!(null);
        }
    }

    write_json(&index_path, &index);
}

//
// ======================================================
//  EDIT MESSAGE
// ======================================================
//

fn edit_entry(prefix: Option<String>, args: MsgArgs) {
    let mid = resolve_target_message(prefix);

    let fur_dir = Path::new(".fur");
    let msg_path = fur_dir.join("messages").join(format!("{}.json", mid));

    let mut msg: Value =
        serde_json::from_str(&fs::read_to_string(&msg_path).unwrap()).unwrap();

    // Interactive edit: open $EDITOR on a temp file
    if args.interactive {
        let new_text = run_interactive_editor(
            msg["text"].as_str().unwrap_or_default()
        );
        msg["text"] = json!(new_text);
    }

    if let Some(t) = args.text {
        msg["text"] = json!(t);
    }

    if let Some(f) = args.file {
        msg["markdown"] = json!(f);
        msg["text"] = json!(null);
    }

    if let Some(a) = args.avatar {
        msg["avatar"] = json!(a);
    }

    write_json(&msg_path, &msg);

    println!("✏️ Edited message {}", &mid[..8]);
}

//
// ======================================================
//  HELPERS
// ======================================================
//

fn run_interactive_editor(initial: &str) -> String {
    use std::process::Command;
    use std::env;

    let tmp = "/tmp/fur_edit_msg.txt";
    fs::write(tmp, initial).unwrap();

    let editor = env::var("EDITOR").unwrap_or("nano".into());

    Command::new(editor)
        .arg(tmp)
        .status()
        .expect("❌ Failed to launch editor");

    fs::read_to_string(tmp).unwrap()
}

fn write_json(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_string_pretty(value).unwrap()).unwrap();
}
