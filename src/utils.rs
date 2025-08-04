use std::fs;

pub fn load_json(path: &str) -> serde_json::Value {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Couldn't read file: {}: {}", path, e));
    serde_json::from_str(&content).expect("Invalid JSON")
}
