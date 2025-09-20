use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Thread {
    pub title: String,
    pub tags: Vec<String>,
    pub items: Vec<ScriptItem>,   // not only messages
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub avatar: String,
    pub text: Option<String>,
    pub file: Option<String>,          // markdown only
    pub attachment: Option<String>,    // image or other binary
    pub children: Vec<Message>,
    pub branches: Vec<Vec<Message>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ScriptItem {
    Message(Message),   // jot or branch
    Command(Command),   // timeline, tree, store...
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Command {
    pub name: String,             // "timeline"
    pub args: Vec<String>,        // ["--out", "TIMELINE_1.md", "--since", "35"]
    pub line_number: usize,       // useful for --since N referencing
}
