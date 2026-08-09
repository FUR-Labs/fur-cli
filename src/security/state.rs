use serde_json::json;
use serde_json::Value;
use std::fs;
use std::path::Path;

use crate::schema::rebuild::LOCK_SENTINEL;

pub fn lock_file() -> &'static str {
    ".fur/lock.json"
}

/// True when either the disposable flag in `.fur/` or the durable sentinel in
/// `chats/` says so.
///
/// viceroy: previously consulted `.fur/lock.json` alone. Once `.fur/` became
/// rebuildable, deleting it discarded the only record that `chats/` was
/// ciphertext — `fur unlock` then reported "already unlocked" and refused to
/// decrypt. The sentinel lives with the data it describes.
pub fn is_locked() -> bool {
    if Path::new("chats").join(LOCK_SENTINEL).exists() {
        return true;
    }

    let path = Path::new(lock_file());

    if !path.exists() {
        return false;
    }

    let content = fs::read_to_string(path).unwrap_or_default();

    if let Ok(v) = serde_json::from_str::<Value>(&content) {
        return v["locked"].as_bool().unwrap_or(false);
    }

    false
}

pub fn write_lock() {
    let data = json!({
        "locked": true,
        "algorithm": "AES-256-GCM",
        "version": 1
    });

    if Path::new(".fur").exists() {
        let _ = fs::write(lock_file(), serde_json::to_string_pretty(&data).unwrap());
    }
}

pub fn remove_lock() {
    let path = Path::new(lock_file());

    if path.exists() {
        let _ = fs::remove_file(path);
    }
}