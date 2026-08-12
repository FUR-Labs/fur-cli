//! Publish canonical FUR conversations and project diaries to a registry.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::schema::diary::{
    build_diary_publish_intent, remove_legacy_diary_metadata, DiaryPublishIntent,
};
use crate::schema::snapshot::{build_publish_intent, PublishIntent};

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PublicationCreated {
    publication_id: String,
    revision_id: String,
    registry_id: String,
    snapshot_digest: String,
    published_at: String,
    publication_state: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct DiaryRegistryCursor {
    registry_url: String,
    publication_id: String,
    revision_id: String,
    registry_id: String,
    snapshot_digest: String,
    published_at: String,
}

pub fn run_publish(conversation: Option<&str>, diary: Option<&str>, registry: &str) {
    if let Some(selector) = diary {
        run_diary_publish(selector, registry);
        return;
    }
    match build_publish_intent(Path::new("."), conversation)
        .and_then(|intent| submit_publish_intent(registry, &intent))
    {
        Ok(created) => print_receipt("Conversation", &created),
        Err(error) => eprintln!("❌ Registry publication failed: {}", error),
    }
}

fn run_diary_publish(selector: &str, registry: &str) {
    let root = match resolve_diary_root(Path::new("."), selector) {
        Ok(root) => root,
        Err(error) => {
            eprintln!("❌ Registry diary publication failed: {}", error);
            return;
        }
    };

    let result = remove_legacy_diary_metadata(&root)
        .and_then(|removed| {
            if removed {
                println!("  Removed obsolete chats/.fur-diary.json metadata");
            }
            build_diary_publish_intent(&root)
        })
        .and_then(|mut intent| {
            if let Some(cursor) = load_diary_cursor(&root, registry)? {
                intent.source["registry_cursor"] = json!({
                    "registry_id": cursor.registry_id,
                    "publication_id": cursor.publication_id,
                    "revision_id": cursor.revision_id
                });
            }
            let created = submit_diary_publish_intent(registry, &intent)?;
            save_diary_cursor(&root, registry, &created)?;
            Ok(created)
        });

    match result {
        Ok(created) => print_receipt("Diary", &created),
        Err(error) => eprintln!("❌ Registry diary publication failed: {}", error),
    }
}

fn resolve_diary_root(start: &Path, selector: &str) -> Result<PathBuf, String> {
    let selected = if selector.trim().is_empty() || selector == "." {
        start.to_path_buf()
    } else {
        let candidate = PathBuf::from(selector);
        if candidate.is_absolute() {
            candidate
        } else {
            start.join(candidate)
        }
    };
    if !selected.join("chats").is_dir() {
        return Err(format!(
            "{} is not a FUR diary (missing chats/)",
            selected.display()
        ));
    }
    Ok(selected)
}

fn cursor_path(root: &Path, registry: &str) -> PathBuf {
    let mut hasher = Sha256::new();
    hasher.update(registry.trim_end_matches('/').as_bytes());
    let key = format!("{:x}", hasher.finalize());
    root.join(".fur")
        .join("registry")
        .join("diaries")
        .join(format!("{}.json", &key[..16]))
}

fn load_diary_cursor(
    root: &Path,
    registry: &str,
) -> Result<Option<DiaryRegistryCursor>, String> {
    let path = cursor_path(root, registry);
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
    let cursor: DiaryRegistryCursor = serde_json::from_str(&text)
        .map_err(|e| format!("invalid {}: {}", path.display(), e))?;
    if cursor.registry_url != registry.trim_end_matches('/') {
        return Err(format!("registry cursor mismatch in {}", path.display()));
    }
    Ok(Some(cursor))
}

fn save_diary_cursor(
    root: &Path,
    registry: &str,
    created: &PublicationCreated,
) -> Result<(), String> {
    let path = cursor_path(root, registry);
    let parent = path.parent().ok_or("registry cursor has no parent directory")?;
    fs::create_dir_all(parent)
        .map_err(|e| format!("cannot create {}: {}", parent.display(), e))?;
    let cursor = DiaryRegistryCursor {
        registry_url: registry.trim_end_matches('/').to_string(),
        publication_id: created.publication_id.clone(),
        revision_id: created.revision_id.clone(),
        registry_id: created.registry_id.clone(),
        snapshot_digest: created.snapshot_digest.clone(),
        published_at: created.published_at.clone(),
    };
    let text = serde_json::to_string_pretty(&cursor)
        .map_err(|e| format!("cannot serialize diary registry cursor: {}", e))?;
    fs::write(&path, format!("{}\n", text))
        .map_err(|e| format!("cannot write {}: {}", path.display(), e))
}

pub(crate) fn save_imported_diary_cursor(
    root: &Path,
    registry: &str,
    publication_id: &str,
    revision_id: &str,
    registry_id: &str,
    snapshot_digest: &str,
    published_at: &str,
) -> Result<(), String> {
    save_diary_cursor(
        root,
        registry,
        &PublicationCreated {
            publication_id: publication_id.to_string(),
            revision_id: revision_id.to_string(),
            registry_id: registry_id.to_string(),
            snapshot_digest: snapshot_digest.to_string(),
            published_at: published_at.to_string(),
            publication_state: "imported".to_string(),
        },
    )
}

fn print_receipt(kind: &str, created: &PublicationCreated) {
    match (kind, created.publication_state.as_str()) {
        ("Diary", "unchanged") => println!("✔ Diary already published; no changes"),
        ("Diary", "revised") => println!("✔ Published new diary revision"),
        ("Diary", _) => println!("✔ Published diary"),
        (_, "unchanged") => println!("✔ Conversation already published; no changes"),
        (_, "revised") => println!("✔ Published new conversation revision"),
        _ => println!("✔ Published conversation"),
    }
    println!("  Publication: {}", created.publication_id);
    println!("  Revision:    {}", created.revision_id);
    println!("  Registry:    {}", created.registry_id);
    println!("  Snapshot:    {}", created.snapshot_digest);
    println!("  Published:   {}", created.published_at);
}

fn submit_publish_intent(
    registry: &str,
    intent: &PublishIntent,
) -> Result<PublicationCreated, String> {
    submit_intent(registry, "/api/v2/publish", intent)
}

fn submit_diary_publish_intent(
    registry: &str,
    intent: &DiaryPublishIntent,
) -> Result<PublicationCreated, String> {
    submit_intent(registry, "/api/v2/diaries/publish", intent)
}

fn submit_intent<T: Serialize>(
    registry: &str,
    endpoint: &str,
    intent: &T,
) -> Result<PublicationCreated, String> {
    let url = format!("{}{}", registry.trim_end_matches('/'), endpoint);
    let response = reqwest::blocking::Client::new()
        .post(&url)
        .json(intent)
        .send()
        .map_err(|e| format!("cannot reach {}: {}", url, e))?;
    if !response.status().is_success() {
        let status = response.status();
        let detail = response.text().unwrap_or_default();
        return Err(format!("registry returned HTTP {}: {}", status, detail));
    }
    response
        .json::<PublicationCreated>()
        .map_err(|e| format!("invalid publication receipt: {}", e))
}
