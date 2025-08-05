use crate::utils::{load_json};

pub fn run_tree() {

    let index = load_json(".fur/index.json");
    let thread_id = index["active_thread"].as_str().unwrap();
    let thread_path = format!(".fur/threads/{}.json", thread_id);
    let thread = load_json(&thread_path);

    let current_id = index["current_message"].as_str().unwrap();

    let messages = thread["messages"].as_array().unwrap();

    let mut map = std::collections::HashMap::new();
    for msg_id in messages {
        if let Some(id) = msg_id.as_str() {
            let path = format!(".fur/messages/{}.json", id);
            let msg_json = load_json(&path);
            map.insert(id.to_string(), msg_json);
        }
    }

    let mut path_to_root = vec![];
    let mut id = current_id;

    while let Some(msg) = map.get(id) {
        path_to_root.push(id);
        id = msg["parent"].as_str().unwrap_or("");
        if id.is_empty() { break; }
    }

    path_to_root.reverse(); // So root is first

    // Start recursive printing from root
    if let Some(root_id) = path_to_root.first() {
        print_tree(&map, root_id, current_id, 0);
    }
}

fn print_tree(
    map: &std::collections::HashMap<String, serde_json::Value>,
    current_id: &str,
    target_id: &str,
    depth: usize
) {
    if let Some(msg) = map.get(current_id) {
        let role = msg["role"].as_str().unwrap_or("?");
        let icon = match role {
            "user" => "👤",
            "assistant" => "🧠",
            _ => "❓"
        };
        let id_display = &current_id[..8];
        let text = msg.get("text")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| msg.get("markdown").and_then(|v| v.as_str()).unwrap_or("<no content>"));

        let preview = text.lines().next().unwrap_or("").trim();
        let marker = if current_id == target_id { "🌳" } else { " " };
        let indent = "  >  ".repeat(depth);

        if msg.get("markdown").is_some() {
            println!("{indent}{marker} {icon} [{role}] \"{preview}\" 📄 {id_display}");
        } else {
            println!("{indent}{marker} {icon} [{role}] \"{preview}\" {id_display}");
        }


        let empty_children = vec![];
        let children = msg["children"].as_array().unwrap_or(&empty_children);
        for child_id in children {
            if let Some(child_str) = child_id.as_str() {
                print_tree(map, child_str, target_id, depth + 1);
            }
        }
    }
}
