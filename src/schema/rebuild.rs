//! Reconstruct `.fur/` from `chats/`.
//!
//! This is the test of whether the architecture actually holds:
//!
//! ```text
//! rm -rf .fur && fur convo
//! ```
//!
//! If the archive comes back, `chats/` has genuinely become the durable format
//! and `.fur/` is disposable operational state.
//!
//! Rebuild is deliberately conservative. It runs when `.fur/` is *absent*. It
//! refuses to run over an existing `.fur/` without `--force`, because a stale
//! or partially-synced `chats/` would otherwise destroy live local state
//! (active conversation, cursor position, avatar emoji) on an ordinary command.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::schema::document::{parse, FurDocument};
use crate::schema::SCHEMA_VERSION;

/// Written by `fur lock`, removed by `fur unlock`, never itself encrypted.
///
/// It lives in `chats/` rather than `.fur/` on purpose: once `.fur/` is
/// disposable, a lock flag stored there could be deleted while the ciphertext
/// remained, leaving rebuild to parse encrypted bytes as front matter.
pub const LOCK_SENTINEL: &str = ".fur-locked";

/// Encrypted password verifier, kept beside the data rather than inside the
/// disposable index.
pub const LOCK_CHECK: &str = ".fur-lockcheck";

/// What a directory looks like before any command touches it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectState {
    /// Neither `.fur/` nor any conversation document.
    Empty,
    /// `.fur/` present and usable.
    Indexed,
    /// Conversation documents exist but `.fur/` does not — a copied archive.
    Unindexed,
    /// `chats/` is encrypted; nothing may be parsed until unlocked.
    Locked,
}

#[derive(Debug, Default)]
pub struct RebuildSummary {
    pub conversations: usize,
    pub messages: usize,
    pub avatars: Vec<String>,
    pub guessed_main: Option<String>,
    pub skipped: Vec<String>,
}

pub fn is_locked_archive(root: &Path) -> bool {
    root.join("chats").join(LOCK_SENTINEL).exists()
}

pub fn detect_state(root: &Path) -> ProjectState {
    if is_locked_archive(root) {
        return ProjectState::Locked;
    }

    if root.join(".fur").join("index.json").exists() {
        return ProjectState::Indexed;
    }

    if spine_paths(root).is_empty() {
        ProjectState::Empty
    } else {
        ProjectState::Unindexed
    }
}

/// Every `chats/<slug>/convo.md` in the project.
///
/// Spines are found by *content*, not by filename: any `.md` directly inside a
/// conversation folder that parses as a document counts. Filenames stay
/// cosmetic so a user can rename freely without breaking rebuild.
pub fn spine_paths(root: &Path) -> Vec<PathBuf> {
    let chats = root.join("chats");

    let Ok(entries) = fs::read_dir(&chats) else {
        return Vec::new();
    };

    let mut out = Vec::new();

    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }

        let Ok(files) = fs::read_dir(&dir) else {
            continue;
        };

        let mut found: Vec<PathBuf> = files
            .flatten()
            .map(|f| f.path())
            .filter(|p| p.is_file())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("md"))
            .filter(|p| looks_like_spine(p))
            .collect();

        found.sort();
        out.append(&mut found);
    }

    out.sort();
    out
}

/// Cheap check: front matter opening plus a conversation id, without paying to
/// parse every long-form document in the folder.
fn looks_like_spine(path: &Path) -> bool {
    let Ok(content) = fs::read_to_string(path) else {
        return false;
    };

    content.starts_with("---\n") && content.contains("\nconversation_id:")
}

