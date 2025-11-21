use std::fs;
use std::path::{Path, PathBuf};
use crate::frs::ast::{Thread, Message, ScriptItem, Command};
use crate::frs::avatars::load_avatars;

/// Pure parser: read .frs into a Thread struct
pub fn parse_frs(path: &str) -> Thread {
    let raw = fs::read_to_string(path).expect("❌ Could not read .frs file");

    let frs_path = Path::new(path);
    let frs_dir = frs_path.parent().unwrap_or_else(|| Path::new("."));

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
                panic!("❌ Could not parse conversation title from: {}", line)
            });
        }
        i += 1;
    };
    let mut conversation = Thread {
        title,
        tags: vec![],
        items: vec![],
    };
    i += 1;

    // ---- header meta (user, tags...)
    let mut default_user: Option<String> = None;

    while i < lines.len() {
        let line = &lines[i];

        // stop when content starts
        if line.starts_with("jot") || line.starts_with("branch") {
            break;
        }

        if line.starts_with("user") {
            if let Some(eq_pos) = line.find('=') {
                let val = line[eq_pos + 1..].trim();
                if val.is_empty() {
                    panic!("❌ Could not parse `user = <name>` line");
                }
                default_user = Some(val.to_string());
            } else {
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
                conversation.tags = tags;
            }
            i += 1;
            continue;
        }

        break;
    }

    let default_user = if let Some(u) = default_user {
        u
    } else {
        let avatars = load_avatars();
        if let Some(main) = avatars.get("main").and_then(|v| v.as_str()) {
            main.to_string()
        } else {
            panic!("❌ Please define main avatar with `user = <name>` or set one with `fur avatar <name>`.");
        }
    };

    // ---- parse content
    conversation.items = parse_block(&lines, &mut i, false, &default_user, frs_dir);
    conversation
}

// ------------------ Helpers ------------------

fn parse_block(
    lines: &[String],
    i: &mut usize,
    stop_at_closing_brace: bool,
    default_user: &str,
    frs_dir: &Path,
) -> Vec<ScriptItem> {
    let mut items: Vec<ScriptItem> = Vec::new();

    while *i < lines.len() {
        let line = &lines[*i];

        if stop_at_closing_brace && line.starts_with('}') {
            *i += 1;
            break;
        }

        if line.starts_with("jot") {
            if let Some(msg) = parse_jot_line(lines, i, default_user, frs_dir) {
                items.push(ScriptItem::Message(msg));
            }
            continue;
        }

        if is_command_line(line) {
            let cmd = parse_command_line(line, *i + 1);
            items.push(ScriptItem::Command(cmd));
            *i += 1;
            continue;
        }

        if is_branch_open(line) {
            *i += 1;

            if items.is_empty() {
                eprintln!("❌ branch with no preceding jot at line {}", i);
                let _ = parse_block(lines, i, true, default_user, frs_dir);
                continue;
            }

            let children_block = parse_block(lines, i, true, default_user, frs_dir);

            if let Some(ScriptItem::Message(last)) = items.last_mut() {
                let children: Vec<Message> = children_block
                    .into_iter()
                    .filter_map(|si| match si {
                        ScriptItem::Message(m) => Some(m),
                        _ => None,
                    })
                    .collect();

                last.branches.push(children.clone());
                last.children.extend(children);
            }
            continue;
        }

        if line.starts_with('}') {
            *i += 1;
            continue;
        }

        if stop_at_closing_brace {
            break;
        } else {
            eprintln!("⚠️ Unrecognized line: {}", line);
            *i += 1;
        }
    }

    items
}

fn is_branch_open(line: &str) -> bool {
    line == "branch {" || line.starts_with("branch {")
}

fn collect_multiline_quoted(lines: &[String], i: &mut usize) -> Option<String> {
    let mut buf = String::new();
    let mut started = false;

    while *i < lines.len() {
        let line = &lines[*i];

        if !started {
            if let Some(start) = line.find('"') {
                started = true;
                let after = &line[start + 1..];
                if let Some(end) = after.find('"') {
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

fn make_message(
    avatar: &str,
    text: Option<String>,
    file: Option<String>,
    attachment: Option<String>,
) -> Message {
    Message {
        avatar: avatar.to_string(),
        text,
        file,
        attachment,
        children: vec![],
        branches: vec![],
    }
}

fn parse_text_jot(lines: &[String], i: &mut usize, avatar: &str) -> Option<Message> {
    collect_multiline_quoted(lines, i)
        .map(|text| make_message(avatar, Some(text), None, None))
}

fn parse_file_jot(line: &str, avatar: &str, frs_dir: &Path) -> Option<Message> {
    let raw_path = extract_quoted(line)
        .or_else(|| line.split_whitespace().last().map(|s| s.to_string()))
        .unwrap_or_default();

    // Canonicalize relative to the .frs file directory
    let abs_path: PathBuf = frs_dir.join(&raw_path);

    Some(make_message(
        avatar,
        None,
        Some(abs_path.to_string_lossy().to_string()),
        None,
    ))
}

fn parse_attach_jot(line: &str, avatar: &str) -> Option<Message> {
    let path = extract_quoted(line)
        .or_else(|| line.split_whitespace().last().map(|s| s.to_string()))
        .unwrap_or_default();

    Some(make_message(avatar, None, None, Some(path)))
}

fn parse_jot_line(
    lines: &[String],
    i: &mut usize,
    default_avatar: &str,
    frs_dir: &Path,
) -> Option<Message> {
    let line = &lines[*i];
    let mut parts = line.split_whitespace();

    if parts.next()? != "jot" {
        return None;
    }

    let second = parts.next().unwrap_or("");

    // Case A: default avatar
    if second == "--file" {
        let msg = parse_file_jot(line, default_avatar, frs_dir);
        *i += 1;
        return msg;
    }

    if second == "--attach" {
        let msg = parse_attach_jot(line, default_avatar);
        *i += 1;
        return msg;
    }

    if second.starts_with('"') {
        let msg = parse_text_jot(lines, i, default_avatar);
        return msg;
    }

    // Case B: explicit avatar
    let avatar = second.to_string();

    if line.contains("--file") {
        let msg = parse_file_jot(line, &avatar, frs_dir);
        *i += 1;
        return msg;
    }

    if line.contains("--attach") {
        let msg = parse_attach_jot(line, &avatar);
        *i += 1;
        return msg;
    }

    let msg = parse_text_jot(lines, i, &avatar);
    msg
}

fn extract_quoted(line: &str) -> Option<String> {
    let start = line.find('"')?;
    let end = line[start + 1..].find('"')? + start +1;
    Some(line[start + 1..end].to_string())
}

fn is_command_line(line: &str) -> bool {
    line.starts_with("timeline")
        || line.starts_with("tree")
        || line.starts_with("status")
        || line.starts_with("store")
        || line.starts_with("printed")
}

fn parse_command_line(line: &str, line_number: usize) -> Command {
    let mut parts = line.split_whitespace();
    let name = parts.next().unwrap_or("").to_string();
    let args = parts.map(|s| s.to_string()).collect();
    Command { name, args, line_number }
}
