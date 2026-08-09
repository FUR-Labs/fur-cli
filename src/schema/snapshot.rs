//! Registry-neutral snapshots of canonical FUR conversation directories.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::schema::document::{parse, FurDocument};
use crate::schema::rebuild::spine_paths;

const ORIGIN_FILENAME: &str = ".fur-origin.json";

#[derive(Debug, Clone, Serialize)]
pub struct PublishIntent {
    pub publish_schema: &'static str,
    pub source: Value,
    pub snapshot: Snapshot,
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub digest_algorithm: &'static str,
    pub digest: String,
    pub spine_path: String,
    pub files: Vec<SnapshotFile>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotFile {
    pub path: String,
    pub media_type: String,
    pub size: u64,
    pub sha256: String,
    pub encoding: &'static str,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct OriginReceipt {
    registry_id: String,
    publication_id: String,
    revision_id: String,
    source_conversation_id: String,
    snapshot_digest: String,
}

pub fn build_publish_intent(root: &Path, selector: Option<&str>) -> Result<PublishIntent, String> {
    let (folder, spine, doc) = resolve_conversation(root, selector)?;
    let mut files = collect_snapshot_files(&folder)?;
    files.sort_by(|left, right| left.path.cmp(&right.path));

    let spine_path = relative_path(&folder, &spine)?;
    validate_document_files(&doc, &files)?;
    let digest = manifest_digest(&files);
    let source = source_metadata(&folder, &doc)?;

    Ok(PublishIntent {
        publish_schema: "fur.registry.publish.v1",
        source,
        snapshot: Snapshot {
            digest_algorithm: "sha256-manifest-v1",
            digest,
            spine_path,
            files,
        },
    })
}

fn validate_document_files(doc: &FurDocument, files: &[SnapshotFile]) -> Result<(), String> {
    let hashes: BTreeMap<&str, &str> = files
        .iter()
        .map(|file| (file.path.as_str(), file.sha256.as_str()))
        .collect();

    for message in &doc.messages {
        for reference in [&message.link, &message.img].into_iter().flatten() {
            if !hashes.contains_key(reference.as_str()) {
                return Err(format!("conversation references missing file: {}", reference));
            }
        }
        if let (Some(link), Some(expected)) = (&message.link, &message.sha256) {
            if hashes.get(link.as_str()).copied() != Some(expected.as_str()) {
                return Err(format!("linked file hash does not match convo.md: {}", link));
            }
        }
    }
    Ok(())
}

fn resolve_conversation(
    root: &Path,
    selector: Option<&str>,
) -> Result<(PathBuf, PathBuf, FurDocument), String> {
    let selected = match selector {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => active_conversation_id(root)?,
    };

    let mut matches = Vec::new();
    for spine in spine_paths(root) {
        let text = fs::read_to_string(&spine)
            .map_err(|e| format!("cannot read {}: {}", spine.display(), e))?;
        let doc = parse(&text).map_err(|e| format!("invalid {}: {}", spine.display(), e))?;
        if doc.conversation_id == selected || doc.conversation_id.starts_with(&selected) {
            let folder = spine
                .parent()
                .ok_or_else(|| format!("{} has no conversation folder", spine.display()))?
                .to_path_buf();
            matches.push((folder, spine, doc));
        }
    }

    match matches.len() {
        0 => Err(format!("no conversation matches '{}'", selected)),
        1 => Ok(matches.remove(0)),
        _ => Err(format!("conversation prefix '{}' is ambiguous", selected)),
    }
}

fn active_conversation_id(root: &Path) -> Result<String, String> {
    let path = root.join(".fur").join("index.json");
    let text = fs::read_to_string(&path)
        .map_err(|_| "no active conversation; pass a conversation hash".to_string())?;
    let index: Value = serde_json::from_str(&text)
        .map_err(|e| format!("cannot parse {}: {}", path.display(), e))?;
    index["active_thread"]
        .as_str()
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "no active conversation; pass a conversation hash".to_string())
}

