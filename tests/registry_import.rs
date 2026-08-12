use std::fs;
use std::path::Path;

use serde_json::json;
use sha2::{Digest, Sha256};
use tempfile::tempdir;

fn hash(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn package() -> serde_json::Value {
    let attachment = b"# Analysis\n\nBody.\n";
    let attachment_hash = hash(attachment);
    let spine = format!(
        "---\nfur_schema: 1\nconversation_id: 11111111-1111-4111-8111-111111111111\ntitle: Imported Research\ncreated_at: 2026-08-09T20:00:00Z\ntags: []\n---\n\n<!-- fur:msg id=22222222-2222-4222-8222-222222222222 avatar=andrew ts=2026-08-09T20:01:00Z -->\n\nLocal question.\n\n<!-- fur:msg id=33333333-3333-4333-8333-333333333333 avatar=gpt5 ts=2026-08-09T20:02:00Z link=analysis.md sha256={} -->\n",
        attachment_hash
    );
    let spine_hash = hash(spine.as_bytes());

    let mut manifest = Sha256::new();
    for (path, size, digest) in [
        ("analysis.md", attachment.len(), attachment_hash.as_str()),
        ("convo.md", spine.len(), spine_hash.as_str()),
    ] {
        manifest.update(path.as_bytes());
        manifest.update([0]);
        manifest.update(size.to_string().as_bytes());
        manifest.update([0]);
        manifest.update(digest.as_bytes());
        manifest.update(b"\n");
    }
    let snapshot_digest = format!("{:x}", manifest.finalize());

    json!({
        "pull_schema": "fur.registry.pull.v1",
        "receipt": {
            "receipt_schema": "fur.registry.origin.v1",
            "origin_kind": "registry-publication",
            "registry_id": "registry.test",
            "publication_id": "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa",
            "revision_id": "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
            "source_conversation_id": "11111111-1111-4111-8111-111111111111",
            "snapshot_digest": snapshot_digest,
            "pulled_at": "2026-08-09T20:15:00Z"
        },
        "snapshot": {
            "digest_algorithm": "sha256-manifest-v1",
            "digest": snapshot_digest,
            "spine_path": "convo.md",
            "files": [
                {
                    "path": "analysis.md",
                    "media_type": "text/markdown",
                    "size": attachment.len(),
                    "sha256": attachment_hash,
                    "encoding": "utf-8",
                    "content": String::from_utf8_lossy(attachment)
                },
                {
                    "path": "convo.md",
                    "media_type": "text/markdown",
                    "size": spine.len(),
                    "sha256": spine_hash,
                    "encoding": "utf-8",
                    "content": spine
                }
            ]
        }
    })
}

#[test]
fn imported_snapshot_can_be_rebuilt_by_fur() {
    let tmp = tempdir().unwrap();
    let value = package();

    let folder = fur_cli::commands::registry::install_pull_value(tmp.path(), value).unwrap();

    assert!(folder.join("convo.md").exists());
    assert!(folder.join("analysis.md").exists());
    assert!(folder.join(".fur-origin.json").exists());
    assert!(tmp
        .path()
        .join(".fur/threads/11111111-1111-4111-8111-111111111111.json")
        .exists());

    let index: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(tmp.path().join(".fur/index.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        index["active_thread"],
        "11111111-1111-4111-8111-111111111111"
    );
}

#[test]
fn tampered_snapshot_is_rejected_before_any_archive_is_written() {
    let tmp = tempdir().unwrap();
    let mut value = package();
    value["snapshot"]["files"][0]["content"] = json!("tampered");

    let error = fur_cli::commands::registry::install_pull_value(tmp.path(), value).unwrap_err();

    assert!(error.contains("incorrect size"));
    assert!(!Path::new(tmp.path()).join("chats").exists());
}

fn diary_package() -> serde_json::Value {
    let conversation = package();
    let folder = "imported-research-11111111";
    let conversation_id = conversation["receipt"]["source_conversation_id"]
        .as_str()
        .unwrap();
    let conversation_digest = conversation["snapshot"]["digest"].as_str().unwrap();
    let mut manifest = Sha256::new();
    manifest.update(folder.as_bytes());
    manifest.update([0]);
    manifest.update(conversation_id.as_bytes());
    manifest.update([0]);
    manifest.update(conversation_digest.as_bytes());
    manifest.update(b"\n");
    let diary_digest = format!("{:x}", manifest.finalize());

    json!({
        "pull_schema": "fur.registry.diary.pull.v1",
        "receipt": {
            "receipt_schema": "fur.registry.diary.origin.v1",
            "origin_kind": "registry-diary",
            "registry_id": "registry.test",
            "publication_id": "dddddddd-dddd-4ddd-8ddd-dddddddddddd",
            "revision_id": "eeeeeeee-eeee-4eee-8eee-eeeeeeeeeeee",
            "snapshot_digest": diary_digest,
            "pulled_at": "2026-08-12T03:00:00Z"
        },
        "source": {
            "format": "fur.project-directory",
            "suggested_name": "research"
        },
        "snapshot": {
            "digest_algorithm": "sha256-diary-manifest-v1",
            "digest": diary_digest,
            "conversations": [{
                "folder": folder,
                "source": {
                    "format": "fur.conversation-directory",
                    "fur_document_schema": 1,
                    "conversation_id": conversation_id,
                    "origin_kind": "local"
                },
                "snapshot": conversation["snapshot"].clone()
            }]
        }
    })
}

#[test]
fn complete_diary_import_is_rebuilt_without_canonical_diary_metadata() {
    let tmp = tempdir().unwrap();

    let count = fur_cli::commands::registry::install_diary_pull_value(
        tmp.path(),
        diary_package(),
    )
    .unwrap();

    assert_eq!(count, 1);
    assert!(tmp.path().join("chats/imported-research-11111111/convo.md").exists());
    assert!(tmp.path().join(".fur/index.json").exists());
    assert!(tmp.path().join(".fur/registry/imported-diary.json").exists());
    assert!(!tmp.path().join("chats/.fur-diary.json").exists());
}

#[test]
fn tampered_diary_is_rejected_before_any_project_is_installed() {
    let tmp = tempdir().unwrap();
    let mut value = diary_package();
    value["snapshot"]["conversations"][0]["snapshot"]["files"][0]["content"] =
        json!("tampered");

    let error = fur_cli::commands::registry::install_diary_pull_value(tmp.path(), value)
        .unwrap_err();

    assert!(error.contains("incorrect size"));
    assert!(!tmp.path().join("chats").exists());
    assert!(!tmp.path().join(".fur").exists());
}
