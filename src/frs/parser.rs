use serde_json::{json};
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
            break extract_quoted(line).unwrap_or_else(|| panic!("❌ Could not parse thread title from: {}", line));
        }
        i += 1;
    };
    let mut thread = Thread::new(title);
    i += 1;

    // ---- optional tags
    if i < lines.len() && lines[i].starts_with("tags") {
        if let Some(tags) = parse_tags_line(&lines[i]) { thread.tags = tags; }
        i += 1;
    }

    // ---- top-level messages (root stem)
    thread.messages = parse_block(&lines, &mut i, false);
    thread
}

/// Import .frs: parse + ensure avatars.json is updated
pub fn import_frs(path: &str) -> Thread {
    let thread = parse_frs(path);

    let mut avatars = load_avatars();
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

// ------------------ Helpers ------------------

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
            *i += 1; // consume "branch {"
            if msgs.is_empty() {
                eprintln!("❌ branch with no preceding jot at line {}", i);
                let _ = parse_block(lines, i, true);
                continue;
            }
            let children_block = parse_block(lines, i, true); // one branch block
            if let Some(last) = msgs.last_mut() {
                // Save as a grouped branch
                last.branches.push(children_block.clone());
                // Also flatten into children for compatibility
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

fn parse_jot_line(line: &str) -> Option<Message> {
    let mut parts = line.split_whitespace();
    let first = parts.next()?;
    if first != "jot" { return None; }
    let avatar = parts.next()?.to_string();

    if line.contains("--file") {
        let path = extract_quoted(line)?;
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
