use std::fs;
use tempfile::tempdir;
use fur_cli::commands::run::run_frs;

#[test]
fn run_branch_script() {
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
        new "Branch Test"
        user = ai

        jot "Root message"
        branch {
            jot "Child A"
            jot "Child B"
        }
        store
    "#;
    fs::write("branch.frs", script).unwrap();

    run_frs("branch.frs");

    assert!(fs::read_dir(".fur/threads").unwrap().count() == 1);
    assert!(fs::read_dir(".fur/messages").unwrap().count() >= 2);
}
