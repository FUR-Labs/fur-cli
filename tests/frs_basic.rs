use tempfile::tempdir;
use std::fs;
use std::env;
use fur_cli::commands::run::run_frs;

#[test]
fn run_basic_script() {
    let tmp = tempdir().unwrap();
    env::set_current_dir(&tmp).unwrap();

    // Bootstrap .fur
    fs::create_dir_all(".fur/threads").unwrap();
    fs::create_dir_all(".fur/messages").unwrap();
    fs::create_dir_all(".fur/tmp").unwrap();
    fs::write(".fur/index.json", r#"{
        "threads": [],
        "active_thread": null,
        "current_message": null,
        "schema_version": "0.2"
    }"#).unwrap();
    fs::write(".fur/avatars.json", r#"{"main": "test"}"#).unwrap();

    // Write test script
    fs::write("test.frs", r#"
        new "Test Script"
        user = test
        jot "hello world"
        store
        status
    "#).unwrap();

    run_frs("test.frs");
}
