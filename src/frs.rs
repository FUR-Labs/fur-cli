use serde::{Deserialize, Serialize};
use std::fs;

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
    thread.messages = parse_block(&lines, &mut i, /*stop_at_closing_brace=*/ false);
    thread
}

/// Parse a block of statements into a Vec<Message>.
/// If `stop_at_closing_brace` is true, stops when encountering a `}` and consumes it.
fn parse_block(lines: &[String], i: &mut usize, stop_at_closing_brace: bool) -> Vec<Message> {
    let mut msgs: Vec<Message> = Vec::new();

    while *i < lines.len() {
        let line = &lines[*i];

        // Stop at `}` for nested branches
        if stop_at_closing_brace && line.starts_with('}') {
            *i += 1; // consume `}`
            break;
        }

        if line.starts_with("jot ") {
            if let Some(msg) = parse_jot_line(line) {
                msgs.push(msg);
            } else {
                eprintln!("⚠️  Could not parse jot line: {}", line);
            }
            *i += 1;
            continue;
        }

        if is_branch_open(line) {
            // consume the `branch {` line
            *i += 1;

            // We must have at least one message in this level to attach children to
            if msgs.is_empty() {
                eprintln!(
                    "❌ `branch {{` found with no preceding `jot` to attach to at line {}",
                    i
                );
                // still parse & skip nested content to maintain cursor
                let _ = parse_block(lines, i, true);
                continue;
            }

            // Parse nested block contents until `}`
            let children = parse_block(lines, i, true);

            // Attach to the last message at this level
            if let Some(last) = msgs.last_mut() {
                last.children.extend(children);
            }
            continue;
        }

        // Unknown line at this level
        if line.starts_with('}') {
            // stray `}` at top-level → warn & consume
            eprintln!("⚠️  Stray closing brace at line {}", i);
            *i += 1;
            continue;
        }

        // Non-empty, non-recognized line
        eprintln!("⚠️  Unrecognized line: {}", line);
        *i += 1;
    }

    msgs
}

fn is_branch_open(line: &str) -> bool {
    // Accepts "branch {" with optional trailing spaces
    line == "branch {" || line.starts_with("branch {")
}

fn parse_tags_line(line: &str) -> Option<Vec<String>> {
    // Expect something like: tags = ["a", "b"]
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
    // "jot <avatar> \"text\"" OR "jot <avatar> --file \"path\""
    let mut parts = line.split_whitespace();
    let first = parts.next()?;
    if first != "jot" {
        return None;
    }
    let avatar = parts.next()?.to_string();

    let is_file = line.contains("--file");
    if is_file {
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
