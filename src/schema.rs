use serde_json::{json, Value};
use chrono::Utc;
use uuid::Uuid;

/* 
=== FUR Schema Constructors ===

Centralized builders for index, conversation, and message JSON.
These are used by `new.rs`, `jot.rs`, and any future modules.
Keeps structure consistent and allows easy schema evolution.

Global schema version (increment as the JSON schema evolves)
 */

pub const SCHEMA_VERSION: &str = "0.2";

pub fn make_index_metadata() -> Value {
    json!({
        "threads": [],
        "active_thread": null,
        "current_message": null,
        "created_at": Utc::now().to_rfc3339(),
        "schema_version": SCHEMA_VERSION
    })
}

pub fn make_conversation_metadata(title: &str, id: &str) -> Value {
    json!({
        "id": id,
        "created_at": Utc::now().to_rfc3339(),
        "messages": [],
        "tags": [],
        "title": title,
        "schema_version": SCHEMA_VERSION
    })
}

pub fn make_message_metadata(
    avatar: &str,
    text: Option<String>,
    markdown: Option<String>,
    img: Option<String>,
    parent: Option<String>,
) -> Value {
    let id = Uuid::new_v4().to_string();
    let timestamp = Utc::now().to_rfc3339();

    json!({
        "id": id,
        "avatar": avatar,
        "timestamp": timestamp,
        "text": text,
        "markdown": markdown,
        "attachment": img,
        "parent": parent,
        "children": [],
        "branches": [],
        "schema_version": SCHEMA_VERSION
    })
}
