use crate::helpers::insertion::run_insert;
use clap::Parser;
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::path::Path;

/// Subcommand: `fur msg`
#[derive(Parser, Debug)]
pub struct MsgArgs {
    /// First positional: ID prefix (only if it matches) or text
    #[arg(index = 1)]
    pub id_prefix: Option<String>,

    /// Insert before target
    #[arg(long)]
    pub pre: bool,

    /// Insert after target
    #[arg(long)]
    pub post: bool,

    #[arg(long)]
    pub edit: bool,

    #[arg(long, alias = "rem")]
    pub delete: bool,

    #[arg(long, alias = "file")]
    pub file: Option<String>,

    #[arg(long)]
    pub avatar: Option<String>,

    #[arg(long)]
    pub interactive: bool,

    /// Everything *after* the ID
    #[arg(index = 2, trailing_var_arg = true)]
    pub rest: Vec<String>,
}

/// Entry point
pub fn run_msg(args: MsgArgs) {
    if args.delete {
        return run_delete(args);
    }

    // INSERT BEFORE
    if args.pre {
        return run_insert(&args, true);
    }

    // INSERT AFTER
    if args.post {
        return run_insert(&args, false);
    }

    if args.edit {
        return run_edit(args);
    }

    eprintln!("❌ msg requires: --pre | --post | --edit | --delete");
}

//
// ======================================================
//  DELETE LOGIC
// ======================================================
//

fn run_delete(args: MsgArgs) {
    // Delete target: ID prefix OR last message
    let target = detect_id(&args.id_prefix).unwrap_or_else(|| resolve_target_message(None));

    print!("Delete message {}? [y/N]: ", &target[..8]);
    std::io::stdout().flush().unwrap();

    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf).unwrap();

    if !["y", "Y", "yes", "YES"].contains(&buf.trim()) {
        println!("❌ Cancelled.");
        return;
    }

    recursive_delete(&target);
    remove_from_parent_or_root(&target);
    update_current_after_delete(&target);

    println!("🗑️ Deleted {}", &target[..8]);
}

//
// ======================================================
//  EDIT LOGIC
// ======================================================
//

fn run_edit(args: MsgArgs) {
    let (id_opt, mut text_opt) = classify_id_or_text(&args);

    // Final target message ID
    let id = id_opt.unwrap_or_else(|| resolve_target_message(None));

    let fur = Path::new(".fur");
    let msg_path = fur.join("messages").join(format!("{}.json", id));

    let mut msg: Value = serde_json::from_str(&fs::read_to_string(&msg_path).unwrap()).unwrap();

    // Interactive override
    if args.interactive {
        let edited = run_interactive_editor(msg["text"].as_str().unwrap_or_default());
        text_opt = Some(edited);
    }

    // Apply text
    if let Some(t) = text_opt {
        msg["text"] = json!(t);
        msg["markdown"] = json!(null);
    }

    // Apply markdown
    if let Some(fp) = args.file {
        msg["markdown"] = json!(fp);
        msg["text"] = json!(null);
    }

    // Avatar change
    if let Some(a) = args.avatar {
        msg["avatar"] = json!(a);
    }

    write_json(&msg_path, &msg);

    println!("✏️ Edited {}", &id[..8]);
}

//
// ======================================================
//  POSITONAL ID RESOLUTION
// ======================================================
//

/// Detect if value looks like an ID prefix.
pub fn detect_id(x: &Option<String>) -> Option<String> {
    let Some(val) = x else { return None };

    if val.starts_with("--") {
        return None;
    }

    resolve_prefix_if_exists(val)
}

/// Determine if the call looked like:
///   msg <id> --edit new text...
/// OR:
///   msg "some text" --edit
pub fn classify_id_or_text(args: &MsgArgs) -> (Option<String>, Option<String>) {
    // Case A: First positional *could* be an ID
    if let Some(pfx) = &args.id_prefix {
        if let Some(full_id) = detect_id(&Some(pfx.clone())) {
            // ID detected
            return (Some(full_id), extract_text_from_rest(args));
        }

        // Not an ID → treat as text
        return (None, Some(pfx.clone()));
    }

    // No id_prefix → rely on rest as text
    (None, extract_text_from_rest(args))
}

/// Combine trailing args into text
fn extract_text_from_rest(args: &MsgArgs) -> Option<String> {
    if args.rest.is_empty() {
        None
    } else {
        Some(args.rest.join(" "))
    }
}

//
// ======================================================
//  PREFIX UTILITIES
// ======================================================
//

fn resolve_prefix_if_exists(pfx: &str) -> Option<String> {
    let fur = Path::new(".fur");
    let (_index, tid) = resolve_active_conversation();

    let convo_path = fur.join("threads").join(format!("{}.json", tid));
    let convo: Value = serde_json::from_str(&fs::read_to_string(&convo_path).unwrap()).unwrap();

    let root_ids = convo["messages"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|x| x.as_str().map(|s| s.to_string()))
        .collect::<Vec<_>>();

    let matches: Vec<&String> = root_ids.iter().filter(|id| id.starts_with(pfx)).collect();

    if matches.len() == 1 {
        Some(matches[0].clone())
    } else {
        None
    }
}

