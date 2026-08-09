use std::fs;
use std::path::Path;

use serde_json::json;
use tempfile::tempdir;

use fur_cli::schema::snapshot::build_publish_intent;

const FIRST_ID: &str = "11111111-1111-4111-8111-111111111111";
const SECOND_ID: &str = "11112222-2222-4222-8222-222222222222";

fn write_conversation(root: &Path, folder: &str, id: &str, title: &str) {
    let directory = root.join("chats").join(folder);
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("convo.md"),
        format!(
            "---\nfur_schema: 1\nconversation_id: {}\ntitle: {}\ncreated_at: 2026-08-09T20:00:00Z\ntags:\n  - research\n---\n\n<!-- fur:msg id=33333333-3333-4333-8333-333333333333 avatar=andrew ts=2026-08-09T20:01:00Z link=analysis.md sha256=91535c9ad75baf89e1dc5c1b5d16eabefb18af926253e9ac551b2233a84ca5d5 -->\n",
            id, title
        ),
    )
    .unwrap();
    fs::write(directory.join("analysis.md"), "# Analysis\n\nBody.\n").unwrap();
}

#[test]
fn publish_without_selector_uses_active_conversation() {
    let tmp = tempdir().unwrap();
    write_conversation(tmp.path(), "first-11111111", FIRST_ID, "First");
    write_conversation(tmp.path(), "second-11112222", SECOND_ID, "Second");
    fs::create_dir_all(tmp.path().join(".fur")).unwrap();
    fs::write(
        tmp.path().join(".fur/index.json"),
        json!({"active_thread": SECOND_ID}).to_string(),
    )
    .unwrap();

    let intent = build_publish_intent(tmp.path(), None).unwrap();
    let value = serde_json::to_value(intent).unwrap();

    assert_eq!(value["source"]["conversation_id"], SECOND_ID);
    assert_eq!(value["source"]["origin_kind"], "local");
    assert_eq!(value["snapshot"]["spine_path"], "convo.md");
}

#[test]
fn unique_short_hash_selects_a_non_active_conversation() {
    let tmp = tempdir().unwrap();
    write_conversation(tmp.path(), "first-11111111", FIRST_ID, "First");
    write_conversation(tmp.path(), "second-11112222", SECOND_ID, "Second");

    let intent = build_publish_intent(tmp.path(), Some("11112222")).unwrap();
    let value = serde_json::to_value(intent).unwrap();

    assert_eq!(value["source"]["conversation_id"], SECOND_ID);
}

#[test]
fn snapshot_is_deterministic_and_carries_import_lineage_outside_its_files() {
    let tmp = tempdir().unwrap();
    write_conversation(tmp.path(), "first-11111111", FIRST_ID, "First");
    let folder = tmp.path().join("chats/first-11111111");
    fs::write(
        folder.join(".fur-origin.json"),
        json!({
            "receipt_schema": "fur.registry.origin.v1",
            "origin_kind": "registry-publication",
            "registry_id": "registry.test",
            "publication_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
            "revision_id": "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
            "source_conversation_id": FIRST_ID,
            "snapshot_digest": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            "pulled_at": "2026-08-09T20:15:00Z"
        })
        .to_string(),
    )
    .unwrap();

    let first = serde_json::to_value(
        build_publish_intent(tmp.path(), Some("11111111")).unwrap(),
    )
    .unwrap();
    let second = serde_json::to_value(
        build_publish_intent(tmp.path(), Some("11111111")).unwrap(),
    )
    .unwrap();

    assert_eq!(first["snapshot"], second["snapshot"]);
    assert_eq!(first["source"]["origin_kind"], "registry-import");
    assert_eq!(
        first["source"]["import_origin"]["publication_id"],
        "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"
    );
    assert!(first["snapshot"]["files"]
        .as_array()
        .unwrap()
        .iter()
        .all(|file| file["path"] != ".fur-origin.json"));
}

#[test]
fn ambiguous_short_hash_is_rejected() {
    let tmp = tempdir().unwrap();
    write_conversation(tmp.path(), "first-11111111", FIRST_ID, "First");
    write_conversation(tmp.path(), "second-11112222", SECOND_ID, "Second");

    let error = build_publish_intent(tmp.path(), Some("1111")).unwrap_err();

    assert!(error.contains("ambiguous"));
}
