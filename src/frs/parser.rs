use serde_json::json;
use std::fs;
use crate::frs::ast::{Thread, Message};
use crate::frs::avatars::{load_avatars, save_avatars, save_avatars_with_main, collect_avatars, get_random_emoji};

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

    // ---- header meta (any order): user, tags ...
    // We keep scanning header lines until the first content line ("jot"/"branch") appears.
    let mut default_user: Option<String> = None;

    while i < lines.len() {
        let line = &lines[i];

        // stop when content starts
        if line.starts_with("jot") || line.starts_with("branch") {
            break;
        }

        if line.starts_with("user") {
            // Accept both: `user = name` and `user name`
            if let Some(eq_pos) = line.find('=') {
                // user = andrew
                let val = line[eq_pos + 1..].trim();
                if val.is_empty() {
                    panic!("❌ Could not parse `user = <name>` line");
                }
                default_user = Some(val.to_string());
            } else {
                // user andrew
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() == 2 {
                    default_user = Some(parts[1].to_string());
                } else {
                    panic!("❌ Could not parse `user <name>` line");
                }
            }
            i += 1;
            continue;
        }

        if line.starts_with("tags") {
            if let Some(tags) = parse_tags_line(line) {
                thread.tags = tags;
            }
            i += 1;
            continue;
        }

        // Unknown header directive — stop treating as header block
        break;
    }

    // Fallback to avatars.json main if user not defined
    let default_user = if let Some(u) = default_user {
        u
    } else {
        let avatars = load_avatars();
        if let Some(main) = avatars.get("main").and_then(|v| v.as_str()) {
            main.to_string()
        } else {
            panic!("❌ Please define main avatar by typing `user = <name>` (or set one with `fur avatar <name>`).");
        }
    };

    // ---- top-level messages (root stem)
    thread.messages = parse_block(&lines, &mut i, false, &default_user);
    thread
}

/// Import .frs: parse + update avatars.json
pub fn import_frs(path: &str) -> Thread {
    let thread = parse_frs(path);

    let mut avatars = load_avatars();
    let mut to_register: Vec<String> = Vec::new();
    collect_avatars(&thread.messages, &mut to_register);

    for name in &to_register {
        if !avatars.as_object().unwrap().contains_key(name) {
            let emoji = if Some(name.as_str()) == avatars.get("main").and_then(|v| v.as_str()) {
                "🦊".to_string() // force fox
            } else {
                get_random_emoji()
            };
            avatars[name] = json!(emoji);
            println!("✨ New avatar detected: \"{}\" → {}", name, emoji);
        }
    }


    // enforce main avatar
    if let Some(main) = avatars.get("main").and_then(|v| v.as_str()) {
        let main_str = main.to_string(); // break borrow
        save_avatars_with_main(&mut avatars, &main_str);
    } else if let Some(first) = to_register.first() {
        save_avatars_with_main(&mut avatars, first);
    } else {
        save_avatars(&avatars);
    }

    thread
}

// ------------------ Helpers ------------------

