use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;
use std::fs;

/// Helper: bootstrap a `.fur` directory in a temp folder
fn setup_fur(tmp: &std::path::Path) {
    let fur_dir = tmp.join(".fur");
    fs::create_dir_all(&fur_dir).unwrap();
    fs::write(fur_dir.join("avatars.json"), "{}").unwrap();
    fs::write(fur_dir.join("index.json"), r#"{
        "threads": [],
        "active_thread": null,
        "current_message": null,
        "schema_version": "0.2"
    }"#).unwrap();
}

#[test]
fn avatar_main_and_other_and_view() {
    let tmp = tempdir().unwrap();
    std::env::set_current_dir(&tmp).unwrap();
    setup_fur(tmp.path());

    // 1. Set main avatar
    Command::cargo_bin("fur").unwrap()
        .arg("avatar")
        .arg("andrew")
        .assert()
        .success()
        .stdout(predicate::str::contains("✔️ Main avatar set"));

    // 2. Add other avatar
    Command::cargo_bin("fur").unwrap()
        .args(&["avatar", "--other", "ai"])
        .assert()
        .success()
        .stdout(predicate::str::contains("✔️ Other avatar 'ai'"));

    // 3. View avatars
    Command::cargo_bin("fur").unwrap()
        .args(&["avatar", "--view"])
        .assert()
        .success()
        .stdout(predicate::str::contains("📇 Avatars:"))
        .stdout(predicate::str::contains("⭐ main"))
        .stdout(predicate::str::contains("ai"));
}
