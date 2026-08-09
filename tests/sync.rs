use fur_cli::schema::bridge::sync_conversation;
use fur_cli::schema::document::parse;
use std::fs;
use std::path::Path;

fn boot(tmp: &Path) {
    let _ = fs::remove_dir_all(tmp);
    fs::create_dir_all(tmp.join(".fur/threads")).unwrap();
    fs::create_dir_all(tmp.join(".fur/messages")).unwrap();
    fs::create_dir_all(tmp.join("chats")).unwrap();
}

fn thread(title: &str, msgs: &str) -> String {
    format!(r#"{{"id":"14e87a09-d24e-4f4e-be73-7c76b4a15f5f","title":"{}",
      "created_at":"2026-08-09T01:23:14Z","messages":[{}],"tags":[],"schema_version":"0.3"}}"#, title, msgs)
}

fn msg(id: &str, avatar: &str, text: &str) -> String {
    format!(r#"{{"id":"{}","avatar":"{}","timestamp":"2026-08-09T01:23:58Z",
      "text":"{}","markdown":null,"attachment":null}}"#, id, avatar, text)
}

#[test]
fn sync_tracks_appends_and_renames() {
    let tmp = std::env::temp_dir().join("fur_sync_test");
    boot(&tmp);
    std::env::set_current_dir(&tmp).unwrap();
    let fur = Path::new(".fur");
    let tid = "14e87a09-d24e-4f4e-be73-7c76b4a15f5f";

    // first jot
    fs::write(".fur/messages/m1.json", msg("m1","andrew","first")).unwrap();
    fs::write(format!(".fur/threads/{}.json", tid), thread("Hello", "\"m1\"")).unwrap();
    sync_conversation(Path::new("."), fur, tid).unwrap();

    let spine = Path::new("chats/hello-14e87a09/convo.md");
    assert!(spine.exists());
    assert_eq!(parse(&fs::read_to_string(spine).unwrap()).unwrap().messages.len(), 1);

    // second jot -> spine regenerated, now two messages
    fs::write(".fur/messages/m2.json", msg("m2","gpt5","second")).unwrap();
    fs::write(format!(".fur/threads/{}.json", tid), thread("Hello", "\"m1\",\"m2\"")).unwrap();
    sync_conversation(Path::new("."), fur, tid).unwrap();
    assert_eq!(parse(&fs::read_to_string(spine).unwrap()).unwrap().messages.len(), 2);

    // delete a message -> spine shrinks
    fs::write(format!(".fur/threads/{}.json", tid), thread("Hello", "\"m2\"")).unwrap();
    sync_conversation(Path::new("."), fur, tid).unwrap();
    assert_eq!(parse(&fs::read_to_string(spine).unwrap()).unwrap().messages.len(), 1);

    // rename -> folder moves, no orphan left behind
    fs::write(format!(".fur/threads/{}.json", tid), thread("Renamed Thing", "\"m2\"")).unwrap();
    sync_conversation(Path::new("."), fur, tid).unwrap();
    assert!(!Path::new("chats/hello-14e87a09").exists(), "orphan folder left behind");
    let moved = Path::new("chats/renamed-thing-14e87a09/convo.md");
    assert!(moved.exists(), "folder not renamed");
    assert_eq!(parse(&fs::read_to_string(moved).unwrap()).unwrap().title, "Renamed Thing");

    let dirs: Vec<_> = fs::read_dir("chats").unwrap().filter_map(|e| e.ok()).collect();
    assert_eq!(dirs.len(), 1, "expected exactly one conversation folder");
}