use std::fs;
use std::path::Path;

use serde_json::json;
use tempfile::tempdir;

use fur_cli::schema::diary::{build_diary_publish_intent, remove_legacy_diary_metadata};

const FIRST_ID: &str = "11111111-1111-4111-8111-111111111111";
const SECOND_ID: &str = "22222222-2222-4222-8222-222222222222";

fn write_conversation(root: &Path, folder: &str, id: &str, title: &str) {
    let directory = root.join("chats").join(folder);
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("convo.md"),
        format!(
            "---\nfur_schema: 1\nconversation_id: {}\ntitle: {}\ncreated_at: 2026-08-09T20:00:00Z\ntags: []\n---\n\n<!-- fur:msg id={}-m1 avatar=andrew ts=2026-08-09T20:01:00Z -->\n\nBody.\n",
            id, title, &id[..8]
        ),
    )
    .unwrap();
}

#[test]
fn diary_is_inferred_without_writing_metadata_into_chats() {
    let tmp = tempdir().unwrap();
    write_conversation(tmp.path(), "zeta-22222222", SECOND_ID, "Zeta");
    write_conversation(tmp.path(), "alpha-11111111", FIRST_ID, "Alpha");

    let first = serde_json::to_value(build_diary_publish_intent(tmp.path()).unwrap()).unwrap();
    let second = serde_json::to_value(build_diary_publish_intent(tmp.path()).unwrap()).unwrap();

    assert_eq!(first, second);
    assert_eq!(first["publish_schema"], "fur.registry.diary.publish.v1");
    assert_eq!(first["source"]["format"], "fur.project-directory");
    assert_eq!(
        first["source"]["suggested_name"],
        tmp.path().file_name().unwrap().to_string_lossy().as_ref()
    );
    assert!(first["source"].get("diary_id").is_none());
    assert!(first["snapshot"].get("files").is_none());
    assert_eq!(first["snapshot"]["conversations"][0]["folder"], "alpha-11111111");
    assert_eq!(first["snapshot"]["conversations"][1]["folder"], "zeta-22222222");
    assert!(!tmp.path().join("chats/.fur-diary.json").exists());
}

#[test]
fn adding_a_conversation_changes_the_collection_digest() {
    let tmp = tempdir().unwrap();
    write_conversation(tmp.path(), "alpha-11111111", FIRST_ID, "Alpha");
    let first = serde_json::to_value(build_diary_publish_intent(tmp.path()).unwrap()).unwrap();

    write_conversation(tmp.path(), "zeta-22222222", SECOND_ID, "Zeta");
    let second = serde_json::to_value(build_diary_publish_intent(tmp.path()).unwrap()).unwrap();

    assert_eq!(first["source"], second["source"]);
    assert_ne!(first["snapshot"]["digest"], second["snapshot"]["digest"]);
}

#[test]
fn imported_conversation_keeps_lineage_outside_canonical_files() {
    let tmp = tempdir().unwrap();
    write_conversation(tmp.path(), "alpha-11111111", FIRST_ID, "Alpha");
    let folder = tmp.path().join("chats/alpha-11111111");
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

    let value = serde_json::to_value(build_diary_publish_intent(tmp.path()).unwrap()).unwrap();
    let conversation = &value["snapshot"]["conversations"][0];
    assert_eq!(conversation["source"]["origin_kind"], "registry-import");
    assert!(conversation["snapshot"]["files"]
        .as_array()
        .unwrap()
        .iter()
        .all(|file| file["path"] != ".fur-origin.json"));
}

#[test]
fn malformed_conversation_directory_is_not_silently_omitted() {
    let tmp = tempdir().unwrap();
    write_conversation(tmp.path(), "alpha-11111111", FIRST_ID, "Alpha");
    fs::create_dir_all(tmp.path().join("chats/broken-deadbeef")).unwrap();
    fs::write(tmp.path().join("chats/broken-deadbeef/note.md"), "not a spine").unwrap();

    let error = build_diary_publish_intent(tmp.path()).unwrap_err();
    assert!(error.contains("exactly one canonical conversation"));
}

#[test]
fn removes_only_recognized_legacy_diary_metadata() {
    let tmp = tempdir().unwrap();
    fs::create_dir_all(tmp.path().join("chats")).unwrap();
    let legacy = tmp.path().join("chats/.fur-diary.json");
    fs::write(
        &legacy,
        json!({
            "diary_schema": 1,
            "diary_id": "11111111-1111-4111-8111-111111111111",
            "title": "Old Diary",
            "created_at": "2026-08-10T00:00:00Z"
        })
        .to_string(),
    )
    .unwrap();

    assert!(remove_legacy_diary_metadata(tmp.path()).unwrap());
    assert!(!legacy.exists());

    fs::write(&legacy, r#"{"personal":"keep me"}"#).unwrap();
    assert!(remove_legacy_diary_metadata(tmp.path()).is_err());
    assert!(legacy.exists());
}