/// Rebuild `.fur/` from every conversation document under `chats/`.
pub fn rebuild(root: &Path, force: bool) -> Result<RebuildSummary, String> {
    if is_locked_archive(root) {
        return Err("🔒 Archive is locked. Run `fur unlock` first.".to_string());
    }

    let fur_dir = root.join(".fur");

    if fur_dir.join("index.json").exists() && !force {
        return Err(
            ".fur/ already exists — refusing to overwrite live state (use --force)".to_string(),
        );
    }

    let spines = spine_paths(root);
    if spines.is_empty() {
        return Err("no conversation documents found under chats/".to_string());
    }

    fs::create_dir_all(fur_dir.join("threads"))
        .map_err(|e| format!("cannot create .fur/threads: {}", e))?;
    fs::create_dir_all(fur_dir.join("messages"))
        .map_err(|e| format!("cannot create .fur/messages: {}", e))?;

    let mut summary = RebuildSummary::default();
    let mut thread_ids: Vec<String> = Vec::new();
    let mut avatar_counts: BTreeMap<String, usize> = BTreeMap::new();

    for spine in &spines {
        let text = match fs::read_to_string(spine) {
            Ok(t) => t,
            Err(e) => {
                summary
                    .skipped
                    .push(format!("{} ({})", spine.display(), e));
                continue;
            }
        };

        let doc = match parse(&text) {
            Ok(d) => d,
            Err(e) => {
                // One malformed document costs that document, not the archive.
                summary
                    .skipped
                    .push(format!("{} ({})", spine.display(), e));
                continue;
            }
        };

        let folder = spine.parent().unwrap_or(root);

        for msg in &doc.messages {
            *avatar_counts.entry(msg.avatar.clone()).or_insert(0) += 1;
            write_message(&fur_dir, root, folder, msg)?;
            summary.messages += 1;
        }

        write_thread(&fur_dir, &doc)?;
        thread_ids.push(doc.conversation_id.clone());
        summary.conversations += 1;
    }

    if thread_ids.is_empty() {
        return Err("no conversation documents could be parsed".to_string());
    }

    write_index(&fur_dir, &thread_ids)?;

    summary.avatars = avatar_counts.keys().cloned().collect();
    summary.guessed_main = avatar_counts
        .iter()
        .max_by_key(|(_, count)| **count)
        .map(|(name, _)| name.clone());

    write_avatars(&fur_dir, &summary)?;

    Ok(summary)
}

fn write_message(
    fur_dir: &Path,
    root: &Path,
    folder: &Path,
    msg: &crate::schema::document::FurMessage,
) -> Result<(), String> {
    // Links are stored as basenames relative to the conversation folder;
    // `.fur/` wants a path relative to the project root.
    let markdown = msg
        .link
        .as_ref()
        .map(|name| relative_to_root(root, &folder.join(name)));

    let markdown_meta = match (&markdown, &msg.sha256) {
        (Some(path), hash) => {
            let size = fs::metadata(root.join(path)).map(|m| m.len()).ok();
            json!({
                "hash": hash,
                "size": size,
                "filename": Path::new(path).file_name().and_then(|f| f.to_str())
            })
        }
        _ => Value::Null,
    };

    let attachment = msg
        .img
        .as_ref()
        .map(|name| relative_to_root(root, &folder.join(name)));

    let value = json!({
        "id": msg.id,
        "avatar": msg.avatar,
        "timestamp": msg.ts,
        "text": if msg.body.is_empty() { Value::Null } else { json!(msg.body) },
        "markdown": markdown,
        "markdown_meta": markdown_meta,
        "attachment": attachment,
        "parent": Value::Null,
        "children": [],
        "branches": [],
        "schema_version": SCHEMA_VERSION
    });

    let path = fur_dir.join("messages").join(format!("{}.json", msg.id));

    fs::write(&path, pretty(&value)).map_err(|e| format!("cannot write {}: {}", path.display(), e))
}

fn write_thread(fur_dir: &Path, doc: &FurDocument) -> Result<(), String> {
    let value = json!({
        "id": doc.conversation_id,
        "created_at": doc.created_at,
        "messages": doc.messages.iter().map(|m| m.id.clone()).collect::<Vec<_>>(),
        "tags": doc.tags,
        "title": doc.title,
        "schema_version": SCHEMA_VERSION
    });

    let path = fur_dir
        .join("threads")
        .join(format!("{}.json", doc.conversation_id));

    fs::write(&path, pretty(&value)).map_err(|e| format!("cannot write {}: {}", path.display(), e))
}

fn write_index(fur_dir: &Path, thread_ids: &[String]) -> Result<(), String> {
    let value = json!({
        "threads": thread_ids,
        "active_thread": thread_ids.last(),
        "current_message": Value::Null,
        "created_at": chrono::Utc::now().to_rfc3339(),
        "schema_version": SCHEMA_VERSION
    });

    let path = fur_dir.join("index.json");

    fs::write(&path, pretty(&value)).map_err(|e| format!("cannot write {}: {}", path.display(), e))
}

