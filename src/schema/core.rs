use chrono::Utc;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use uuid::Uuid;

pub const CURRENT_SCHEMA: &str = "0.3";

/// Upgrade a message to the current schema if needed.
/// Returns true if the message was modified.
pub fn upgrade_message_schema(msg: &mut Value) -> bool {
    let mut changed = false;

    let schema = msg["schema_version"].as_str().unwrap_or("0.1");

    if schema != CURRENT_SCHEMA {
        msg["schema_version"] = json!(CURRENT_SCHEMA);
        changed = true;
    }

    if let Some(md_path) = msg["markdown"].as_str() {
        if msg.get("markdown_meta").is_none() {
            let md = Path::new(md_path);

            if md.exists() {
                if let Ok(bytes) = fs::read(md) {
                    let mut hasher = Sha256::new();
                    hasher.update(&bytes);

                    let hash = format!("{:x}", hasher.finalize());

                    let size = bytes.len();

                    let filename = md.file_name().unwrap().to_string_lossy().to_string();

                    msg["markdown_meta"] = json!({
                        "hash": hash,
                        "size": size,
                        "filename": filename
                    });

                    changed = true;
                }
            }
        }
    }

    changed
}

/*
=== FUR Schema Constructors ===
Centralized builders for index, conversation, and message JSON.

Schema evolution supported via schema_version field.
*/

pub const SCHEMA_VERSION: &str = "0.3";

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
        "parents": [],
        "children": [],
        "title": title,
        "schema_version": SCHEMA_VERSION
    })
}

fn compute_file_hash(path: &Path) -> Option<String> {
    if !path.exists() {
        return None;
    }

    let bytes = fs::read(path).ok()?;

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let result = hasher.finalize();

    Some(format!("{:x}", result))
}

fn compute_file_size(path: &Path) -> Option<u64> {
    fs::metadata(path).map(|m| m.len()).ok()
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

    let markdown_meta = markdown.as_ref().and_then(|p| {
        let path = Path::new(p);

        let hash = compute_file_hash(path);
        let size = compute_file_size(path);
        let filename = path
            .file_name()
            .and_then(|f| f.to_str())
            .map(|s| s.to_string());

        Some(json!({
            "hash": hash,
            "size": size,
            "filename": filename
        }))
    });

    json!({
        "id": id,
        "avatar": avatar,
        "timestamp": timestamp,
        "text": text,
        "markdown": markdown,
        "markdown_meta": markdown_meta,
        "attachment": img,
        "parent": parent,
        "children": [],
        "branches": [],
        "schema_version": SCHEMA_VERSION
    })
}
