use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use rand::prelude::IndexedRandom;

#[derive(Debug, Serialize, Deserialize)]
pub struct Thread {
    pub title: String,
    pub tags: Vec<String>,
    pub messages: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub avatar: String,
    pub text: Option<String>,
    pub file: Option<String>,
    pub children: Vec<Message>,
}

impl Thread {
    pub fn new(title: String) -> Self {
        Thread {
            title,
            tags: vec![],
            messages: vec![],
        }
    }
}

// ------------------
// Public entrypoints
// ------------------

/// Pure parser: read .frs into a Thread struct (no side effects)
pub fn parse_frs(path: &str) -> Thread {
    let raw = fs::read_to_string(path).expect("❌ Could not read .frs file");
    let mut lines: Vec<String> = raw
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    let mut i = 0usize;

    // ---- header: new "Title"
    let title = loop {
        if i >= lines.len() {
            panic!("❌ Missing `new \"Title\"` at top of file");
        }
        let line = &lines[i];
        if line.starts_with("new ") {
            break extract_quoted(line).unwrap_or_else(|| {
                panic!("❌ Could not parse thread title from: {}", line)
            });
        }
        i += 1;
    };
    let mut thread = Thread::new(title);
    i += 1;

    // ---- optional tags
    if i < lines.len() && lines[i].starts_with("tags") {
        if let Some(tags) = parse_tags_line(&lines[i]) {
            thread.tags = tags;
        }
        i += 1;
    }

    // ---- top-level messages/branches
    thread.messages = parse_block(&lines, &mut i, false);
    thread
}

/// Import .frs: parse + ensure avatars.json is updated
pub fn import_frs(path: &str) -> Thread {
    let thread = parse_frs(path);

    let mut avatars = load_avatars();

    // collect all avatars in thread
    let mut to_register: Vec<String> = Vec::new();
    collect_avatars(&thread.messages, &mut to_register);

    for name in to_register {
        if !avatars.get(&name).is_some() {
            let emoji = get_random_emoji();
            avatars[&name] = json!(emoji);
            println!("✨ New avatar detected: \"{}\" → {}", name, emoji);
        }
    }

    save_avatars(&avatars);
    thread
}


use uuid::Uuid;
use chrono::Utc;

/// Persist a parsed Thread into .fur/threads + .fur/messages
pub fn persist_frs(thread: &Thread) -> String {
    let fur_dir = Path::new(".fur");
    if !fur_dir.exists() {
        panic!("🚨 .fur directory not initialized. Run `fur new` at least once.");
    }

    // 1. Create new thread ID
    let thread_id = Uuid::new_v4().to_string();
    let timestamp = Utc::now().to_rfc3339();

    // 2. Collect messages and write them
    let mut message_ids: Vec<String> = Vec::new();
    persist_messages(&thread.messages, None, &mut message_ids);

    // 3. Write thread JSON
    let thread_json = json!({
        "id": thread_id,
        "created_at": timestamp,
        "title": thread.title,
        "tags": thread.tags,
        "messages": message_ids,
    });

    let thread_path = fur_dir.join("threads").join(format!("{}.json", thread_id));
    fs::write(&thread_path, serde_json::to_string_pretty(&thread_json).unwrap())
        .expect("❌ Could not write thread file");

    // 4. Update index.json
    let index_path = fur_dir.join("index.json");
    let mut index_data: Value =
        serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();

    index_data["threads"]
        .as_array_mut()
        .unwrap()
        .push(thread_id.clone().into());
    index_data["active_thread"] = thread_id.clone().into();
    index_data["current_message"] = Value::Null;

    fs::write(
        &index_path,
        serde_json::to_string_pretty(&index_data).unwrap(),
    )
    .unwrap();

    println!("🌱 Imported thread into .fur: {} — \"{}\"", &thread_id[..8], thread.title);

    thread_id
}

