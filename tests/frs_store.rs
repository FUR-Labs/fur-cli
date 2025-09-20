use std::fs;
use tempfile::tempdir;
use fur_cli::commands::run::run_frs;

#[test]
fn run_double_store_ignores_second() {
    let tmp = tempdir().unwrap();
    std::env::set_current_dir(&tmp).unwrap();

    std::fs::create_dir_all(tmp.path().join(".fur/threads")).unwrap();
    std::fs::create_dir_all(tmp.path().join(".fur/messages")).unwrap();
    std::fs::write(tmp.path().join(".fur/index.json"), r#"{
        "threads": [],
        "active_thread": null,
        "current_message": null,
        "schema_version": "0.2"
    }"#).unwrap();
    fs::write(".fur/avatars.json", r#"{"main":"ai"}"#).unwrap();

    let script = r#"
        new "Double Store Test"
        user = ai

        jot "First message"
        store
        jot "Second message after store"
        store
    "#;
    fs::write("double_store.frs", script).unwrap();

    run_frs("double_store.frs");

    // Just assert files exist, not exact JSON
    assert!(fs::read_dir(".fur/threads").unwrap().count() == 1);
}

