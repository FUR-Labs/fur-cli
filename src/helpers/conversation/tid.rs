use serde_json::Value;

/* ============================================================
   resolve_tid()
   Resolve a conversation ID from user input or default to active
   - Handles prefixes
   - Detects ambiguities
   - Returns clean String or None
============================================================ */

pub fn resolve_tid(
    index: &Value,
    id_opt: &Option<String>,
) -> Option<String> {
    // Load list of thread IDs
    let threads: Vec<String> = index["threads"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    /* --------------------------------------------------------
       CASE 1: user supplied a prefix → resolve it
    -------------------------------------------------------- */
    if let Some(prefix) = id_opt {
        let matches: Vec<&String> = threads
            .iter()
            .filter(|tid| tid.starts_with(prefix))
            .collect();

        match matches.len() {
            0 => {
                eprintln!("❌ No conversation matches prefix '{}'", prefix);
                return None;
            }
            1 => return Some(matches[0].clone()),
            _ => {
                eprintln!("❌ Ambiguous prefix '{}': {:?}", prefix, matches);
                return None;
            }
        }
    }

    /* --------------------------------------------------------
       CASE 2: no prefix → use active thread
    -------------------------------------------------------- */
    let active = index["active_thread"].as_str().unwrap_or("");

    if active.is_empty() {
        eprintln!("❌ No active conversation selected.");
        return None;
    }

    Some(active.to_string())
}

