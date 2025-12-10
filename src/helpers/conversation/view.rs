use std::fs;
use std::path::Path;

use chrono::{DateTime, Local, Utc};
use serde_json::Value;

use crate::schema::upgrade_conversation_schema;
use crate::renderer::table::render_table;

use crate::helpers::conversation::sort::{
    RowData,
    compare_rows,
    default_sort_column,
    default_sort_direction,
};


/* =========================================================================
   PUBLIC ENTRYPOINT
   Called from conversation.rs: handle_view_threads()
   Builds the table, applies sorting, and renders final view.
============================================================================ */

pub fn view_conversations(
    index: &Value,
    fur_dir: &Path,
    show_all: bool,
) {
    let active_tid = index["active_thread"].as_str().unwrap_or("");

    let sort_col = index["conversation_sort"]["column"]
        .as_str()
        .unwrap_or(default_sort_column());

    let sort_dir = index["conversation_sort"]["direction"]
        .as_str()
        .unwrap_or(default_sort_direction());

    // Collect all conversation IDs
    let thread_ids: Vec<String> = index["threads"]
        .as_array().unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    let mut rows: Vec<RowData> = Vec::new();
    let mut union_columns: Vec<String> = Vec::new();
    let mut all_display_names = std::collections::HashMap::new();

    /* ---------------------------------------------------------------------
       SCAN & LOAD EACH CONVERSATION
    --------------------------------------------------------------------- */
    for tid in &thread_ids {
        let path = fur_dir.join("threads").join(format!("{}.json", tid));

        if let Ok(raw) = fs::read_to_string(&path) {
            if let Ok(mut convo) = serde_json::from_str::<Value>(&raw) {
                // Always upgrade schema first
                convo = upgrade_conversation_schema(convo);

                // Metadata
                let title = convo["title"]
                    .as_str()
                    .unwrap_or("Untitled")
                    .to_string();

                let created_utc = parse_created(&convo);

                let msg_count = convo["messages"]
                    .as_array()
                    .map(|a| a.len())
                    .unwrap_or(0);

                // Compute size
                let msg_ids = extract_message_ids(&convo);
                let size_bytes = compute_full_conversation_size(fur_dir, tid, &msg_ids);

                // Extract dynamic metadata
                let (dynamic, col_names, disp_names) = extract_dynamic_columns(&convo);

                // Merge union of column names
                for c in col_names {
                    if !union_columns.contains(&c) {
                        union_columns.push(c.clone());
                    }
                }

                // Merge display name mapping
                for (internal, disp) in disp_names {
                    all_display_names.insert(internal, disp);
                }

                rows.push(RowData {
                    tid: tid.clone(),
                    title,
                    created_utc,
                    msg_count,
                    size_bytes,
                    dynamic,
                });
            }
        }
    }

    // Sort columns lexicographically for deterministic table order
    union_columns.sort();

    /* ---------------------------------------------------------------------
       SORT ROWS
    --------------------------------------------------------------------- */
    rows.sort_by(|a, b| {
        let ord = compare_rows(a, b, sort_col);
        if sort_dir == "asc" { ord } else { ord.reverse() }
    });

    /* ---------------------------------------------------------------------
       BUILD STATIC HEADERS
    --------------------------------------------------------------------- */
    let mut headers: Vec<String> = vec![
        "ID".into(),
        "Title".into(),
        "Created".into(),
        "#Msgs".into(),
        "Size".into(),
    ];

    for internal in &union_columns {
        let disp = all_display_names
            .get(internal)
            .cloned()
            .unwrap_or_else(|| to_title_case(internal));
        headers.push(disp);
    }

    /* ---------------------------------------------------------------------
       BUILD ROW STRINGS FOR THE TABLE
    --------------------------------------------------------------------- */
    let mut string_rows = Vec::new();
    let mut active_idx: Option<usize> = None;

    for (i, rd) in rows.iter().enumerate() {
        if rd.tid == active_tid {
            active_idx = Some(i);
        }

        // Convert static fields
        let mut row = vec![
            rd.tid[..8].to_string(),
            rd.title.clone(),
            format_datetime(rd.created_utc),
            rd.msg_count.to_string(),
            format_size(rd.size_bytes),
        ];

        // Convert dynamic columns
        for col in &union_columns {
            let v = lookup_dynamic(&rd.dynamic, col);
            row.push(v);
        }

        string_rows.push(row);
    }

    /* ---------------------------------------------------------------------
       TRUNCATE AROUND ACTIVE THREAD IF show_all = false
    --------------------------------------------------------------------- */
    if !show_all {
        string_rows = truncate_rows(string_rows, active_idx);
    }

    /* ---------------------------------------------------------------------
       RENDER TABLE
    --------------------------------------------------------------------- */
    let header_refs: Vec<&str> = headers.iter().map(|s| s.as_str()).collect();

    render_table(
        "Conversations",
        &header_refs,
        string_rows,
        active_idx,
    );
}

