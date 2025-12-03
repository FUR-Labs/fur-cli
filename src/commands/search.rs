use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;
use colored::*;
use serde_json::{Value, json};

/// Arguments for `fur search`
#[derive(Parser)]
pub struct SearchArgs {
    /// Search query (supports comma-separated list)
    pub query: String,

    /// Maximum matches per conversation (default: unlimited)
    #[arg(long)]
    pub limit: Option<usize>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

/// Entrypoint
pub fn run_search(args: SearchArgs) {
    let fur_dir = Path::new(".fur");
    if !fur_dir.exists() {
        eprintln!("❌ No .fur/ directory found. Run `fur new` first.");
        return;
    }

    let threads_dir = fur_dir.join("threads");
    let messages_dir = fur_dir.join("messages");

    if !threads_dir.exists() || !messages_dir.exists() {
        eprintln!("❌ Invalid .fur project structure.");
        return;
    }

    let queries = parse_queries(&args.query);
    if queries.is_empty() {
        eprintln!("❌ No valid search query provided.");
        return;
    }

    // Collect all results
    let mut output_json: Vec<Value> = Vec::new();

    let threads = list_conversations(&threads_dir);
    for (tid, convo_json) in threads {
        let title = convo_json["title"].as_str().unwrap_or("Untitled").to_string();
        let msg_ids: Vec<String> = convo_json["messages"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        let mut matches: Vec<Value> = Vec::new();

        // Search through each message
        for mid in &msg_ids {
            let msg_path = messages_dir.join(format!("{}.json", mid));
            if let Ok(content) = fs::read_to_string(&msg_path) {
                if let Ok(msg) = serde_json::from_str::<Value>(&content) {
                    let avatar = msg["avatar"].as_str().unwrap_or("unknown").to_string();

                    // --- Search message text ---
                    if let Some(text) = msg["text"].as_str() {
                        if let Some((q, snippet)) = match_any_query(text, &queries) {
                            matches.push(json!({
                                "message_id": mid,
                                "avatar": avatar,
                                "source": "text",
                                "query": q,
                                "snippet": snippet
                            }));
                            continue;
                        }
                    }

                    // --- Search markdown file contents ---
                    if let Some(md_path_raw) = msg["markdown"].as_str() {
                        if let Some(snippet_json) =
                            search_markdown(md_path_raw, &queries)
                        {
                            let q = snippet_json.0;
                            let snippet = snippet_json.1;

                            matches.push(json!({
                                "message_id": mid,
                                "avatar": avatar,
                                "source": "markdown",
                                "query": q,
                                "snippet": snippet
                            }));
                        }
                    }
                }
            }
        }

        if args.limit.is_some() {
            let n = args.limit.unwrap();
            if matches.len() > n {
                matches.truncate(n);
            }
        }

        if args.json {
            if !matches.is_empty() {
                output_json.push(json!({
                    "conversation_id": tid,
                    "title": title,
                    "matches": matches
                }));
            }
        } else {
            print_conversation_results(&tid, &title, &matches);
        }
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&output_json).unwrap());
    }
}

//
// ---------- Helpers ----------
//

/// Parse a search string into individual queries.
/// Supports:
///   "deep learning"
///   "deep learning, neural models"
///   "deep, learning"
fn parse_queries(q: &str) -> Vec<String> {
    let lowered = q.trim().to_lowercase();

    // Case 1: contains commas → split
    if lowered.contains(',') {
        return lowered
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }

    // Case 2: no comma → single query
    vec![lowered]
}

/// Load all conversation JSONs in threads/
fn list_conversations(threads_dir: &Path) -> Vec<(String, Value)> {
    let mut out = Vec::new();

    for entry in fs::read_dir(threads_dir).unwrap_or_else(|_| panic!("Cannot read threads dir")) {
        if let Ok(e) = entry {
            let path = e.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(json) = serde_json::from_str::<Value>(&content) {
                            out.push((stem.to_string(), json));
                        }
                    }
                }
            }
        }
    }

    out
}

/// Search plain text for any query. Returns (query, snippet)
fn match_any_query(text: &str, queries: &[String]) -> Option<(String, String)> {
    let lower = text.to_lowercase();

    for q in queries {
        if let Some(pos) = lower.find(q) {
            return Some((q.clone(), snippet_around(text, pos, q.len())));
        }
    }

    None
}

/// Search markdown content safely; returns Some((query, snippet)) or None
fn search_markdown(md_path_raw: &str, queries: &[String]) -> Option<(String, String)> {
    let md_path = PathBuf::from(md_path_raw);
    let path = if md_path.is_absolute() {
        md_path
    } else {
        PathBuf::from(".").join(md_path_raw)
    };

    let content = fs::read_to_string(&path).ok()?;
    match_any_query(&content, queries)
}

/// Extract a snippet of ±40 characters around the match
fn snippet_around(text: &str, pos: usize, len: usize) -> String {
    let start = pos.saturating_sub(40);
    let end = pos + len + 40;

    // Try a cheap, fast byte slice first (safe)
    if let Some(s) = text.get(start..end) {
        return s.replace('\n', " ");
    }

    // If that failed, fall back to UTF-8 safe slicing
    let chars: Vec<char> = text.chars().collect();
    let total = chars.len();

    let cpos = text[..pos].chars().count(); // convert byte offset → char index
    let clen = text[pos..pos+len].chars().count();

    let cstart = cpos.saturating_sub(40);
    let cend = (cpos + clen + 40).min(total);

    chars[cstart..cend]
        .iter()
        .collect::<String>()
        .replace('\n', " ")
}


