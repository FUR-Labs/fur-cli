use std::fs;
use std::path::Path;

use serde_json::{Value, json};

use crate::helpers::tags::{normalize_tag, parse_tag_list};
use crate::helpers::conversation::tid::resolve_tid;

use crate::helpers::conversation::global::{
    load_global_schema,
    create_global_column,
    rename_global_column,
    ensure_convo_has_all_global_columns,
};

use crate::schema::upgrade_conversation_schema;

/* =============================================================================
   GLOBAL COLUMN CONTROLLER
============================================================================= */

pub fn handle_column_ops(
    args: crate::commands::conversation::ThreadArgs,
    index: &mut Value,
    fur_dir: &Path,
) {
    let index_path = fur_dir.join("index.json");

    let tid = match resolve_tid(index, &args.id) {
        Some(t) => t,
        None => return,
    };

    let convo_path = fur_dir.join("threads").join(format!("{}.json", tid));
    let raw = fs::read_to_string(&convo_path).unwrap();
    let mut convo: Value = serde_json::from_str(&raw).unwrap();

    convo = upgrade_conversation_schema(convo, index);

    let (global_cols, _order) = load_global_schema(index);

    /* ---------------- GLOBAL COLUMN CREATION ---------------- */
    if let Some(new_raw) = args.col_new {
        let internal = normalize_tag(&new_raw);
        let display = to_title_case(&internal);

        create_global_column(&internal, &display, index, &index_path, fur_dir);
        return;
    }

    /* ---------------- GLOBAL RENAME ------------------ */
    if let Some(rename_raw) = args.col_rename {
        let parts: Vec<&str> = rename_raw.split('=').collect();
        if parts.len() != 2 {
            eprintln!("❌ Format must be: --col-rename col=new-name");
            return;
        }

        let internal = normalize_tag(parts[0]);
        let new_display = parts[1].trim();
        rename_global_column(&internal, new_display, index, &index_path);
        return;
    }

    /* ---------------- ENSURE ALL GLOBAL COLUMNS EXIST ---------------- */
    ensure_convo_has_all_global_columns(&mut convo, &global_cols);

    let meta = convo["meta"].as_object_mut().unwrap();
    let cols = meta["columns"].as_object_mut().unwrap();

    /* ---------------- ADD VALUES ------------------ */
    if let Some(add_raw) = args.col_add {
        let parts: Vec<&str> = add_raw.split('=').collect();
        if parts.len() != 2 {
            eprintln!("❌ Format must be: --col-add col=v1,v2");
            return;
        }

        let internal = normalize_tag(parts[0]);
        if !global_cols.contains_key(&internal) {
            eprintln!("❌ Global column '{}' does not exist.", internal);
            return;
        }

        let new_vals = parse_tag_list(parts[1]);
        let arr = cols.get_mut(&internal).unwrap().as_array_mut().unwrap();

        for v in new_vals {
            if !arr.iter().any(|x| x.as_str() == Some(&v)) {
                arr.push(Value::String(v));
            }
        }

        if internal == "tags" {
            convo["tags"] = arr.clone().into();
        }

        save(&convo_path, &convo);
        println!("➕ Added values to '{}'", internal);
        return;
    }

    /* ---------------- REMOVE VALUES ------------------ */
    if let Some(remove_raw) = args.col_remove {
        let parts: Vec<&str> = remove_raw.split('=').collect();
        if parts.len() != 2 {
            eprintln!("❌ Format must be: --col-remove col=v1,v2");
            return;
        }

        let internal = normalize_tag(parts[0]);
        if !global_cols.contains_key(&internal) {
            eprintln!("❌ Global column '{}' does not exist.", internal);
            return;
        }

        let remove_vals = parse_tag_list(parts[1]);
        let arr = cols.get_mut(&internal).unwrap().as_array_mut().unwrap();

        arr.retain(|v| !remove_vals.contains(&v.as_str().unwrap_or("").to_string()));

        if internal == "tags" {
            convo["tags"] = arr.clone().into();
        }

        save(&convo_path, &convo);
        println!("➖ Removed values from '{}'", internal);
        return;
    }

    /* ---------------- CLEAR VALUES ------------------ */
    if let Some(clear_raw) = args.col_clear {
        let internal = normalize_tag(&clear_raw);
        if !global_cols.contains_key(&internal) {
            eprintln!("❌ Global column '{}' does not exist.", internal);
            return;
        }

        cols.insert(internal.clone(), json!([]));
        if internal == "tags" {
            convo["tags"] = json!([]);
        }

        save(&convo_path, &convo);
        println!("🧹 Cleared '{}'", internal);
        return;
    }
}

fn save(path: &Path, convo: &Value) {
    fs::write(path, serde_json::to_string_pretty(convo).unwrap()).unwrap();
}

fn to_title_case(s: &str) -> String {
    s.split('_')
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + chars.as_str()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
