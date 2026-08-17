//! Lineage edges: reciprocity when both ends are local, one-sided when not.
use std::fs;
use std::path::Path;

use fur_cli::commands::link::{apply, Outcome};
use fur_cli::schema::document::parse;

const A: &str = "aaaaaaaa-1111-4111-8111-111111111111";
const B: &str = "bbbbbbbb-2222-4222-8222-222222222222";
const ABSENT: &str = "ffffffff-9999-4999-8999-999999999999";


fn boot(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".fur/threads")).unwrap();
    fs::create_dir_all(root.join(".fur/messages")).unwrap();
    fs::create_dir_all(root.join("chats")).unwrap();

    for (id, title) in [(A, "Plan"), (B, "Weak Points")] {
        fs::write(
            root.join(format!(".fur/messages/{}-m.json", &id[..8])),
            format!(
                r#"{{"id":"{}-m","avatar":"andrew","timestamp":"2026-08-17T00:00:00Z","text":"body","markdown":null,"attachment":null}}"#,
                &id[..8]
            ),
        )
        .unwrap();
        fs::write(
            root.join(format!(".fur/threads/{}.json", id)),
            format!(
                r#"{{"id":"{}","title":"{}","created_at":"2026-08-17T00:00:00Z","messages":["{}-m"],"tags":[],"parents":[],"children":[],"schema_version":"0.3"}}"#,
                id, title, &id[..8]
            ),
        )
        .unwrap();
    }

    fs::write(
        root.join(".fur/index.json"),
        format!(
            r#"{{"threads":["{}","{}"],"active_thread":"{}","current_message":null,"schema_version":"0.3"}}"#,
            A, B, A
        ),
    )
    .unwrap();

    root
}

fn spine(root: &Path, slug_prefix: &str) -> String {
    let chats = root.join("chats");
    for entry in fs::read_dir(&chats).unwrap().flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(slug_prefix) {
            return fs::read_to_string(entry.path().join("convo.md")).unwrap();
        }
    }
    panic!("no conversation folder starting with {}", slug_prefix);
}

#[test]
fn linking_two_local_conversations_writes_both_ends() {
    let root = boot("fur_link_reciprocal");

    let report = apply(&root, Some(A), B, fur_cli::commands::link::edge_child(), false).unwrap();
    assert_eq!(report.near_outcome(), &Outcome::Added);

    let plan = parse(&spine(&root, "plan-")).unwrap();
    let weak = parse(&spine(&root, "weak-points-")).unwrap();

    assert_eq!(plan.children, vec![B.to_string()]);
    assert_eq!(weak.parents, vec![A.to_string()]);
    assert!(plan.parents.is_empty());
}

#[test]
fn linking_an_absent_conversation_is_one_sided_and_legal() {
    let root = boot("fur_link_dangling");

    apply(&root, Some(B), ABSENT, fur_cli::commands::link::edge_parent(), false).unwrap();

    let weak = parse(&spine(&root, "weak-points-")).unwrap();
    assert_eq!(weak.parents, vec![ABSENT.to_string()]);
}

#[test]
fn unlinking_removes_both_ends() {
    let root = boot("fur_link_unlink");

    apply(&root, Some(A), B, fur_cli::commands::link::edge_child(), false).unwrap();
    apply(&root, Some(A), B, fur_cli::commands::link::edge_child(), true).unwrap();

    assert!(parse(&spine(&root, "plan-")).unwrap().children.is_empty());
    assert!(parse(&spine(&root, "weak-points-")).unwrap().parents.is_empty());
}

#[test]
fn self_reference_is_refused() {
    let root = boot("fur_link_self");
    let err = apply(&root, Some(A), A, fur_cli::commands::link::edge_parent(), false).unwrap_err();
    assert!(err.contains("its own"), "got: {}", err);
}

#[test]
fn a_short_typo_is_refused_rather_than_dangling() {
    let root = boot("fur_link_typo");
    let err = apply(&root, Some(A), "zzz", fur_cli::commands::link::edge_child(), false).unwrap_err();
    assert!(err.contains("no conversation matches"), "got: {}", err);
}
