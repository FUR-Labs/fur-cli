//! Intentional publication-registry import bridge.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::schema::bridge::slug_for;
use crate::schema::document::{parse, FurDocument};
use crate::schema::rebuild::{import_document, rebuild, spine_paths};

const ORIGIN_FILENAME: &str = ".fur-origin.json";

#[derive(Debug, Deserialize)]
struct PullPackage {
    pull_schema: String,
    receipt: OriginReceipt,
    snapshot: Snapshot,
}

#[derive(Debug, Deserialize, Serialize)]
struct OriginReceipt {
    receipt_schema: String,
    origin_kind: String,
    registry_id: String,
    publication_id: String,
    revision_id: String,
    source_conversation_id: String,
    snapshot_digest: String,
    pulled_at: String,
}

#[derive(Debug, Deserialize)]
struct Snapshot {
    digest_algorithm: String,
    digest: String,
    spine_path: String,
    files: Vec<SnapshotFile>,
}

#[derive(Debug, Deserialize)]
struct SnapshotFile {
    path: String,
    #[allow(dead_code)]
    media_type: String,
    size: u64,
    sha256: String,
    encoding: String,
    content: String,
}

struct ValidatedPull {
    package: PullPackage,
    doc: FurDocument,
    files: BTreeMap<String, Vec<u8>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct DiaryOriginReceipt {
    receipt_schema: String,
    origin_kind: String,
    registry_id: String,
    publication_id: String,
    revision_id: String,
    snapshot_digest: String,
    pulled_at: String,
}

#[derive(Debug, Deserialize)]
struct DiaryPullPackage {
    pull_schema: String,
    receipt: DiaryOriginReceipt,
    #[allow(dead_code)]
    source: serde_json::Value,
    snapshot: DiaryPullSnapshot,
}

#[derive(Debug, Deserialize)]
struct DiaryPullSnapshot {
    digest_algorithm: String,
    digest: String,
    conversations: Vec<DiaryPullConversation>,
}

#[derive(Debug, Deserialize)]
struct DiaryPullConversation {
    folder: String,
    source: serde_json::Value,
    snapshot: Snapshot,
}

pub fn run_import(publication_id: &str, diary: bool, registry: &str) {
    if diary {
        match fetch_diary_pull_package(publication_id, registry).and_then(|package| {
            let receipt = package.receipt.clone();
            let count = install_diary_pull_package(Path::new("."), package)?;
            crate::commands::publish::save_imported_diary_cursor(
                Path::new("."),
                registry,
                &receipt.publication_id,
                &receipt.revision_id,
                &receipt.registry_id,
                &receipt.snapshot_digest,
                &receipt.pulled_at,
            )?;
            Ok(count)
        })
        {
            Ok(count) => {
                println!("✔ Imported registry diary into ./chats");
                println!("  Conversations: {}", count);
                println!("  Rebuilt:       ./.fur");
            }
            Err(error) => eprintln!("❌ Registry diary import failed: {}", error),
        }
        return;
    }
    match fetch_pull_package(publication_id, registry)
        .and_then(|package| install_pull_package(Path::new("."), package))
    {
        Ok(folder) => {
            println!("✔ Imported registry publication into {}", folder.display());
            println!("  Provenance: {}/{}", folder.display(), ORIGIN_FILENAME);
        }
        Err(error) => eprintln!("❌ Registry import failed: {}", error),
    }
}

#[allow(dead_code)]
pub fn install_diary_pull_value(
    root: &Path,
    value: serde_json::Value,
) -> Result<usize, String> {
    let package: DiaryPullPackage = serde_json::from_value(value)
        .map_err(|e| format!("invalid diary pull response: {}", e))?;
    install_diary_pull_package(root, package)
}

fn fetch_diary_pull_package(
    publication_id: &str,
    registry: &str,
) -> Result<DiaryPullPackage, String> {
    let url = format!(
        "{}/api/v2/diaries/{}/pull",
        registry.trim_end_matches('/'),
        publication_id
    );
    let response = reqwest::blocking::get(&url)
        .map_err(|e| format!("cannot reach {}: {}", url, e))?;
    if !response.status().is_success() {
        return Err(format!("registry returned HTTP {}", response.status()));
    }
    let package = response
        .json::<DiaryPullPackage>()
        .map_err(|e| format!("invalid diary pull response: {}", e))?;
    if package.receipt.publication_id != publication_id {
        return Err("registry returned a different diary publication id".to_string());
    }
    Ok(package)
}

fn install_diary_pull_package(root: &Path, package: DiaryPullPackage) -> Result<usize, String> {
    if root.join("chats").exists() || root.join(".fur").exists() {
        return Err(
            "diary import requires an empty destination without chats/ or .fur/".to_string(),
        );
    }
    if package.pull_schema != "fur.registry.diary.pull.v1"
        || package.receipt.receipt_schema != "fur.registry.diary.origin.v1"
        || package.receipt.origin_kind != "registry-diary"
    {
        return Err("invalid registry diary pull envelope".to_string());
    }
    if package.snapshot.digest_algorithm != "sha256-diary-manifest-v1" {
        return Err(format!(
            "unsupported diary digest algorithm: {}",
            package.snapshot.digest_algorithm
        ));
    }

    let expected_diary_digest = package.snapshot.digest.clone();
    let mut validated = Vec::new();
    let mut seen_folders = std::collections::BTreeSet::new();
    let mut seen_conversations = std::collections::BTreeSet::new();
    for conversation in package.snapshot.conversations {
        validate_relative_path(&conversation.folder)?;
        if conversation.folder.contains('/') || !seen_folders.insert(conversation.folder.clone()) {
            return Err(format!("invalid or duplicate diary folder: {}", conversation.folder));
        }
        let source_id = conversation.source["conversation_id"]
            .as_str()
            .ok_or("diary conversation source has no conversation_id")?
            .to_string();
        if !seen_conversations.insert(source_id.clone()) {
            return Err(format!("duplicate diary conversation id: {}", source_id));
        }
        let pull = PullPackage {
            pull_schema: "fur.registry.pull.v1".to_string(),
            receipt: OriginReceipt {
                receipt_schema: "fur.registry.origin.v1".to_string(),
                origin_kind: "registry-publication".to_string(),
                registry_id: package.receipt.registry_id.clone(),
                publication_id: package.receipt.publication_id.clone(),
                revision_id: package.receipt.revision_id.clone(),
                source_conversation_id: source_id,
                snapshot_digest: conversation.snapshot.digest.clone(),
                pulled_at: package.receipt.pulled_at.clone(),
            },
            snapshot: conversation.snapshot,
        };
        validated.push((conversation.folder, validate_pull_package(pull)?));
    }
    if validated.is_empty() {
        return Err("diary pull contains no conversations".to_string());
    }

    let digest = diary_pull_digest(&validated);
    if digest != expected_diary_digest || digest != package.receipt.snapshot_digest {
        return Err("diary snapshot digest does not match its manifest or receipt".to_string());
    }

    let staging = root.join(format!(".fur-diary-import-{}", Uuid::new_v4()));
    fs::create_dir_all(staging.join("chats"))
        .map_err(|e| format!("cannot create diary import staging: {}", e))?;
    let result = (|| {
        for (folder, conversation) in &validated {
            let target = staging.join("chats").join(folder);
            fs::create_dir_all(&target)
                .map_err(|e| format!("cannot create {}: {}", target.display(), e))?;
            write_staged_import(&target, conversation)?;
        }
        rebuild(&staging, false)?;
        let receipt_path = staging.join(".fur/registry/imported-diary.json");
        fs::create_dir_all(receipt_path.parent().unwrap())
            .map_err(|e| format!("cannot create diary receipt directory: {}", e))?;
        let receipt = serde_json::to_string_pretty(&package.receipt)
            .map_err(|e| format!("cannot serialize diary receipt: {}", e))?;
        fs::write(&receipt_path, format!("{}\n", receipt))
            .map_err(|e| format!("cannot write diary receipt: {}", e))?;
        Ok::<(), String>(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }

    fs::rename(staging.join("chats"), root.join("chats"))
        .map_err(|e| format!("cannot install diary chats/: {}", e))?;
    if let Err(error) = fs::rename(staging.join(".fur"), root.join(".fur")) {
        let _ = fs::rename(root.join("chats"), staging.join("chats"));
        let _ = fs::remove_dir_all(&staging);
        return Err(format!("cannot install rebuilt diary index: {}", error));
    }
    let _ = fs::remove_dir_all(&staging);
    Ok(validated.len())
}

fn diary_pull_digest(validated: &[(String, ValidatedPull)]) -> String {
    let mut hasher = Sha256::new();
    for (folder, conversation) in validated {
        hasher.update(folder.as_bytes());
        hasher.update([0]);
        hasher.update(conversation.doc.conversation_id.as_bytes());
        hasher.update([0]);
        hasher.update(conversation.package.snapshot.digest.as_bytes());
        hasher.update(b"\n");
    }
    format!("{:x}", hasher.finalize())
}

/// Install an already-decoded pull response. Kept public so the protocol can
/// be tested without opening a network listener.
///
/// The binary compiles the command modules separately from the library, where
/// this integration-test seam is intentionally unused.
#[allow(dead_code)]
pub fn install_pull_value(root: &Path, value: serde_json::Value) -> Result<PathBuf, String> {
    let package: PullPackage = serde_json::from_value(value)
        .map_err(|e| format!("invalid pull response: {}", e))?;
    install_pull_package(root, package)
}

fn fetch_pull_package(publication_id: &str, registry: &str) -> Result<PullPackage, String> {
    let url = format!(
        "{}/api/v2/publications/{}/pull",
        registry.trim_end_matches('/'),
        publication_id
    );
    let response = reqwest::blocking::get(&url)
        .map_err(|e| format!("cannot reach {}: {}", url, e))?;

    if !response.status().is_success() {
        return Err(format!("registry returned HTTP {}", response.status()));
    }

    let package = response
        .json::<PullPackage>()
        .map_err(|e| format!("invalid pull response: {}", e))?;

    if package.receipt.publication_id != publication_id {
        return Err("registry returned a different publication id".to_string());
    }

    Ok(package)
}

fn install_pull_package(root: &Path, package: PullPackage) -> Result<PathBuf, String> {
    let validated = validate_pull_package(package)?;
    reject_existing_conversation(root, &validated.doc.conversation_id)?;
    reject_index_collisions(root, &validated.doc)?;

    let folder_name = slug_for(&validated.doc);
    let target = root.join("chats").join(folder_name);
    if target.exists() {
        return Err(format!("{} already exists", target.display()));
    }

    let staging = root
        .join(".fur")
        .join("tmp")
        .join(format!("registry-import-{}", Uuid::new_v4()));
    fs::create_dir_all(&staging)
        .map_err(|e| format!("cannot create import staging directory: {}", e))?;

    let result = write_staged_import(&staging, &validated);
    if let Err(error) = result {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }

    fs::create_dir_all(root.join("chats"))
        .map_err(|e| format!("cannot create chats/: {}", e))?;
    fs::rename(&staging, &target)
        .map_err(|e| format!("cannot install {}: {}", target.display(), e))?;

    let spine = target.join(&validated.package.snapshot.spine_path);
    if let Err(error) = import_document(root, &target, &validated.doc) {
        return Err(format!(
            "archive installed at {}, but indexing failed: {}",
            spine.display(),
            error
        ));
    }

    Ok(target)
}

fn write_staged_import(staging: &Path, validated: &ValidatedPull) -> Result<(), String> {
    for (relative, content) in &validated.files {
        let destination = staging.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("cannot create {}: {}", parent.display(), e))?;
        }
        fs::write(&destination, content)
            .map_err(|e| format!("cannot write {}: {}", destination.display(), e))?;
    }

    let receipt = serde_json::to_string_pretty(&validated.package.receipt)
        .map_err(|e| format!("cannot serialize origin receipt: {}", e))?;
    fs::write(staging.join(ORIGIN_FILENAME), format!("{}\n", receipt))
        .map_err(|e| format!("cannot write origin receipt: {}", e))
}

fn validate_pull_package(package: PullPackage) -> Result<ValidatedPull, String> {
    if package.pull_schema != "fur.registry.pull.v1" {
        return Err(format!("unsupported pull schema: {}", package.pull_schema));
    }
    if package.receipt.receipt_schema != "fur.registry.origin.v1"
        || package.receipt.origin_kind != "registry-publication"
    {
        return Err("invalid registry origin receipt".to_string());
    }
    if package.snapshot.digest_algorithm != "sha256-manifest-v1" {
        return Err(format!(
            "unsupported snapshot digest algorithm: {}",
            package.snapshot.digest_algorithm
        ));
    }

    let mut files = BTreeMap::new();
    for file in &package.snapshot.files {
        validate_relative_path(&file.path)?;
        let bytes = decode_file(file)?;
        if bytes.len() as u64 != file.size {
            return Err(format!("incorrect size for {}", file.path));
        }
        if sha256(&bytes) != file.sha256 {
            return Err(format!("incorrect sha256 for {}", file.path));
        }
        if files.insert(file.path.clone(), bytes).is_some() {
            return Err(format!("duplicate snapshot path: {}", file.path));
        }
    }

    validate_relative_path(&package.snapshot.spine_path)?;
    let spine_bytes = files
        .get(&package.snapshot.spine_path)
        .ok_or("snapshot spine is missing")?;
    let spine = std::str::from_utf8(spine_bytes)
        .map_err(|_| "snapshot spine is not UTF-8".to_string())?;
    let doc = parse(spine).map_err(|e| format!("invalid FUR spine: {}", e))?;

    if doc.conversation_id != package.receipt.source_conversation_id {
        return Err("receipt conversation id does not match the FUR spine".to_string());
    }

    let digest = manifest_digest(&package.snapshot.files);
    if digest != package.snapshot.digest || digest != package.receipt.snapshot_digest {
        return Err("snapshot digest does not match its manifest or receipt".to_string());
    }

    validate_document_references(&doc, &files)?;

    Ok(ValidatedPull {
        package,
        doc,
        files,
    })
}

fn validate_document_references(
    doc: &FurDocument,
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<(), String> {
    for message in &doc.messages {
        if let Some(link) = &message.link {
            validate_relative_path(link)?;
            let bytes = files
                .get(link)
                .ok_or_else(|| format!("linked file is missing: {}", link))?;
            if let Some(expected) = &message.sha256 {
                if sha256(bytes) != *expected {
                    return Err(format!("linked file hash does not match: {}", link));
                }
            }
        }
        if let Some(image) = &message.img {
            validate_relative_path(image)?;
            if !files.contains_key(image) {
                return Err(format!("image file is missing: {}", image));
            }
        }
    }
    Ok(())
}

fn reject_existing_conversation(root: &Path, conversation_id: &str) -> Result<(), String> {
    for spine in spine_paths(root) {
        let Ok(content) = fs::read_to_string(&spine) else {
            continue;
        };
        let Ok(doc) = parse(&content) else {
            continue;
        };
        if doc.conversation_id == conversation_id {
            return Err(format!(
                "conversation {} already exists at {}",
                conversation_id,
                spine.display()
            ));
        }
    }
    Ok(())
}

fn reject_index_collisions(root: &Path, doc: &FurDocument) -> Result<(), String> {
    let fur_dir = root.join(".fur");
    let thread = fur_dir
        .join("threads")
        .join(format!("{}.json", doc.conversation_id));
    if thread.exists() {
        return Err(format!(
            "conversation {} is already indexed",
            doc.conversation_id
        ));
    }

    for message in &doc.messages {
        let path = fur_dir
            .join("messages")
            .join(format!("{}.json", message.id));
        if path.exists() {
            return Err(format!("message id {} is already indexed", message.id));
        }
    }

    Ok(())
}

fn validate_relative_path(raw: &str) -> Result<(), String> {
    if raw.is_empty() || raw.contains('\\') {
        return Err(format!("unsafe snapshot path: {}", raw));
    }
    let path = Path::new(raw);
    if path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(format!("unsafe snapshot path: {}", raw));
    }
    Ok(())
}

fn decode_file(file: &SnapshotFile) -> Result<Vec<u8>, String> {
    match file.encoding.as_str() {
        "utf-8" => Ok(file.content.as_bytes().to_vec()),
        "base64" => BASE64
            .decode(&file.content)
            .map_err(|e| format!("invalid base64 in {}: {}", file.path, e)),
        other => Err(format!("unsupported encoding for {}: {}", file.path, other)),
    }
}

fn manifest_digest(files: &[SnapshotFile]) -> String {
    let mut ordered: Vec<&SnapshotFile> = files.iter().collect();
    ordered.sort_by(|a, b| a.path.cmp(&b.path));

    let mut hasher = Sha256::new();
    for file in ordered {
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
