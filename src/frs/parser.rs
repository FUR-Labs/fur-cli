use serde_json::json;
use std::fs;
use crate::frs::ast::{Thread, Message};
use crate::frs::avatars::{load_avatars, save_avatars, collect_avatars, get_random_emoji};

/// Pure parser: read .frs into a Thread struct (no side effects)
pub fn parse_frs(path: &str) -> Thread {
    let raw = fs::read_to_string(path).expect("❌ Could not read .frs file");
    let lines: Vec<String> = raw
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    let mut i = 0usize;

    // ---- header: new "Title"
    let title = loop {
        if i >= lines.len() { panic!("❌ Missing `new \"Title\"` at top of file"); }
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

    // ---- optional user (main avatar)
    let default_user: String;
    if i < lines.len() && lines[i].starts_with("user") {
        let parts: Vec<&str> = lines[i].split('=').map(|s| s.trim()).collect();
        if parts.len() == 2 {
            default_user = parts[1].to_string();
        } else {
            panic!("❌ Could not parse `user = <name>` line");
        }
        i += 1;
    } else {
        // fallback to avatars.json["main"]
        let avatars = load_avatars();
        if let Some(main) = avatars.get("main").and_then(|v| v.as_str()) {
            default_user = main.to_string();
        } else {
            panic!("❌ Please define main avatar by typing `user = <name>`");
        }
    }


    // ---- top-level messages (root stem)
    thread.messages = parse_block(&lines, &mut i, false, Some(default_user));
    thread
}

/// Import .frs: parse + update avatars.json
pub fn import_frs(path: &str) -> Thread {
    let thread = parse_frs(path);

    let mut avatars = load_avatars();
    let mut to_register: Vec<String> = Vec::new();
    collect_avatars(&thread.messages, &mut to_register);

    for name in to_register {
        if !avatars.as_object().unwrap().contains_key(&name) {
            let emoji = get_random_emoji();
            avatars[&name] = json!(emoji);
            println!("✨ New avatar detected: \"{}\" → {}", name, emoji);
        }
    }

    save_avatars(&avatars);
    thread
}

// ------------------ Helpers ------------------

fn parse_block(
    lines: &[String],
    i: &mut usize,
    stop_at_closing_brace: bool,
    default_user: Option<String>,
) -> Vec<Message> {
    let mut msgs: Vec<Message> = Vec::new();

    while *i < lines.len() {
        let line = &lines[*i];

        if stop_at_closing_brace && line.starts_with('}') {
            *i += 1;
            break;
        }

        if line.starts_with("jot") {
            if let Some(msg) = parse_jot_line(line, default_user.as_deref().unwrap_or("???")) {
                msgs.push(msg);
            }
            *i += 1;
            continue;
        }

        if is_branch_open(line) {
            *i += 1; // consume "branch {"
            if msgs.is_empty() {
                eprintln!("❌ branch with no preceding jot at line {}", i);
                let _ = parse_block(lines, i, true, default_user.clone());
                continue;
            }
            let children_block = parse_block(lines, i, true, default_user.clone()); // one branch block
            if let Some(last) = msgs.last_mut() {
                last.branches.push(children_block.clone());
                last.children.extend(children_block);
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

/// Parse a jot line: either "jot avatar text" or "jot text" (default user)
fn parse_jot_line(line: &str, default_avatar: &str) -> Option<Message> {
    let mut parts = line.split_whitespace();
    let first = parts.next()?;
    if first != "jot" {
        return None;
    }

    let second = parts.next().unwrap_or("");

    if second == "--file" || second.starts_with('"') {
        // Case: jot "text..."   OR   jot --file path
        if second == "--file" {
            let path = parts.next()?.to_string();
            return Some(Message {
                avatar: default_avatar.to_string(),
                text: None,
                file: Some(path),
                children: vec![],
                branches: vec![],
            });
        } else {
            let text = extract_quoted(line)?;
            return Some(Message {
                avatar: default_avatar.to_string(),
                text: Some(text),
                file: None,
                children: vec![],
                branches: vec![],
            });
        }
    }

    // Case: jot avatar ...
    let avatar = second.to_string();
    if line.contains("--file") {
        let path = extract_quoted(line).unwrap_or_else(|| {
            line.split_whitespace().last().unwrap_or("").to_string()
        });
        return Some(Message {
            avatar,
            text: None,
            file: Some(path),
            children: vec![],
            branches: vec![],
        });
    }

    let text = extract_quoted(line)?;
    Some(Message {
        avatar,
        text: Some(text),
        file: None,
        children: vec![],
        branches: vec![],
    })
}

fn extract_quoted(line: &str) -> Option<String> {
    let start = line.find('"')?;
    let end = line[start + 1..].find('"')? + start + 1;
    Some(line[start + 1..end].to_string())
}
