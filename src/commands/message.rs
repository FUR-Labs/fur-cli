use clap::Parser;
use serde_json::{Value, json};
use std::fs;
use std::io::{Write};
use std::path::Path;

/// Subcommand: `fur msg`
#[derive(Parser, Debug)]
pub struct MsgArgs {
    /// First positional: ID prefix (only if it matches) or text
    #[arg(index = 1)]
    pub id_prefix: Option<String>,

    /// Second positional: text
    #[arg(index = 2)]
    pub text_value: Option<String>,

    #[arg(long)]
    pub edit: bool,

    #[arg(long, alias="rem")]
    pub delete: bool,

    #[arg(long, alias="file")]
    pub file: Option<String>,

    #[arg(long)]
    pub avatar: Option<String>,

    #[arg(long)]
    pub interactive: bool,
}

/// Entry point
pub fn run_msg(args: MsgArgs) {
    if args.delete {
        return run_delete(args);
    }
    run_edit(args);
}

//
// ======================================================
//  DELETE LOGIC
// ======================================================
//

fn run_delete(args: MsgArgs) {
    // Delete target: ID prefix OR last message
    let target = detect_id(&args.id_prefix)
        .unwrap_or_else(|| resolve_target_message(None));

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
    let (id_opt, mut new_text) =
        classify_id_and_text(args.id_prefix, args.text_value);

    // Final target message ID
    let mid = id_opt.unwrap_or_else(|| resolve_target_message(None));

    let fur = Path::new(".fur");
    let msg_path = fur.join("messages").join(format!("{}.json", mid));

    let mut msg: Value =
        serde_json::from_str(&fs::read_to_string(&msg_path).unwrap()).unwrap();

    // Interactive override
    if args.interactive {
        let edited = run_interactive_editor(
            msg["text"].as_str().unwrap_or_default()
        );
        new_text = Some(edited);
    }

    // Apply text
    if let Some(t) = new_text {
        msg["text"] = json!(t);
        msg["markdown"] = json!(null);
    }

    // Apply markdown
    if let Some(fpath) = args.file {
        msg["markdown"] = json!(fpath);
        msg["text"] = json!(null);
    }

    // Avatar change
    if let Some(a) = args.avatar {
        msg["avatar"] = json!(a);
    }

    write_json(&msg_path, &msg);

    println!("✏️ Edited {}", &mid[..8]);
}

//
// ======================================================
//  POSITONAL ARG PARSING LOGIC
// ======================================================
//

/// Detect if a value looks like a message ID prefix.
/// Returns Some(full_id) or None.
fn detect_id(x: &Option<String>) -> Option<String> {
    let Some(val) = x else { return None; };

    // positional that begins with "--" cannot be ID
    if val.starts_with("--") {
        return None;
    }

    // Try to match existing prefix
    if let Some(id) = resolve_prefix_if_exists(val) {
        return Some(id);
    }

    None
}

/// Interpret positionals into (id, text)
///
/// Rules:
///   - If first positional matches a prefix → ID
///   - Second positional always text
///   - If first positional does NOT match → treat as text
fn classify_id_and_text(
    id_prefix: Option<String>,
    text_value: Option<String>
) -> (Option<String>, Option<String>) {

    // Case 1: first positional is a valid ID prefix
    if id_prefix.is_some() {
        if let Some(real_id) = detect_id(&id_prefix) {
            return (Some(real_id), text_value);
        }
    }

    // Case 2: first positional is actually text
    if let Some(val) = id_prefix {
        return (None, Some(val));
    }

    // Case 3: only second positional is provided
    if let Some(val) = text_value {
        return (None, Some(val));
    }

    (None, None)
}


/// Internal helper: check for prefix match safely  
fn resolve_prefix_if_exists(pfx: &str) -> Option<String> {
    let fur = Path::new(".fur");
    let (_index, tid) = resolve_active_conversation();

    let convo_path = fur.join("threads").join(format!("{}.json", tid));
    let convo: Value =
        serde_json::from_str(&fs::read_to_string(&convo_path).unwrap()).unwrap();

    let root = convo["messages"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|x| x.as_str().map(|s| s.to_string()))
        .collect::<Vec<String>>();

    let matches: Vec<&String> =
        root.iter().filter(|id| id.starts_with(pfx)).collect();

    if matches.len() == 1 {
        Some(matches[0].clone())
    } else {
        None
    }
}

//
// ======================================================
//  ID RESOLUTION HELPERS
// ======================================================
//

fn resolve_active_conversation() -> (Value, String) {
    let idx_path = Path::new(".fur/index.json");
    let index: Value =
        serde_json::from_str(&fs::read_to_string(idx_path).unwrap()).unwrap();
    let tid = index["active_thread"].as_str().unwrap_or("").to_string();
    (index, tid)
}

fn resolve_target_message(prefix: Option<String>) -> String {
    let fur = Path::new(".fur");

    let (index, tid) = resolve_active_conversation();
    let convo_path = fur.join("threads").join(format!("{}.json", tid));
    let convo: Value =
        serde_json::from_str(&fs::read_to_string(&convo_path).unwrap()).unwrap();

    let root = convo["messages"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect::<Vec<String>>();

    if let Some(p) = prefix {
        return resolve_prefix(&root, &p);
    }

    // current_message wins
    if let Some(cur) = index["current_message"].as_str() {
        if !cur.is_empty() {
            return cur.to_string();
        }
    }

    // fallback → last root
    root.last().expect("❌ No messages").to_string()
}

fn resolve_prefix(root_ids: &Vec<String>, prefix: &str) -> String {
    let matches: Vec<&String> =
        root_ids.iter().filter(|id| id.starts_with(prefix)).collect();

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

    let content = match fs::read_to_string(&msg_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let msg: Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return,
    };

    if let Some(children) = msg["children"].as_array() {
        for c in children {
            if let Some(cid) = c.as_str() {
                recursive_delete(cid);
            }
        }
    }

    let _ = fs::remove_file(&msg_path);
}

fn remove_from_parent_or_root(mid: &str) {
    let fur = Path::new(".fur");

    let msg_path = fur.join("messages").join(format!("{}.json", mid));
    let raw = fs::read_to_string(&msg_path).unwrap_or("{}".into());
    let msg: Value = serde_json::from_str(&raw).unwrap_or(json!({}));

    // If message had a parent
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

    // Else: it's root-level in conversation
    let (_index, tid) = resolve_active_conversation();
    let convo_path = fur.join("threads").join(format!("{}.json", tid));
    let mut convo: Value =
        serde_json::from_str(&fs::read_to_string(&convo_path).unwrap()).unwrap();

    if let Some(arr) = convo["messages"].as_array_mut() {
        arr.retain(|v| v.as_str() != Some(mid));
    }

    write_json(&convo_path, &convo);
}

fn update_current_after_delete(mid: &str) {
    let fur = Path::new(".fur");
    let idx_path = fur.join("index.json");
    let mut index: Value =
        serde_json::from_str(&fs::read_to_string(&idx_path).unwrap()).unwrap();

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
    use std::process::Command;
    use std::env;

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
