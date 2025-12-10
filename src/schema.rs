use serde_json::{json, Value};
use chrono::Utc;
use uuid::Uuid;

/*
===============================================================================
 SCHEMA MODULE — v0.3 (GLOBAL-COLUMN AWARE)
===============================================================================
*/

pub const SCHEMA_VERSION: &str = "0.3";

/// Build index.json
pub fn make_index_metadata() -> Value {
    json!({
        "threads": [],
        "active_thread": null,
        "current_message": null,
        "created_at": Utc::now().to_rfc3339(),
        "schema_version": SCHEMA_VERSION,

        // GLOBAL COLUMN SYSTEM
        "global_columns": {},
        "global_column_order": [],

        "conversation_sort": {
            "column": "created",
            "direction": "desc"
        }
    })
}

/// Build conversation metadata
pub fn make_conversation_metadata(title: &str, id: &str) -> Value {
    json!({
        "id": id,
        "created_at": Utc::now().to_rfc3339(),
        "messages": [],
        "tags": [],
        "title": title,
        "schema_version": SCHEMA_VERSION,
        "meta": {
            "columns": { "tags": [] },
            "display_names": { "tags": "Tags" }
        }
    })
}

/// Build message metadata
pub fn make_message_metadata(
    avatar: &str,
    text: Option<String>,
    markdown: Option<String>,
    img: Option<String>,
    parent: Option<String>,
) -> Value {
    json!({
        "id": Uuid::new_v4().to_string(),
        "avatar": avatar,
        "timestamp": Utc::now().to_rfc3339(),
        "text": text,
        "markdown": markdown,
        "attachment": img,
        "parent": parent,
        "children": [],
        "branches": [],
        "schema_version": SCHEMA_VERSION
    })
}

/* =============================================================================
   UPGRADE SYSTEM
============================================================================= */

pub fn upgrade_conversation_schema(mut convo: Value, index: &Value) -> Value {
    let current = convo
        .get("schema_version")
        .and_then(|v| v.as_str())
        .unwrap_or("0.1");

    if current != SCHEMA_VERSION {
        convo = migrate_legacy(convo);
        convo["schema_version"] = json!(SCHEMA_VERSION);
    }

    ensure_global_columns(&mut convo, index);

    convo
}

/// Migrate legacy structures → minimal meta v0.3
fn migrate_legacy(mut convo: Value) -> Value {
    if convo.get("meta").is_none() {
        convo["meta"] = json!({});
    }
    if convo["meta"].get("columns").is_none() {
        convo["meta"]["columns"] = json!({});
    }
    if convo["meta"].get("display_names").is_none() {
        convo["meta"]["display_names"] = json!({});
    }

    // Sync legacy tags into new column system
    let legacy_tags: Vec<String> = convo["tags"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    // Insert tags
    {
        let meta = convo["meta"].as_object_mut().unwrap();
        let cols = meta["columns"].as_object_mut().unwrap();
        cols.insert("tags".into(), json!(legacy_tags));
    }

    {
        let meta = convo["meta"].as_object_mut().unwrap();
        let disp = meta["display_names"].as_object_mut().unwrap();
        disp.insert("tags".into(), json!("Tags"));
    }

    convo
}

/// Ensure convo has all global columns, without double borrowing
fn ensure_global_columns(convo: &mut Value, index: &Value) {

    let global_cols = match index["global_columns"].as_object() {
        Some(map) => map.clone(),
        None => serde_json::Map::new(),
    };

    // FIRST PASS: ensure columns
    {
        let meta = convo["meta"].as_object_mut().unwrap();
        if !meta.contains_key("columns") {
            meta.insert("columns".into(), json!({}));
        }

        let cols = meta["columns"].as_object_mut().unwrap();

        for (internal, _cfg) in &global_cols {
            if !cols.contains_key(internal) {
                cols.insert(internal.clone(), json!([]));
            }
        }
    }

    // SECOND PASS: ensure display names
    {
        let meta = convo["meta"].as_object_mut().unwrap();
        if !meta.contains_key("display_names") {
            meta.insert("display_names".into(), json!({}));
        }

        let disp = meta["display_names"].as_object_mut().unwrap();

        for (internal, cfg) in &global_cols {
            let display = cfg["display"].as_str().unwrap_or("Column").to_string();
            disp.insert(internal.clone(), json!(display));
        }
    }
}
