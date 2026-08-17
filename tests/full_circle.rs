// The architecture test: export -> delete .fur -> rebuild -> export again.
// If the second export is byte-identical, chats/ is genuinely durable.
use fur_cli::schema::bridge::sync_conversation;
use fur_cli::schema::document::parse;
use fur_cli::schema::rebuild::{detect_state, rebuild, ProjectState};
use std::fs;
use std::path::Path;

#[test]
fn delete_fur_and_recover_from_chats() {
    let root = std::env::temp_dir().join("fur_full_circle");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".fur/threads")).unwrap();
    fs::create_dir_all(root.join(".fur/messages")).unwrap();
    fs::create_dir_all(root.join("chats")).unwrap();
    std::env::set_current_dir(&root).unwrap();

    let tid = "14e87a09-d24e-4f4e-be73-7c76b4a15f5f";
    let parent = "9c3d1f77-0000-4000-8000-000000000001";
    let child = "9c3d1f77-0000-4000-8000-000000000002";

    fs::write("chats/CHAT-20260809-012744.md", "# Long form\n\nBody.\n").unwrap();
    fs::write(".fur/messages/m1.json", r#"{"id":"m1","avatar":"andrew","timestamp":"2026-08-09T01:23:58Z","text":"give me a complex theory","markdown":null,"attachment":null}"#).unwrap();
    fs::write(".fur/messages/m2.json", r#"{"id":"m2","avatar":"gpt5","timestamp":"2026-08-09T01:27:45Z","text":null,"markdown":"chats/CHAT-20260809-012744.md","markdown_meta":{"hash":"ab15","size":22,"filename":"CHAT-20260809-012744.md"},"attachment":null}"#).unwrap();
    fs::write(".fur/messages/m3.json", r#"{"id":"m3","avatar":"andrew","timestamp":"2026-08-09T02:31:58Z","text":"gut boi","markdown":null,"attachment":null}"#).unwrap();
    fs::write(
        format!(".fur/threads/{}.json", tid),
        format!(
            r#"{{"id":"{}","title":"Hello","created_at":"2026-08-09T01:23:14Z","messages":["m1","m2","m3"],"tags":["demo"],"parents":["{}"],"children":["{}"],"schema_version":"0.3"}}"#,
            tid, parent, child
        ),
    )
    .unwrap();
    fs::write(".fur/index.json", format!(r#"{{"threads":["{}"],"active_thread":"{}","current_message":null,"schema_version":"0.3"}}"#, tid, tid)).unwrap();

    sync_conversation(Path::new("."), Path::new(".fur"), tid).unwrap();
    let spine = Path::new("chats/hello-14e87a09/convo.md");
    let before = fs::read_to_string(spine).unwrap();

    // lineage reached the durable document, and forced the schema bump
    let doc = parse(&before).unwrap();
    assert_eq!(doc.parents, vec![parent.to_string()]);
    assert_eq!(doc.children, vec![child.to_string()]);
    assert!(before.contains("fur_schema: 2"), "lineage did not bump the schema");

    // the moment of truth
    fs::remove_dir_all(".fur").unwrap();
    assert_eq!(detect_state(Path::new(".")), ProjectState::Unindexed);

    let summary = rebuild(Path::new("."), false).unwrap();
    assert_eq!(summary.conversations, 1);
    assert_eq!(summary.messages, 3);
    assert_eq!(summary.guessed_main.as_deref(), Some("andrew"));

    // lineage survived the trip through chats/ and back into .fur/
    let thread: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(format!(".fur/threads/{}.json", tid)).unwrap())
            .unwrap();
    assert_eq!(thread["parents"][0], parent);
    assert_eq!(thread["children"][0], child);

    // re-export from the rebuilt .fur and compare
    sync_conversation(Path::new("."), Path::new(".fur"), tid).unwrap();
    let after = fs::read_to_string(spine).unwrap();

    assert_eq!(before, after, "archive did not survive the round trip");
    assert!(Path::new("chats/hello-14e87a09/CHAT-20260809-012744.md").exists());
    println!("\n--- recovered spine ---\n{}", after);
}