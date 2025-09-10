use serde::{Serialize, Deserialize};

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

use std::fs;

pub fn parse_frs(path: &str) -> Thread {
    let content = fs::read_to_string(path).expect("❌ Could not read .frs file");

    let mut lines = content.lines().map(str::trim).filter(|l| !l.is_empty());

    let mut thread = Thread::new("".to_string());

    while let Some(line) = lines.next() {
        if line.starts_with("new ") {
            // Extract title inside quotes
            if let Some(start) = line.find('"') {
                if let Some(end) = line.rfind('"') {
                    thread.title = line[start + 1..end].to_string();
                }
            }
        } else if line.starts_with("tags") {
            // Extract tags inside [ ... ]
            if let Some(start) = line.find('[') {
                if let Some(end) = line.find(']') {
                    let tags_str = &line[start + 1..end];
                    thread.tags = tags_str
                        .split(',')
                        .map(|s| s.trim().trim_matches('"').to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                }
            }
        } else if line.starts_with("jot ") {
            // jot avatar "message" OR jot avatar --file "path"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let avatar = parts[1].to_string();

                if parts[2] == "--file" {
                    // file-based jot
                    if let Some(start) = line.find('"') {
                        if let Some(end) = line.rfind('"') {
                            let path = line[start + 1..end].to_string();
                            thread.messages.push(Message {
                                avatar,
                                text: None,
                                file: Some(path),
                                children: vec![],
                            });
                        }
                    }
                } else {
                    // text-based jot
                    if let Some(start) = line.find('"') {
                        if let Some(end) = line.rfind('"') {
                            let text = line[start + 1..end].to_string();
                            thread.messages.push(Message {
                                avatar,
                                text: Some(text),
                                file: None,
                                children: vec![],
                            });
                        }
                    }
                }
            }
        }
    }

    thread
}