/* =========================================================================
   HELPERS — parsing, dynamic metadata, size, formatting
============================================================================ */

fn parse_created(convo: &Value) -> DateTime<Utc> {
    let created_str = convo["created_at"].as_str().unwrap_or("");
    DateTime::parse_from_rfc3339(created_str)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn extract_message_ids(convo: &Value) -> Vec<String> {
    convo["messages"]
        .as_array().unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect()
}

fn lookup_dynamic(dynamic: &[(String, String, String)], col: &str) -> String {
    for (internal, _, value) in dynamic {
        if internal == col {
            return value.clone();
        }
    }
    "".into()
}

fn extract_dynamic_columns(
    convo: &Value
) -> (
    Vec<(String, String, String)>,  // dynamic entries
    Vec<String>,                     // internal column names
    Vec<(String, String)>            // (internal → display name)
) {
    let mut dyn_entries = Vec::new();
    let mut col_names = Vec::new();
    let mut disp_map = Vec::new();

    let columns = convo["meta"]["columns"].as_object().unwrap();
    let empty = serde_json::Map::new();
    let disp = convo["meta"]["display_names"]
        .as_object()
        .unwrap_or(&empty);

    for (internal, array_val) in columns {
        let display = disp.get(internal)
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| to_title_case(internal));


        let joined = array_val.as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        dyn_entries.push((internal.clone(), display.clone(), joined));
        col_names.push(internal.clone());
        disp_map.push((internal.clone(), display));
    }

    (dyn_entries, col_names, disp_map)
}

/* =========================================================================
   FILE SIZE AGGREGATION (conversation.json + messages + markdown)
============================================================================ */

fn compute_full_conversation_size(
    fur_dir: &Path,
    tid: &str,
    msg_ids: &[String],
) -> u64 {
    let mut total = 0;

    // conversation JSON
    let convo_path = fur_dir.join("threads").join(format!("{}.json", tid));
    total += file_size(&convo_path);

    // message JSON + markdown attachments
    for mid in msg_ids {
        let msg_path = fur_dir.join("messages").join(format!("{}.json", mid));
        total += file_size(&msg_path);

        if let Ok(content) = fs::read_to_string(&msg_path) {
            if let Ok(json) = serde_json::from_str::<Value>(&content) {
                if let Some(md_raw) = json["markdown"].as_str() {
                    let md_path = Path::new(md_raw);
                    let resolved = if md_path.is_absolute() {
                        md_path.to_path_buf()
                    } else {
                        Path::new(".").join(md_raw)
                    };
                    total += file_size(&resolved);
                }
            }
        }
    }

    total
}

fn file_size(path: &Path) -> u64 {
    fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

/* =========================================================================
   FORMATTING HELPERS
============================================================================ */

fn format_datetime(dt: DateTime<Utc>) -> String {
    let local: DateTime<Local> = DateTime::from(dt);
    format!("{} | {}", local.format("%Y-%m-%d"), local.format("%H:%M"))
}

fn format_size(bytes: u64) -> String {
    if bytes < 1_048_576 {
        format!("{} KB", (bytes as f64 / 1024.0).round() as u64)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/* =========================================================================
   TRUNCATION AROUND ACTIVE THREAD
============================================================================ */

fn truncate_rows(
    rows: Vec<Vec<String>>,
    active_idx: Option<usize>,
) -> Vec<Vec<String>> {
    if active_idx.is_none() {
        return rows;
    }
    let a = active_idx.unwrap();
    let win = 3;

    let start = a.saturating_sub(win);
    let end = (a + win).min(rows.len().saturating_sub(1));

    let mut out = Vec::new();

    if start > 0 {
        out.push(vec!["...".into()]);
    }

    for i in start..=end {
        out.push(rows[i].clone());
    }

    if end < rows.len() - 1 {
        out.push(vec!["...".into()]);
    }

    out
}

/* =========================================================================
   TITLE CASE
============================================================================ */

fn to_title_case(s: &str) -> String {
    s.split('_')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
