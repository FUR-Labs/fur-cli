use std::path::Path;
use clap::Parser;
use colored::*;
use serde_json::{Value, json};

use crate::helpers::search::{
    parse_queries,
    list_conversations,
    search_messages_in_conversation,
};


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

        let mut matches = search_messages_in_conversation(
            &msg_ids,
            &messages_dir,
            &queries,
        );

        if let Some(limit) = args.limit {
            if matches.len() > limit {
                matches.truncate(limit);
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
    use std::path::PathBuf;

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
