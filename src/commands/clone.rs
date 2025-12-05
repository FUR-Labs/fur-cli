use std::fs;
use std::path::Path;
use serde_json::Value;
use crate::helpers::cloning::{
    load_conversation_metadata, 
    make_new_conversation_header, 
    build_id_remap,
    clone_all_messages,
    write_new_conversation,
    update_index
};

/// Clone the currently active conversation
pub fn run_clone_from_active(title: Option<String>) {
    let index_path = Path::new(".fur").join("index.json");
    let index: Value =
        serde_json::from_str(&fs::read_to_string(&index_path).unwrap()).unwrap();

    let active_tid = index["active_thread"]
        .as_str()
        .expect("❌ No active conversation set");

    run_clone(active_tid, title);
}

/// Deep clone a specific conversation
pub fn run_clone(tid: &str, title: Option<String>) {
    let fur_dir = Path::new(".fur");
    let old_convo_path = fur_dir.join("threads").join(format!("{}.json", tid));

    if !old_convo_path.exists() {
        eprintln!("❌ Conversation {} not found.", tid);
        return;
    }

    let (old_title, old_messages) = load_conversation_metadata(&old_convo_path);

    let (new_convo_id, new_title, timestamp) =
        make_new_conversation_header(&old_title, title);

    let id_map = build_id_remap(&old_messages);

    fs::create_dir_all("chats").ok();

    clone_all_messages(&id_map, &old_messages);

    write_new_conversation(
        &new_convo_id,
        &new_title,
        &timestamp,
        &id_map,
        &old_messages,
    );

    update_index(&new_convo_id);

    println!(
        "🌀 Successfully cloned conversation \"{}\" → new ID {}",
        old_title,
        &new_convo_id[..8]
    );
}