fn collect_snapshot_files(folder: &Path) -> Result<Vec<SnapshotFile>, String> {
    let mut paths = Vec::new();
    collect_paths(folder, &mut paths)?;

    let mut files = Vec::new();
    for path in paths {
        let relative = relative_path(folder, &path)?;
        if relative == ORIGIN_FILENAME {
            continue;
        }
        let bytes = fs::read(&path)
            .map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
        let (encoding, content) = match String::from_utf8(bytes.clone()) {
            Ok(text) => ("utf-8", text),
            Err(_) => ("base64", BASE64.encode(&bytes)),
        };
        files.push(SnapshotFile {
            path: relative.clone(),
            media_type: media_type(&relative).to_string(),
            size: bytes.len() as u64,
            sha256: sha256(&bytes),
            encoding,
            content,
        });
    }
    Ok(files)
}

fn collect_paths(directory: &Path, paths: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(directory)
        .map_err(|e| format!("cannot read {}: {}", directory.display(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("cannot read directory entry: {}", e))?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|e| format!("cannot inspect {}: {}", path.display(), e))?;
        if metadata.file_type().is_symlink() {
            return Err(format!("refusing to publish symlink: {}", path.display()));
        }
        if metadata.is_dir() {
            collect_paths(&path, paths)?;
        } else if metadata.is_file() {
            paths.push(path);
        }
    }
    Ok(())
}

fn source_metadata(folder: &Path, doc: &FurDocument) -> Result<Value, String> {
    let origin_path = folder.join(ORIGIN_FILENAME);
    let mut source = json!({
        "format": "fur.conversation-directory",
        "fur_document_schema": doc.schema,
        "conversation_id": doc.conversation_id,
        "origin_kind": "local",
        "producer": {"fur_version": env!("CARGO_PKG_VERSION")}
    });

    if origin_path.exists() {
        let text = fs::read_to_string(&origin_path)
            .map_err(|e| format!("cannot read {}: {}", origin_path.display(), e))?;
        let receipt: OriginReceipt = serde_json::from_str(&text)
            .map_err(|e| format!("invalid {}: {}", origin_path.display(), e))?;
        if receipt.source_conversation_id != doc.conversation_id {
            return Err("origin receipt does not match this conversation".to_string());
        }
        source["origin_kind"] = json!("registry-import");
        source["import_origin"] = json!({
            "registry_id": receipt.registry_id,
            "publication_id": receipt.publication_id,
            "revision_id": receipt.revision_id,
            "source_conversation_id": receipt.source_conversation_id,
            "snapshot_digest": receipt.snapshot_digest
        });
    }
    Ok(source)
}

fn relative_path(root: &Path, path: &Path) -> Result<String, String> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| format!("{} is outside {}", path.display(), root.display()))?;
    if relative.components().any(|part| !matches!(part, Component::Normal(_))) {
        return Err(format!("unsafe snapshot path: {}", relative.display()));
    }
    let parts = relative
        .components()
        .map(|part| {
            part.as_os_str()
                .to_str()
                .ok_or_else(|| format!("snapshot path is not UTF-8: {}", relative.display()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let normalized = parts.join("/");
    if normalized.contains('\\') {
        return Err(format!("unsafe snapshot path: {}", relative.display()));
    }
    Ok(normalized)
}

fn manifest_digest(files: &[SnapshotFile]) -> String {
    let mut hasher = Sha256::new();
    for file in files {
        hasher.update(file.path.as_bytes());
        hasher.update([0]);
        hasher.update(file.size.to_string().as_bytes());
        hasher.update([0]);
        hasher.update(file.sha256.as_bytes());
        hasher.update(b"\n");
    }
    format!("{:x}", hasher.finalize())
}

fn sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn media_type(path: &str) -> &'static str {
    match Path::new(path).extension().and_then(|value| value.to_str()) {
        Some("md") => "text/markdown",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        Some("pdf") => "application/pdf",
        _ => "application/octet-stream",
    }
}