/// Avatar *names* come from the documents; the emoji mapping and `main` are
/// reader preferences that no document can carry. A placeholder is written so
/// nothing crashes, and onboarding refines it later.
fn write_avatars(fur_dir: &Path, summary: &RebuildSummary) -> Result<(), String> {
    let path = fur_dir.join("avatars.json");

    if path.exists() {
        return Ok(());
    }

    let mut map = serde_json::Map::new();

    if let Some(main) = &summary.guessed_main {
        map.insert("main".to_string(), json!(main));
    }

    for name in &summary.avatars {
        map.insert(name.clone(), json!("🐾"));
    }

    fs::write(&path, pretty(&Value::Object(map)))
        .map_err(|e| format!("cannot write {}: {}", path.display(), e))
}

fn relative_to_root(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("chats")).unwrap();
        dir
    }

    fn spine(root: &Path, slug: &str, body: &str) {
        let folder = root.join("chats").join(slug);
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("convo.md"), body).unwrap();
    }

    const DOC: &str = "---\nfur_schema: 1\nconversation_id: 14e87a09-d24e-4f4e-be73-7c76b4a15f5f\ntitle: Hello\ncreated_at: 2026-08-09T01:23:14Z\ntags: []\n---\n\n<!-- fur:msg id=m1 avatar=andrew ts=2026-08-09T01:23:58Z -->\n\nfirst\n\n<!-- fur:msg id=m2 avatar=andrew ts=2026-08-09T01:24:58Z -->\n\nsecond\n";

    #[test]
    fn detects_an_unindexed_archive() {
        let root = scratch("fur_rb_state");
        spine(&root, "hello-14e87a09", DOC);
        assert_eq!(detect_state(&root), ProjectState::Unindexed);
    }

    #[test]
    fn detects_empty_and_locked() {
        let root = scratch("fur_rb_empty");
        assert_eq!(detect_state(&root), ProjectState::Empty);

        fs::write(root.join("chats").join(LOCK_SENTINEL), "1").unwrap();
        assert_eq!(detect_state(&root), ProjectState::Locked);
    }

    #[test]
    fn rebuilds_threads_messages_and_index() {
        let root = scratch("fur_rb_build");
        spine(&root, "hello-14e87a09", DOC);

        let summary = rebuild(&root, false).unwrap();

        assert_eq!(summary.conversations, 1);
        assert_eq!(summary.messages, 2);
        assert_eq!(summary.guessed_main.as_deref(), Some("andrew"));

        assert!(root
            .join(".fur/threads/14e87a09-d24e-4f4e-be73-7c76b4a15f5f.json")
            .exists());
        assert!(root.join(".fur/messages/m1.json").exists());
        assert!(root.join(".fur/messages/m2.json").exists());

        let index: Value =
            serde_json::from_str(&fs::read_to_string(root.join(".fur/index.json")).unwrap())
                .unwrap();
        assert_eq!(index["threads"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn refuses_to_clobber_live_state() {
        let root = scratch("fur_rb_guard");
        spine(&root, "hello-14e87a09", DOC);
        rebuild(&root, false).unwrap();

        assert!(rebuild(&root, false).is_err());
        assert!(rebuild(&root, true).is_ok());
    }

    #[test]
    fn refuses_to_parse_a_locked_archive() {
        let root = scratch("fur_rb_locked");
        spine(&root, "hello-14e87a09", DOC);
        fs::write(root.join("chats").join(LOCK_SENTINEL), "1").unwrap();

        let err = rebuild(&root, false).unwrap_err();
        assert!(err.contains("locked"), "got: {}", err);
    }

    #[test]
    fn one_bad_document_does_not_sink_the_archive() {
        let root = scratch("fur_rb_partial");
        spine(&root, "hello-14e87a09", DOC);
        spine(&root, "broken-deadbeef", "---\nconversation_id: x\n---\n\nnope\n");

        let summary = rebuild(&root, false).unwrap();

        assert_eq!(summary.conversations, 1);
        assert_eq!(summary.skipped.len(), 1);
    }
}