/// Pretty print results for one conversation
fn print_conversation_results(tid: &str, title: &str, matches: &[Value]) {
    if matches.is_empty() {
        return;
    }

    println!(
        "\n{} {} ({})",
        "📘 Conversation:".bright_cyan().bold(),
        title.bold(),
        tid[..8].bright_black()
    );
    println!("{}", "─".repeat(60).dimmed());

    for m in matches {
        let mid = m["message_id"].as_str().unwrap_or("-");
        let avatar = m["avatar"].as_str().unwrap_or("-");
        let source = m["source"].as_str().unwrap_or("-");
        let snippet = m["snippet"].as_str().unwrap_or("-");

        println!(
            "{} {} {}",
            format!("[{}]", &mid[..8]).bright_yellow(),
            avatar.bright_green(),
            format!("({})", source).bright_black()
        );
        println!("  • {}\n", snippet);
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;
    use serde_json::Value;

    fn setup_fur_project() -> (tempfile::TempDir, PathBuf) {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();

        // Create .fur structure
        let fur = root.join(".fur");
        fs::create_dir_all(fur.join("threads")).unwrap();
        fs::create_dir_all(fur.join("messages")).unwrap();

        // Create index.json
        fs::write(
            fur.join("index.json"),
            r#"{
                "threads": ["t1","t2"],
                "active_thread": "t1",
                "current_message": null
            }"#,
        )
        .unwrap();

        // Conversation t1
        fs::write(
            fur.join("threads/t1.json"),
            r#"{
                "id": "t1",
                "title": "Deep Learning Notes",
                "created_at": "2024-01-01T00:00:00Z",
                "messages": ["m1","m2"],
                "tags": [],
                "schema_version": "0.2"
            }"#,
        )
        .unwrap();

        // Conversation t2
        fs::write(
            fur.join("threads/t2.json"),
            r#"{
                "id": "t2",
                "title": "Physics Notebook",
                "created_at": "2024-01-01T00:00:00Z",
                "messages": ["m3"],
                "tags": ["science"],
                "schema_version": "0.2"
            }"#,
        )
        .unwrap();

        // Message m1 (plain text hit)
        fs::write(
            fur.join("messages/m1.json"),
            r#"{
                "id": "m1",
                "avatar": "me",
                "timestamp": "2024-01-01T00:00:00Z",
                "text": "I am studying deep learning today.",
                "markdown": null,
                "attachment": null,
                "parent": null,
                "children": [],
                "branches": []
            }"#,
        )
        .unwrap();

        // Message m2 (markdown hit)
        fs::write(
            root.join("notes.md"),
            "Neural networks are universal function approximators.",
        )
        .unwrap();

        fs::write(
            fur.join("messages/m2.json"),
            format!(
                r#"{{
                    "id": "m2",
                    "avatar": "ai",
                    "timestamp": "2024-01-01T00:00:00Z",
                    "text": null,
                    "markdown": "{}",
                    "attachment": null,
                    "parent": null,
                    "children": [],
                    "branches": []
                }}"#,
                root.join("notes.md").display()
            ),
        )
        .unwrap();

        // Message m3 (no hit)
        fs::write(
            fur.join("messages/m3.json"),
            r#"{
                "id": "m3",
                "avatar": "me",
                "timestamp": "2024-01-01T00:00:00Z",
                "text": "Quantum mechanics is elegant.",
                "markdown": null,
                "attachment": null,
                "parent": null,
                "children": [],
                "branches": []
            }"#,
        )
        .unwrap();

        (dir, root)
    }

    #[test]
    fn test_search_simple_text() {
        let (_tmp, root) = setup_fur_project();
        std::env::set_current_dir(&root).unwrap();

        let args = SearchArgs {
            query: "deep learning".to_string(),
            limit: None,
            json: true,
        };

        // Capture output
        let out = capture_search_output(args, &root);

        let json: Value = serde_json::from_str(&out).unwrap();

        // Should have 1 conversation hit
        assert_eq!(json.as_array().unwrap().len(), 1);

        let matches = json[0]["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 1);

        assert_eq!(matches[0]["source"], "text");
    }

    #[test]
    fn test_search_markdown() {
        let (_tmp, root) = setup_fur_project();
        std::env::set_current_dir(&root).unwrap();

        let args = SearchArgs {
            query: "universal".to_string(),
            limit: None,
            json: true,
        };

        // Capture output
        let out = capture_search_output(args, &root);
        let json: Value = serde_json::from_str(&out).unwrap();

        let matches = json[0]["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 1);

        assert_eq!(matches[0]["source"], "markdown");
    }

    fn capture_search_output(args: SearchArgs, root: &Path) -> String {
        use assert_cmd::Command;

        let mut cmd = Command::cargo_bin("fur").expect("Binary exists");

        // Build command: fur search <query> --json
        let c = cmd.current_dir(root).arg("search");

        c.arg(&args.query);

        if args.json {
            c.arg("--json");
        }
        if let Some(limit) = args.limit {
            c.arg("--limit").arg(limit.to_string());
        }

        let out = c.assert().success().get_output().stdout.clone();

        String::from_utf8(out).unwrap()
    }


}
