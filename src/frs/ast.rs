use serde::{Deserialize, Serialize};

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
