use assert_cmd::Command;
use predicates::str::contains;
use tempfile::tempdir;
use std::fs;

/// Helper: bootstrap a `.fur` directory with minimal state
fn setup_fur(tmp: &std::path::Path) {
    let fur_dir = tmp.join(".fur");
    std::fs::create_dir_all(fur_dir.join("threads")).unwrap();
    std::fs::create_dir_all(fur_dir.join("messages")).unwrap();
    fs::write(
        fur_dir.join("index.json"),
        r#"{
            "threads": [],
            "active_thread": null,
            "current_message": null,
            "schema_version": "0.2"
        }"#,
    )
    .unwrap();
    fs::write(fur_dir.join("avatars.json"), r#"{"main":"me"}"#).unwrap();
}

#[test]
fn chat_creates_file_and_message() {
    let tmp = tempdir().unwrap();
    std::env::set_current_dir(&tmp).unwrap();
    setup_fur(tmp.path());

    // 1. Start a new thread so we have an active context
    Command::cargo_bin("fur").unwrap()
        .args(&["new", "Chat Test"])
        .assert()
        .success()
        .stdout(contains("[NEW] Thread created"));

    // 2. Simulate running `fur chat` with multi-line input
    let input = "# Title\n\nHello world from chat!\n";
    let mut cmd = Command::cargo_bin("fur").unwrap();
    cmd.arg("chat")
        .arg("gpt5")
        .write_stdin(input)
        .assert()
        .success()
        .stdout(contains("💾 Saved to"))
        .stdout(contains("✍️ Message jotted down"));

    // 3. Verify file was created under chats/
    let chats_dir = tmp.path().join("chats");
    assert!(chats_dir.exists());
    let files: Vec<_> = fs::read_dir(&chats_dir).unwrap().collect();
    assert!(!files.is_empty());

    // 4. Verify at least one message was saved in .fur/messages/
    let messages_dir = tmp.path().join(".fur/messages");
    let msg_count = fs::read_dir(&messages_dir).unwrap().count();
    assert!(msg_count > 0);
}
