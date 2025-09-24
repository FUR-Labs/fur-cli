use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;
use std::fs;

/// Helper: bootstrap a `.fur` directory in a temp folder
fn setup_fur(tmp: &std::path::Path) {
    let fur_dir = tmp.join(".fur");
    fs::create_dir_all(&fur_dir).unwrap();
    fs::write(fur_dir.join("avatars.json"), r#"{
        "main": "andrew",
        "andrew": "🦊",
        "ai": "🤖"
    }"#).unwrap();
    fs::write(fur_dir.join("index.json"), r#"{
        "threads": [],
        "active_thread": null,
        "current_message": null,
        "schema_version": "0.2"
    }"#).unwrap();
}

#[test]
fn avatar_view_lists_main_and_secondary() {
    let tmp = tempdir().unwrap();
    std::env::set_current_dir(&tmp).unwrap();
    setup_fur(tmp.path());

    // Run `fur avatar --view` (same as `fur avatar`)
    Command::cargo_bin("fur").unwrap()
        .args(&["avatar", "--view"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Avatars")) // sleek header
        .stdout(predicate::str::contains("⭐ main"))
        .stdout(predicate::str::contains("ai"))
        .stdout(predicate::str::contains("andrew"));
}
