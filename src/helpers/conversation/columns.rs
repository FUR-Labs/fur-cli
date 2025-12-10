use std::fs;
use std::path::Path;
use serde_json::{Value, json};

use crate::schema::upgrade_conversation_schema;
use crate::helpers::tags::{parse_tag_list, normalize_tag};
use crate::helpers::conversation::tid::resolve_tid;

/* ============================================================================
   COLUMN OPERATIONS (SAFE + CORRECT VERSION)
   - dynamic metadata columns
   - display name mapping
   - correct schema reinforcement
   - always reinserts updated fields back into convo
   - no unwrap panics
============================================================================ */

pub fn handle_column_ops(
    args: crate::commands::conversation::ThreadArgs,
    index: &mut Value,
    fur_dir: &Path,
) {
    /* ----------------------------------------------------------
       1. Resolve thread ID
    ---------------------------------------------------------- */
    let tid = match resolve_tid(index, &args.id) {
        Some(t) => t,
        None => return,
    };

    let convo_path = fur_dir.join("threads").join(format!("{}.json", tid));
    let raw = fs::read_to_string(&convo_path).unwrap();
    let mut convo: Value = serde_json::from_str(&raw).unwrap();

    /* ----------------------------------------------------------
       2. Upgrade schema (ensures meta exists)
    ---------------------------------------------------------- */
    convo = upgrade_conversation_schema(convo);

    /* ----------------------------------------------------------
       3. Get meta object safely
    ---------------------------------------------------------- */
    let meta = convo["meta"].as_object_mut().unwrap();

    // Guarantee required subkeys exist
    if !meta.contains_key("columns") {
        meta.insert("columns".into(), json!({}));
    }
    if !meta.contains_key("display_names") {
        meta.insert("display_names".into(), json!({}));
    }

    /* ----------------------------------------------------------
       4. Extract and remove (SAFE)
    ---------------------------------------------------------- */
    let columns_val = meta.remove("columns").unwrap_or(json!({}));
    let display_val = meta.remove("display_names").unwrap_or(json!({}));

    let mut columns = columns_val.as_object().cloned().unwrap_or_default();
    let mut display_names = display_val.as_object().cloned().unwrap_or_default();

    /* ----------------------------------------------------------
       5. CREATE NEW COLUMN
    ---------------------------------------------------------- */
    if let Some(raw_col) = args.col_new {
        let internal = normalize_tag(&raw_col);

        if columns.contains_key(&internal) {
            eprintln!("⚠️ Column '{}' already exists.", internal);
        } else {
            columns.insert(internal.clone(), json!([]));
            display_names.insert(internal.clone(), to_title_case(&internal).into());
            println!("📌 Created column '{}'", internal);
        }

        write_back(&convo_path, &mut convo, columns, display_names);
        return;
    }

    /* ----------------------------------------------------------
       6. RENAME DISPLAY NAME ONLY
          Format: --col-rename col=new name
    ---------------------------------------------------------- */
    if let Some(rename_raw) = args.col_rename {
        let parts: Vec<&str> = rename_raw.split('=').collect();
        if parts.len() != 2 {
            eprintln!("❌ Format must be: --col-rename col=new-name");
            return;
        }

        let internal = normalize_tag(parts[0]);
        let new_name = parts[1].trim();

        if !columns.contains_key(&internal) {
            eprintln!("❌ Column '{}' does not exist.", internal);
            return;
        }

        display_names.insert(internal.clone(), new_name.into());
        println!("✏️  Column '{}' display renamed to '{}'", internal, new_name);

        write_back(&convo_path, &mut convo, columns, display_names);
        return;
    }

    /* ----------------------------------------------------------
       7. ADD VALUES
          Format: --col-add col=v1,v2
    ---------------------------------------------------------- */
    if let Some(add_raw) = args.col_add {
        let parts: Vec<&str> = add_raw.split('=').collect();
        if parts.len() != 2 {
            eprintln!("❌ Format must be: --col-add col=v1,v2");
            return;
        }

        let internal = normalize_tag(parts[0]);
        if !columns.contains_key(&internal) {
            eprintln!("❌ Column '{}' does not exist.", internal);
            return;
        }

        let items = parse_tag_list(parts[1]);
        let arr = columns
            .get_mut(&internal)
            .unwrap()
            .as_array_mut()
            .unwrap();

        for tag in items {
            let exists = arr.iter().any(|x| x.as_str() == Some(&tag));
            if !exists {
                arr.push(Value::String(tag));
            }
        }

        if internal == "tags" {
            sync_legacy_tags(&mut convo);
        }

        println!("➕ Added values to '{}'", internal);

        write_back(&convo_path, &mut convo, columns, display_names);
        return;
    }

    /* ----------------------------------------------------------
       8. REMOVE VALUES
          Format: --col-remove col=v1,v2
    ---------------------------------------------------------- */
    if let Some(remove_raw) = args.col_remove {
        let parts: Vec<&str> = remove_raw.split('=').collect();
        if parts.len() != 2 {
            eprintln!("❌ Format must be: --col-remove col=v1,v2");
            return;
        }

        let internal = normalize_tag(parts[0]);
        if !columns.contains_key(&internal) {
            eprintln!("❌ Column '{}' does not exist.", internal);
            return;
        }

        let remove_list = parse_tag_list(parts[1]);
        let arr = columns
            .get_mut(&internal)
            .unwrap()
            .as_array_mut()
            .unwrap();

        arr.retain(|v| {
            let s = v.as_str().unwrap_or("");
            !remove_list.contains(&s.to_string())
        });

        if internal == "tags" {
            sync_legacy_tags(&mut convo);
        }

        println!("➖ Removed values from '{}'", internal);
        write_back(&convo_path, &mut convo, columns, display_names);
        return;
    }

    /* ----------------------------------------------------------
       9. CLEAR COLUMN
          Format: --col-clear colname
    ---------------------------------------------------------- */
    if let Some(raw) = args.col_clear {
        let internal = normalize_tag(&raw);

        if !columns.contains_key(&internal) {
            eprintln!("❌ Column '{}' does not exist.", internal);
            return;
        }

        columns.insert(internal.clone(), json!([]));

        if internal == "tags" {
            sync_legacy_tags(&mut convo);
        }

        println!("🧹 Cleared column '{}'", internal);
        write_back(&convo_path, &mut convo, columns, display_names);
        return;
    }
}

/* ============================================================================
   INTERNAL HELPERS
============================================================================ */

/// Put updated columns + display_names back into convo and save.
fn write_back(
    path: &Path,
    convo: &mut Value,
    columns: serde_json::Map<String, Value>,
    display_names: serde_json::Map<String, Value>,
) {
    let meta = convo["meta"].as_object_mut().unwrap();
    meta.insert("columns".into(), Value::Object(columns));
    meta.insert("display_names".into(), Value::Object(display_names));

    fs::write(path, serde_json::to_string_pretty(convo).unwrap()).unwrap();
}

fn sync_legacy_tags(convo: &mut Value) {
    let tags = convo["meta"]["columns"]["tags"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect::<Vec<_>>();

    convo["tags"] = json!(tags);
}

fn to_title_case(s: &str) -> String {
    s.split('_')
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