//
// ======================================================
//  ACTIVE CONVERSATION RESOLUTION
// ======================================================
//

fn resolve_active_conversation() -> (Value, String) {
    let idx_path = Path::new(".fur/index.json");
    let index: Value = serde_json::from_str(&fs::read_to_string(idx_path).unwrap()).unwrap();

    let tid = index["active_thread"].as_str().unwrap_or("").to_string();

    (index, tid)
}

pub fn resolve_target_message(prefix: Option<String>) -> String {
    let fur = Path::new(".fur");

    let (index, tid) = resolve_active_conversation();
    let convo_path = fur.join("threads").join(format!("{}.json", tid));

    let convo: Value = serde_json::from_str(&fs::read_to_string(&convo_path).unwrap()).unwrap();

    let root_ids = convo["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect::<Vec<_>>();

    if let Some(pfx) = prefix {
        return resolve_prefix(&root_ids, &pfx);
    }

    if let Some(cur) = index["current_message"].as_str() {
        if !cur.is_empty() {
            return cur.to_string();
        }
    }

    root_ids.last().expect("❌ No messages").to_string()
}

fn resolve_prefix(root_ids: &Vec<String>, prefix: &str) -> String {
    let matches: Vec<&String> = root_ids
        .iter()
        .filter(|id| id.starts_with(prefix))
        .collect();

    if matches.is_empty() {
        eprintln!("❌ No message matches '{}'", prefix);
        std::process::exit(1);
    }
    if matches.len() > 1 {
        eprintln!("❌ Ambiguous '{}': {:?}", prefix, matches);
        std::process::exit(1);
    }

    matches[0].to_string()
}

//
// ======================================================
//  DELETE IMPLEMENTATION
// ======================================================
//

fn recursive_delete(mid: &str) {
    let fur = Path::new(".fur");
    let msg_path = fur.join("messages").join(format!("{}.json", mid));

    let Ok(content) = fs::read_to_string(&msg_path) else {
        return;
    };
    let Ok(msg) = serde_json::from_str::<Value>(&content) else {
        return;
    };

    if let Some(children) = msg["children"].as_array() {
        for child in children {
            if let Some(cid) = child.as_str() {
                recursive_delete(cid);
            }
        }
    }

    let _ = fs::remove_file(&msg_path);
}

fn remove_from_parent_or_root(mid: &str) {
    let fur = Path::new(".fur");

    // Load deleted msg metadata (if exists)
    let msg_path = fur.join("messages").join(format!("{}.json", mid));
    let raw = fs::read_to_string(&msg_path).unwrap_or("{}".into());
    let msg: Value = serde_json::from_str(&raw).unwrap_or(json!({}));

    // If part of a thread tree
    if let Some(pid) = msg["parent"].as_str() {
        let ppath = fur.join("messages").join(format!("{}.json", pid));
        if let Ok(content) = fs::read_to_string(&ppath) {
            let mut parent: Value = serde_json::from_str(&content).unwrap();
            if let Some(arr) = parent["children"].as_array_mut() {
                arr.retain(|v| v.as_str() != Some(mid));
            }
            write_json(&ppath, &parent);
        }
        return;
    }

    // Otherwise part of root list
    let (_index, tid) = resolve_active_conversation();
    let convo_path = fur.join("threads").join(format!("{}.json", tid));

    let mut convo: Value = serde_json::from_str(&fs::read_to_string(&convo_path).unwrap()).unwrap();

    if let Some(arr) = convo["messages"].as_array_mut() {
        arr.retain(|v| v.as_str() != Some(mid));
    }

    write_json(&convo_path, &convo);
}

fn update_current_after_delete(mid: &str) {
    let fur = Path::new(".fur");
    let idx_path = fur.join("index.json");

    let mut index: Value = serde_json::from_str(&fs::read_to_string(&idx_path).unwrap()).unwrap();

    if let Some(cur) = index["current_message"].as_str() {
        if cur == mid {
            index["current_message"] = json!(null);
        }
    }

    write_json(&idx_path, &index);
}

//
// ======================================================
//  HELPERS
// ======================================================
//

fn run_interactive_editor(initial: &str) -> String {
    use std::env;
    use std::process::Command;

    let tmp = "/tmp/fur_edit_msg.txt";
    fs::write(tmp, initial).unwrap();

    let editor = env::var("EDITOR").unwrap_or("nano".into());

    Command::new(editor)
        .arg(tmp)
        .status()
        .expect("❌ Could not start editor");

    fs::read_to_string(tmp).unwrap()
}

fn write_json(path: &Path, v: &Value) {
    fs::write(path, serde_json::to_string_pretty(v).unwrap()).unwrap();
}
