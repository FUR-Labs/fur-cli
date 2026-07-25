use serde_json::json;
use serde_json::Value;
use std::fs;
use std::path::Path;

pub fn lock_file() -> &'static str {
    ".fur/lock.json"
}

pub fn is_locked() -> bool {
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

    fs::write(lock_file(), serde_json::to_string_pretty(&data).unwrap()).unwrap();
}

pub fn remove_lock() {
    let path = Path::new(lock_file());

    if path.exists() {
        let _ = fs::remove_file(path);
    }
}
