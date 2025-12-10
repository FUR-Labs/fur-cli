use std::fs;
use std::path::Path;
use serde_json::{Value, json};

/* ============================================================================
   GLOBAL COLUMN ENGINE
   Controls:
   - schema in index.json
   - propagation to all conversation files
   - normalization + display names
============================================================================ */

/// Load (global_columns, global_column_order) from index.json
pub fn load_global_schema(index: &Value) -> (serde_json::Map<String, Value>, Vec<String>) {
    let cols = index["global_columns"]
        .as_object()
        .cloned()
        .unwrap_or_default();

    let order = index["global_column_order"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    (cols, order)
}


/// Save updated global schema back to index.json
pub fn write_global_schema(
    index_path: &Path,
    index: &mut Value,
    global_cols: serde_json::Map<String, Value>,
    order: Vec<String>,
) {
    index["global_columns"] = Value::Object(global_cols);
    index["global_column_order"] = json!(order);

    fs::write(index_path, serde_json::to_string_pretty(index).unwrap()).unwrap();
}


/// Create a new global column (normalized internal key), and propagate to all conversations.
pub fn create_global_column(
    internal: &str,
    display: &str,
    index: &mut Value,
    index_path: &Path,
    fur_dir: &Path,
) {
    let (mut global_cols, mut order) = load_global_schema(index);

    if global_cols.contains_key(internal) {
        eprintln!("⚠️ Column '{}' already exists globally.", internal);
        return;
    }

    // Add to global schema
    global_cols.insert(internal.to_string(), json!({ "display": display }));
    order.push(internal.to_string());

    // Write updated schema to index.json
    write_global_schema(index_path, index, global_cols.clone(), order.clone());

    // Propagate to ALL conversations
    propagate_new_column_to_all_conversations(internal, fur_dir, index);

    println!("📌 Created GLOBAL column '{}'", internal);
}


/// Ensure all conversations have an empty list for the new column
fn propagate_new_column_to_all_conversations(
    internal: &str,
    fur_dir: &Path,
    index: &Value,
) {
    let threads_path = fur_dir.join("threads");

    for entry in fs::read_dir(threads_path).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|v| v.to_str()) != Some("json") {
            continue;
        }

        if let Ok(raw) = fs::read_to_string(&path) {
            if let Ok(mut convo) = serde_json::from_str::<Value>(&raw) {

                // 🧩 CRITICAL FIX: upgrade BEFORE touching meta
                convo = crate::schema::upgrade_conversation_schema(convo, index);

                // Guaranteed to exist after upgrade
                let meta = convo["meta"].as_object_mut().unwrap();
                let cols = meta["columns"].as_object_mut().unwrap();

                if !cols.contains_key(internal) {
                    cols.insert(internal.to_string(), json!([]));
                }

                fs::write(&path, serde_json::to_string_pretty(&convo).unwrap()).unwrap();
            }
        }
    }
}


/// Rename display name globally
pub fn rename_global_column(
    internal: &str,
    new_display: &str,
    index: &mut Value,
    index_path: &Path,
) {
    let (mut global_cols, order) = load_global_schema(index);

    if !global_cols.contains_key(internal) {
        eprintln!("❌ Column '{}' does not exist globally.", internal);
        return;
    }

    global_cols.insert(internal.into(), json!({ "display": new_display }));
    write_global_schema(index_path, index, global_cols, order);

    println!("✏️ Renamed GLOBAL column '{}' → '{}'", internal, new_display);
}


/// Guarantee that this conversation has all global columns
pub fn ensure_convo_has_all_global_columns(
    convo: &mut Value,
    global_cols: &serde_json::Map<String, Value>,
) {
    let meta = convo["meta"].as_object_mut().unwrap();

    if !meta.contains_key("columns") {
        meta.insert("columns".into(), json!({}));
    }

    let cols = meta["columns"].as_object_mut().unwrap();

    for internal in global_cols.keys() {
        if !cols.contains_key(internal) {
            cols.insert(internal.clone(), json!([]));
        }
    }
}