fn persist_messages(msgs: &[Message], parent: Option<String>, all_ids: &mut Vec<String>) {
    for m in msgs {
        let msg_id = Uuid::new_v4().to_string();

        // Children are persisted recursively
        let mut child_ids: Vec<String> = Vec::new();
        persist_messages(&m.children, Some(msg_id.clone()), &mut child_ids);

        let msg_json = json!({
            "id": msg_id,
            "avatar": m.avatar,
            "name": m.avatar, // store avatar name, emoji already in avatars.json
            "text": m.text,
            "markdown": m.file,
            "parent": parent,
            "children": child_ids,
            "timestamp": Utc::now().to_rfc3339(),
        });

        let path = Path::new(".fur/messages").join(format!("{}.json", msg_id));
        fs::write(&path, serde_json::to_string_pretty(&msg_json).unwrap())
            .expect("❌ Could not write message file");

        all_ids.push(msg_id);
    }
}


// ------------------
// Helpers
// ------------------

fn parse_block(lines: &[String], i: &mut usize, stop_at_closing_brace: bool) -> Vec<Message> {
    let mut msgs: Vec<Message> = Vec::new();

    while *i < lines.len() {
        let line = &lines[*i];

        if stop_at_closing_brace && line.starts_with('}') {
            *i += 1;
            break;
        }

        if line.starts_with("jot ") {
            if let Some(msg) = parse_jot_line(line) {
                msgs.push(msg);
            }
            *i += 1;
            continue;
        }

        if is_branch_open(line) {
            *i += 1; // consume
            if msgs.is_empty() {
                eprintln!("❌ branch with no preceding jot at line {}", i);
                let _ = parse_block(lines, i, true);
                continue;
            }
            let children = parse_block(lines, i, true);
            if let Some(last) = msgs.last_mut() {
                last.children.extend(children);
            }
            continue;
        }

        if line.starts_with('}') {
            *i += 1;
            continue;
        }

        eprintln!("⚠️ Unrecognized line: {}", line);
        *i += 1;
    }

    msgs
}

fn is_branch_open(line: &str) -> bool {
    line == "branch {" || line.starts_with("branch {")
}

fn parse_tags_line(line: &str) -> Option<Vec<String>> {
    let start = line.find('[')?;
    let end = line.rfind(']')?;
    let inner = &line[start + 1..end];
    let tags = inner
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    Some(tags)
}

fn parse_jot_line(line: &str) -> Option<Message> {
    let mut parts = line.split_whitespace();
    let first = parts.next()?;
    if first != "jot" {
        return None;
    }
    let avatar = parts.next()?.to_string();

    if line.contains("--file") {
        let path = extract_quoted(line)?;
        return Some(Message {
            avatar,
            text: None,
            file: Some(path),
            children: vec![],
        });
    }

    let text = extract_quoted(line)?;
    Some(Message {
        avatar,
        text: Some(text),
        file: None,
        children: vec![],
    })
}

fn extract_quoted(line: &str) -> Option<String> {
    let start = line.find('"')?;
    let end = line[start + 1..].find('"')? + start + 1;
    Some(line[start + 1..end].to_string())
}

// ------------------
// Avatar utilities
// ------------------

fn load_avatars() -> Value {
    let path = Path::new(".fur/avatars.json");
    if path.exists() {
        let content = fs::read_to_string(path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_else(|_| json!({}))
    } else {
        json!({})
    }
}

fn save_avatars(avatars: &Value) {
    let path = Path::new(".fur/avatars.json");
    if let Ok(serialized) = serde_json::to_string_pretty(avatars) {
        let _ = fs::write(path, serialized);
    }
}

fn collect_avatars(msgs: &[Message], acc: &mut Vec<String>) {
    for m in msgs {
        if !acc.contains(&m.avatar) {
            acc.push(m.avatar.clone());
        }
        collect_avatars(&m.children, acc);
    }
}

fn get_random_emoji() -> String {
    let emojis = ["👹", "🐵", "🐧", "🐺", "🦁"];
    let mut rng = rand::rng();
    emojis.choose(&mut rng).unwrap_or(&"🐾").to_string()
}
