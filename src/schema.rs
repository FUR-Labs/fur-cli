use serde_json::{json, Value};
use chrono::Utc;
use uuid::Uuid;

/*
===============================================================
 FUR SCHEMA MODULE — v0.3
---------------------------------------------------------------
This version introduces:

 - meta.columns: arbitrary column namespace (list-style values)
 - meta.column_order: for dynamic table rendering
 - meta.sort: user-persistent sorting preferences

This module also includes helpers for upgrading legacy schemas
(v0.1 and v0.2) to the new v0.3 layout without breaking anything.

Old conversations had:
    "tags": []

New structure also has:
    "meta": { "columns": { "tags": [...] } }

The original "tags" top-level field is preserved for backward
compatibility, but ALL new code should read/change tags via
meta.columns.tags.

===============================================================
*/

pub const SCHEMA_VERSION: &str = "0.3";


/// Build a new index.json for a fresh FUR project
pub fn make_index_metadata() -> Value {
    json!({
        "threads": [],
        "active_thread": null,
        "current_message": null,
        "created_at": Utc::now().to_rfc3339(),
        "schema_version": SCHEMA_VERSION,
        "conversation_sort": {
            "column": "created",   // default persistent sort
            "direction": "desc"
        }
    })
}

/// Build a brand-new conversation metadata block
pub fn make_conversation_metadata(title: &str, id: &str) -> Value {
    json!({
        "id": id,
        "created_at": Utc::now().to_rfc3339(),
        "messages": [],
        "tags": [],                          // legacy field retained
        "title": title,
        "schema_version": SCHEMA_VERSION,

        // v0.3 additions
        "meta": {
            // Columns behave like tags: Vec<String>
            "columns": {
                "tags": []                  // mapped from top-level tags upon upgrade
            },

            // Table ordering (Title Case is rendered in the table module)
            "column_order": [
                "id",
                "title",
                "created",
                "message_count",
                "size",
                "tags"
            ],

            // Persistent conversation list sorting
            "sort": {
                "column": "created",
                "direction": "desc"
            }
        }
    })
}

/// Build new message metadata, unchanged except schema bump
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


/* =====================================================================
   AUTO-MIGRATION UTILITIES (v0.1/v0.2 → v0.3)
   ===================================================================== */

/// Upgrade a conversation Value to v0.3 in-place.
/// Always call this immediately after loading conversation JSON.
pub fn upgrade_conversation_schema(mut convo: Value) -> Value {
    let current_version = convo.get("schema_version")
        .and_then(|v| v.as_str())
        .unwrap_or("0.1");

    if current_version == SCHEMA_VERSION {
        return ensure_minimal_meta(convo);
    }

    let legacy_tags: Vec<String> = convo.get("tags")
        .and_then(|v| v.as_array())
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    if convo.get("meta").is_none() {
        convo["meta"] = json!({});
    }

    if convo["meta"].get("columns").is_none() {
        convo["meta"]["columns"] = json!({});
    }

    if convo["meta"].get("display_names").is_none() {
        convo["meta"]["display_names"] = json!({});
    }

    if convo["meta"]["columns"].get("tags").is_none() {
        convo["meta"]["columns"]["tags"] = json!(legacy_tags);
    }

    if convo["meta"].get("column_order").is_none() {
        convo["meta"]["column_order"] = json!([
            "id",
            "title",
            "created",
            "message_count",
            "size",
            "tags"
        ]);
    }

    if convo["meta"].get("sort").is_none() {
        convo["meta"]["sort"] = json!({
            "column": "created",
            "direction": "desc"
        });
    }

    convo["schema_version"] = json!(SCHEMA_VERSION);

    ensure_minimal_meta(convo)
}


/// Ensures meta fields are never missing even if user modifies files manually
fn ensure_minimal_meta(mut convo: Value) -> Value {
    if convo.get("meta").is_none() {
        convo["meta"] = json!({});
    }

    if convo["meta"].get("columns").is_none() {
        convo["meta"]["columns"] = json!({});
    }

    // Backfill tags column if missing
    if convo["meta"]["columns"].get("tags").is_none() {
        let legacy_tags: Vec<String> = convo.get("tags")
            .and_then(|v| v.as_array())
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        convo["meta"]["columns"]["tags"] = json!(legacy_tags);
    }

    // Backfill basic column ordering
    if convo["meta"].get("column_order").is_none() {
        convo["meta"]["column_order"] = json!([
            "id",
            "title",
            "created",
            "message_count",
            "size",
            "tags"
        ]);
    }

    // Backfill sort block
    if convo["meta"].get("sort").is_none() {
        convo["meta"]["sort"] = json!({
            "column": "created",
            "direction": "desc"
        });
    }

    convo
}

