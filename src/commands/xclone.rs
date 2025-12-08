use std::fs;
use std::path::{Path};
use serde_json::{Value, json};
use crate::helpers::cloning::{
    load_conversation_metadata,
    make_new_conversation_header,
    build_id_remap,
};

/// Ensure target project has a minimal .fur/ structure + chats/
fn ensure_target_project(dst_root: &Path) {
    let threads = dst_root.join("threads");
    let messages = dst_root.join("messages");
    let chats = dst_root.parent().unwrap().join("chats"); // <-- FIX HERE

    fs::create_dir_all(&threads).unwrap();
    fs::create_dir_all(&messages).unwrap();
    fs::create_dir_all(&chats).unwrap();
}

/// Clone all messages from source → destination .fur/messages + chats/
fn clone_messages_into_target(
    src_msgs_dir: &Path,
    dst_msgs_dir: &Path,
    id_map: &std::collections::HashMap<String, String>,
    old_messages: &[String],
) {
    for old_id in old_messages {
        let src_path = src_msgs_dir.join(format!("{}.json", old_id));

        let old_msg: Value =
            serde_json::from_str(&fs::read_to_string(&src_path).unwrap()).unwrap();

        let new_id = id_map.get(old_id).unwrap();
        let mut new_msg = old_msg.clone();

        // Remap ID
        new_msg["id"] = json!(new_id);

        // Remap structural pointers
        if let Some(p) = old_msg["parent"].as_str() {
            new_msg["parent"] = id_map.get(p).map(|s| json!(s)).unwrap_or(json!(null));
        } else {
            new_msg["parent"] = json!(null);
        }

        if let Some(children) = old_msg["children"].as_array() {
            new_msg["children"] = json!(children.iter()
                .filter_map(|v| v.as_str())
                .filter_map(|id| id_map.get(id))
                .collect::<Vec<_>>());
        }

        if let Some(branches) = old_msg["branches"].as_array() {
            new_msg["branches"] = json!(branches.iter()
                .filter_map(|v| v.as_str())
                .filter_map(|id| id_map.get(id))
                .collect::<Vec<_>>());
        }

        // --- MARKDOWN FIX HERE ---
        if let Some(md_rel) = old_msg["markdown"].as_str() {
            let src_md = src_msgs_dir.parent().unwrap().parent().unwrap().join(md_rel);
            let dst_md = dst_msgs_dir
                .parent().unwrap().parent().unwrap() // <target> root
                .join(md_rel); // always "chats/...md"

            if src_md.exists() {
                fs::create_dir_all(dst_md.parent().unwrap()).unwrap();
                fs::copy(&src_md, &dst_md)
                    .expect("❌ Failed to copy markdown attachment");
            }

            new_msg["markdown"] = json!(md_rel);
        } else {
            new_msg["markdown"] = json!(null);
        }

        // Write new message JSON
        let dst_path = dst_msgs_dir.join(format!("{}.json", new_id));
        fs::write(dst_path, serde_json::to_string_pretty(&new_msg).unwrap()).unwrap();
    }
}

/// Write the new conversation header into <target>/.fur/threads/
fn write_new_convo_into_target(
    dst_root: &Path,
    new_tid: &str,
    new_title: &str,
    timestamp: &str,
    id_map: &std::collections::HashMap<String, String>,
    old_messages: &[String],
) {
    let threads_dir = dst_root.join("threads");

    let new_messages: Vec<String> = old_messages
        .iter()
        .map(|old| id_map.get(old).unwrap().clone())
        .collect();

    let convo = json!({
        "id": new_tid,
        "title": new_title,
        "created_at": timestamp,
        "messages": new_messages,
        "tags": []
    });

    let new_path = threads_dir.join(format!("{}.json", new_tid));
    fs::write(new_path, serde_json::to_string_pretty(&convo).unwrap()).unwrap();
}

/// Append the cloned TID into <target>/.fur/index.json
fn update_target_index(dst_root: &Path, new_tid: &str) {
    let index_path = dst_root.join("index.json");

    let mut index: Value = if index_path.exists() {
        serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap()
    } else {
        // If no index.json exists, create a minimal one
        json!({
            "threads": [],
            "active_thread": null,
            "current_message": null,
            "created_at": chrono::Utc::now().to_rfc3339()
        })
    };

    index["threads"].as_array_mut().unwrap().push(json!(new_tid));
    index["active_thread"] = json!(new_tid);
    index["current_message"] = json!(null);

    fs::write(index_path, serde_json::to_string_pretty(&index).unwrap()).unwrap();
}

/// Main entry for the xclone command
pub fn run_xclone(to: &str, tid: &str, title: Option<String>) {
    let src_root = Path::new(".fur");
    let dst_root = Path::new(to).join(".fur");

    ensure_target_project(&dst_root);

    let src_convo_path = src_root.join("threads").join(format!("{}.json", tid));
    if !src_convo_path.exists() {
        eprintln!("❌ Conversation {} not found.", tid);
        return;
    }

    let (old_title, old_messages) = load_conversation_metadata(&src_convo_path);

    let (new_tid, new_title, timestamp) =
        make_new_conversation_header(&old_title, title);

    let id_map = build_id_remap(&old_messages);

    clone_messages_into_target(
        &src_root.join("messages"),
        &dst_root.join("messages"),
        &id_map,
        &old_messages,
    );

    write_new_convo_into_target(
        &dst_root,
        &new_tid,
        &new_title,
        &timestamp,
        &id_map,
        &old_messages,
    );

    update_target_index(&dst_root, &new_tid);

    println!(
        "🌀 Deep-cloned conversation \"{}\" → {} into {}",
        old_title,
        &new_tid[..8],
        dst_root.display()
    );
}
