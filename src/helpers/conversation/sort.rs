use std::fs;
use std::cmp::Ordering;
use std::path::Path;

use serde_json::{Value};
use chrono::{DateTime, Utc};

use crate::helpers::tags::normalize_tag;

/* =============================================================
   SORTING CONTROL MODULE
   Provides:

   - Persistent sort preference writing (index.json)
   - A unified row-comparison function for dynamic tables
   - Column normalization (snake_case)
   - Value comparison for dynamic metadata columns

   Should be used by:
   - conversation.rs (in handle_sorting_ops + view logic)
============================================================= */

pub fn handle_sorting_ops(
    args: &crate::commands::conversation::ThreadArgs,
    index: &mut Value,
    index_path: &Path,
) {
    // Acquire or create "conversation_sort" section
    let obj = index["conversation_sort"].as_object();
    let mut sort_cfg = match obj {
        Some(map) => map.clone(),
        None => serde_json::Map::new(),
    };

    /* ---------------------------------------------------------
       UPDATE COLUMN
    --------------------------------------------------------- */
    if let Some(raw) = &args.sort_by {
        let internal = normalize_tag(raw);
        sort_cfg.insert("column".into(), internal.into());
    }

    /* ---------------------------------------------------------
       UPDATE DIRECTION
    --------------------------------------------------------- */
    if args.asc {
        sort_cfg.insert("direction".into(), "asc".into());
    }
    if args.desc {
        sort_cfg.insert("direction".into(), "desc".into());
    }

    // Write back
    index["conversation_sort"] = Value::Object(sort_cfg);

    fs::write(
        index_path,
        serde_json::to_string_pretty(index).unwrap(),
    ).unwrap();

    println!("🔃 Updated sorting preferences.");
}

/* =============================================================
   RowData STRUCT MIRROR
   (conversation.rs should define this, but we mirror enough
    fields to allow comparison logic)

   NOTE — RowData definitions must match caller's struct exactly.
============================================================= */

#[derive(Debug)]
pub struct RowData {
    pub tid: String,
    pub title: String,
    pub created_utc: DateTime<Utc>,
    pub msg_count: usize,
    pub size_bytes: u64,
    pub dynamic: Vec<(String, String, String)>, 
    // (internal_key, display_name, value_string)
}

/* =============================================================
   MAIN COMPARISON ENTRYPOINT
============================================================= */

pub fn compare_rows(a: &RowData, b: &RowData, col: &str) -> Ordering {
    match col {
        /* -----------------------------
           STATIC COLUMNS
        ----------------------------- */
        "id" => a.tid.cmp(&b.tid),

        "title" => a.title.to_lowercase().cmp(&b.title.to_lowercase()),

        "created" => a.created_utc.cmp(&b.created_utc),

        "message_count" | "msg_count" => a.msg_count.cmp(&b.msg_count),

        "size" | "size_bytes" => a.size_bytes.cmp(&b.size_bytes),

        /* -----------------------------
           DYNAMIC COLUMNS
        ----------------------------- */
        other => compare_dynamic(a, b, other),
    }
}

/* =============================================================
   DYNAMIC COLUMN COMPARISON
============================================================= */

fn compare_dynamic(a: &RowData, b: &RowData, col: &str) -> Ordering {
    let left = lookup_dynamic_value(a, col);
    let right = lookup_dynamic_value(b, col);

    left.to_lowercase().cmp(&right.to_lowercase())
}

/* Internal helper to find column value for a row */
fn lookup_dynamic_value(row: &RowData, col: &str) -> String {
    for (internal, _, value) in &row.dynamic {
        if internal == col {
            return value.clone();
        }
    }

    String::new()
}


/* =============================================================
   DEFAULTS
============================================================= */

pub fn default_sort_column() -> &'static str {
    "created"
}

pub fn default_sort_direction() -> &'static str {
    "desc"
}
