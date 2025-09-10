use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Thread {
    pub title: String,
    pub tags: Vec<String>,
    pub messages: Vec<Message>, // top-level jots (root stem)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub avatar: String,
    pub text: Option<String>,
    pub file: Option<String>,
    pub children: Vec<Message>,
    pub branches: Vec<Vec<Message>>,
}

impl Thread {
    pub fn new(title: String) -> Self {
        Thread { title, tags: vec![], messages: vec![] }
    }
}
