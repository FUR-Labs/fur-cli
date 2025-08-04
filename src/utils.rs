use std::fs;
use serde_json::Value;

/// Load and parse a JSON file
pub fn load_json(path: &str) -> Value {
    let data = fs::read_to_string(path).expect(&format!("Couldn't read file: {}", path));
    serde_json::from_str(&data).expect("Invalid JSON")
}