fn parse_block(
    lines: &[String],
    i: &mut usize,
    stop_at_closing_brace: bool,
    default_user: &str,
) -> Vec<Message> {
    let mut msgs: Vec<Message> = Vec::new();

    while *i < lines.len() {
        let line = &lines[*i];

        if stop_at_closing_brace && line.starts_with('}') {
            *i += 1;
            break;
        }

        if line.starts_with("jot") {
            if let Some(msg) = parse_jot_line(lines, i, default_user) {
                msgs.push(ScriptItem::Message(msg));
            }
            continue;
        }

        if is_command_line(line) {
            let cmd = parse_command_line(line, *i + 1); // +1 because human line numbers
            msgs.push(ScriptItem::Command(cmd));
            *i += 1;
            continue;
        }

        if is_branch_open(line) {
            *i += 1; // consume "branch {"
            if msgs.is_empty() {
                eprintln!("❌ branch with no preceding jot at line {}", i);
                let _ = parse_block(lines, i, true, default_user);
                continue;
            }
            let children_block = parse_block(lines, i, true, default_user); // one branch block
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

        // Unknown/stray line — stop parsing at this level
        if stop_at_closing_brace {
            break;
        } else {
            eprintln!("⚠️ Unrecognized line: {}", line);
            *i += 1;
        }
    }

    msgs
}

fn is_branch_open(line: &str) -> bool {
    line == "branch {" || line.starts_with("branch {")
}

/// Collect multi-line quoted text starting at current line.
/// Advances `i` until the closing `"` is found.
fn collect_multiline_quoted(lines: &[String], i: &mut usize) -> Option<String> {
    let mut buf = String::new();
    let mut started = false;

    while *i < lines.len() {
        let line = &lines[*i];

        if !started {
            // find the first quote
            if let Some(start) = line.find('"') {
                started = true;
                let after = &line[start + 1..];
                if let Some(end) = after.find('"') {
                    // opening and closing quote on same line
                    buf.push_str(&after[..end]);
                    *i += 1;
                    return Some(buf);
                } else {
                    buf.push_str(after);
                }
            }
        } else {
            buf.push('\n');
            if let Some(end) = line.find('"') {
                buf.push_str(&line[..end]);
                *i += 1;
                return Some(buf);
            } else {
                buf.push_str(line);
            }
        }

        *i += 1;
    }

    None
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

/// Parse a jot line: either:
/// - `jot "text"` (uses default user)
/// - `jot --file path` (uses default user)
/// - `jot ai "text"`
/// - `jot ai --file LARGE_THESIS.md` (path may be quoted or bare)
fn parse_jot_line(lines: &[String], i: &mut usize, default_avatar: &str) -> Option<Message> {
    let line = &lines[*i];
    let mut parts = line.split_whitespace();
    let first = parts.next()?;
    if first != "jot" {
        return None;
    }

    let second = parts.next().unwrap_or("");

    // Case A: `jot "text..."`  OR  `jot --file path`
    if second == "--file" || second.starts_with('"') {
        if second == "--file" {
            let path = extract_quoted(line)
                .or_else(|| parts.last().map(|s| s.to_string()))
                .unwrap_or_default();
            *i += 1;
            return Some(Message {
                avatar: default_avatar.to_string(),
                text: None,
                file: Some(path),
                attachment: None, 
                children: vec![],
                branches: vec![],
            });
        } else {
            // multi-line text case
            if let Some(text) = collect_multiline_quoted(lines, i) {
                return Some(Message {
                    avatar: default_avatar.to_string(),
                    text: Some(text),
                    file: None,
                    attachment: None, 
                    children: vec![],
                    branches: vec![],
                });
            } else {
                return None;
            }
        }
    }

    // Case B: `jot ai ...`
    let avatar = second.to_string();
    if line.contains("--file") {
        let path = extract_quoted(line)
            .or_else(|| line.split_whitespace().last().map(|s| s.to_string()))
            .unwrap_or_default();
        *i += 1;
        return Some(Message {
            avatar,
            text: None,
            file: Some(path),
            attachment: None, 
            children: vec![],
            branches: vec![],
        });
    }

    if let Some(text) = collect_multiline_quoted(lines, i) {
        return Some(Message {
            avatar,
            text: Some(text),
            file: None,
            attachment: None, 
            children: vec![],
            branches: vec![],
        });
    } else {
        None
    }
}


fn extract_quoted(line: &str) -> Option<String> {
    let start = line.find('"')?;
    let end = line[start + 1..].find('"')? + start + 1;
    Some(line[start + 1..end].to_string())
}

fn is_command_line(line: &str) -> bool {
    line.starts_with("timeline")
        || line.starts_with("tree")
        || line.starts_with("status")
        || line.starts_with("store")
}

fn parse_command_line(line: &str, line_number: usize) -> Command {
    let mut parts = line.split_whitespace();
    let name = parts.next().unwrap_or("").to_string();
    let args = parts.map(|s| s.to_string()).collect();
    Command { name, args, line_number }
}
