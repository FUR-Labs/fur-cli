//! Bridge between the `.fur/` JSON store and the canonical Markdown document.
//!
//! Phase A: this reads `.fur/` and *writes* `chats/<slug>/convo.md`. It never
//! writes back into `.fur/` and never deletes anything, so running the export
//! is non-destructive and repeatable.
//!
//! Output layout:
//!
//! ```text
//! chats/
//! └── schema-proposal-changes-fur/
//!     ├── convo.md                    <- spine
//!     └── CHAT-20260808-181158.md     <- long-form body, copied in
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::schema::document::{serialize, FurDocument, FurMessage};

/// Build a `FurDocument` from a conversation in `.fur/`.
///
/// Message bodies come from `text`. When a message has a `markdown`
/// attachment instead, the body stays empty and the file is recorded as a
/// `link=` for the caller to copy alongside the spine.
pub fn document_from_thread(fur_dir: &Path, tid: &str) -> Result<FurDocument, String> {
    let convo_path = fur_dir.join("threads").join(format!("{}.json", tid));

    let convo: Value = read_json(&convo_path)?;

    let mut doc = FurDocument::new(
        convo["id"].as_str().unwrap_or(tid),
        convo["title"].as_str().unwrap_or("Untitled"),
        convo["created_at"].as_str().unwrap_or(""),
    );

    doc.tags = string_list(&convo, "tags");
    doc.parents = string_list(&convo, "parents");
    doc.children = string_list(&convo, "children");

    let msg_ids: Vec<String> = convo["messages"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    for mid in &msg_ids {
        let msg_path = fur_dir.join("messages").join(format!("{}.json", mid));

        // A message file that has gone missing is reported, not silently
        // dropped: losing a message during export would be invisible data loss.
        let msg: Value = read_json(&msg_path)
            .map_err(|e| format!("message {} unreadable ({})", short(mid), e))?;

        doc.messages.push(message_from_json(mid, &msg));
    }

    Ok(doc)
}

fn message_from_json(mid: &str, msg: &Value) -> FurMessage {
    let mut out = FurMessage::new(
        msg["id"].as_str().unwrap_or(mid),
        msg["avatar"].as_str().unwrap_or("unknown"),
        msg["timestamp"].as_str().unwrap_or(""),
    );

    if let Some(text) = msg["text"].as_str() {
        out = out.with_body(text);
    }

    if let Some(md_raw) = msg["markdown"].as_str() {
        let filename = Path::new(md_raw)
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or(md_raw)
            .to_string();

        // Prefer the hash already stored by the schema; fall back to reading
        // the file. Absent both, emit the link without a hash rather than
        // fabricating one.
        let hash = msg["markdown_meta"]["hash"]
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| hash_file(Path::new(md_raw)));

        out = out.with_link(filename, hash);
    }

    if let Some(img) = msg["attachment"].as_str() {
        out.img = Path::new(img)
            .file_name()
            .and_then(|f| f.to_str())
            .map(|s| s.to_string());
    }

    out
}

/// Source paths of every long-form file referenced by a conversation, taken
/// from `.fur/` rather than from the document, since the document stores only
/// basenames.
pub fn linked_sources(fur_dir: &Path, tid: &str) -> Result<Vec<PathBuf>, String> {
    let convo: Value = read_json(&fur_dir.join("threads").join(format!("{}.json", tid)))?;

    let mut out = Vec::new();

    let msg_ids = convo["messages"].as_array().cloned().unwrap_or_default();

    for mid in msg_ids.iter().filter_map(|v| v.as_str()) {
        let msg_path = fur_dir.join("messages").join(format!("{}.json", mid));
        let Ok(msg) = read_json(&msg_path) else {
            continue;
        };

        for key in ["markdown", "attachment"] {
            if let Some(raw) = msg[key].as_str() {
                out.push(resolve_relative(raw));
            }
        }
    }

    Ok(out)
}

/// Write `chats/<slug>/convo.md` plus copies of every linked file.
///
/// Returns the folder that was written. Refuses to clobber an existing spine
/// unless `force` is set — an export that silently overwrote a hand-edited
/// document would be exactly the data loss this whole design is trying to end.
pub fn write_conversation_folder(
    project_root: &Path,
    doc: &FurDocument,
    linked: &[PathBuf],
    force: bool,
) -> Result<PathBuf, String> {
    let folder = project_root.join("chats").join(slug_for(doc));
    let spine = folder.join("convo.md");

    if spine.exists() && !force {
        return Err(format!(
            "{} already exists (use --force to overwrite)",
            spine.display()
        ));
    }

    fs::create_dir_all(&folder).map_err(|e| format!("cannot create {}: {}", folder.display(), e))?;

    fs::write(&spine, serialize(doc))
        .map_err(|e| format!("cannot write {}: {}", spine.display(), e))?;

    for src in linked {
        let Some(name) = src.file_name() else {
            continue;
        };
        let dst = folder.join(name);

        if !src.exists() {
            eprintln!("⚠ linked file missing, not copied: {}", src.display());
            continue;
        }
        if dst.exists() && !force {
            continue;
        }
        if same_file(src, &dst) {
            continue;
        }

        fs::copy(src, &dst)
            .map_err(|e| format!("cannot copy {} → {}: {}", src.display(), dst.display(), e))?;
    }

    Ok(folder)
}

/// Keep `chats/<slug>/` in step with `.fur/` for one conversation.
///
/// Dual-write: `.fur/` remains authoritative and is written first by the
/// calling command; this regenerates the document from it. Regeneration rather
/// than append is deliberate — `serialize` is byte-stable, so rewriting the
/// whole spine is idempotent and the two stores cannot drift apart.
pub fn sync_conversation(
    project_root: &Path,
    fur_dir: &Path,
    tid: &str,
) -> Result<PathBuf, String> {
    let doc = document_from_thread(fur_dir, tid)?;
    let linked = linked_sources(fur_dir, tid)?;

    // A retitled conversation wants a new slug. Move the existing folder
    // instead of leaving an orphan behind under the old name.
    let chats = project_root.join("chats");
    let desired = slug_for(&doc);

    if let Some(existing) = find_folder_for(&chats, &doc.conversation_id) {
        let same = existing.file_name().and_then(|f| f.to_str()) == Some(desired.as_str());
        if !same {
            let target = chats.join(&desired);
            if !target.exists() {
                fs::rename(&existing, &target).map_err(|e| {
                    format!("cannot rename {} → {}: {}", existing.display(), target.display(), e)
                })?;
            }
        }
    }

    write_conversation_folder(project_root, &doc, &linked, true)
}

/// Best-effort sync of the active conversation, for commands that have just
/// mutated `.fur/`.
///
/// Never panics and never blocks the command that called it: a failure to
/// update `chats/` is a warning, because `.fur/` is still the source of truth
/// in this phase and the user's write has already succeeded.
pub fn sync_active() {
    if crate::security::state::is_locked() {
        return;
    }

    let fur_dir = Path::new(".fur");
    let index_path = fur_dir.join("index.json");

    if !index_path.exists() {
        return;
    }

    let Some(content) = crate::security::io::read_text_file(&index_path) else {
        return;
    };

    let Ok(index) = serde_json::from_str::<Value>(&content) else {
        return;
    };

    let Some(tid) = index["active_thread"].as_str() else {
        return;
    };

    if tid.is_empty() {
        return;
    }

    if let Err(e) = sync_conversation(Path::new("."), fur_dir, tid) {
        eprintln!("⚠ chats/ not updated: {}", e);
    }
}

fn same_file(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(x), Ok(y)) => x == y,
        _ => false,
    }
}

/// Folder the active conversation's files belong in, creating it if needed.
///
/// `chat` needs this *before* the message exists, so it is derived from the
/// thread metadata rather than by looking for a folder that may not be there
/// yet.
pub fn active_folder(project_root: &Path) -> Option<PathBuf> {
    let fur_dir = project_root.join(".fur");

    let index: Value = read_json(&fur_dir.join("index.json")).ok()?;
    let tid = index["active_thread"].as_str()?;

    if tid.is_empty() {
        return None;
    }

    let doc = document_from_thread(&fur_dir, tid).ok()?;
    let chats = project_root.join("chats");

    // Honour an existing folder even if the title has since changed.
    let folder = find_folder_for(&chats, &doc.conversation_id)
        .unwrap_or_else(|| chats.join(slug_for(&doc)));

    fs::create_dir_all(&folder).ok()?;

    Some(folder)
}

/// Top-level `chats/*.md` files that are already present inside a conversation
/// folder. Rebuild only walks `chats/<slug>/`, so these are invisible to it
/// while still being encrypted by `lock` — orphans in every sense.
pub fn find_orphans(project_root: &Path) -> Vec<PathBuf> {
    let chats = project_root.join("chats");

    let Ok(entries) = fs::read_dir(&chats) else {
        return Vec::new();
    };

    let mut adopted: Vec<String> = Vec::new();

    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        if let Ok(files) = fs::read_dir(&dir) {
            for f in files.flatten() {
                if let Some(name) = f.path().file_name().and_then(|n| n.to_str()) {
                    adopted.push(name.to_string());
                }
            }
        }
    }

    let Ok(entries) = fs::read_dir(&chats) else {
        return Vec::new();
    };

    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("md"))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| adopted.iter().any(|a| a == n))
                .unwrap_or(false)
        })
        .collect()
}

/// Locate an existing conversation folder by its id suffix, so a rename can
/// find the folder even though the slug has changed.
fn find_folder_for(chats: &Path, conversation_id: &str) -> Option<PathBuf> {
    let suffix = format!("-{}", short(conversation_id));

    let entries = fs::read_dir(chats).ok()?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let name = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
        if name.ends_with(&suffix) || name == short(conversation_id) {
            return Some(path);
        }
    }

    None
}

/// Folder name for a conversation: readable title slug, disambiguated by the
/// short id so two conversations with the same title cannot collide.
pub fn slug_for(doc: &FurDocument) -> String {
    let base = slugify(&doc.title);
    let short = short(&doc.conversation_id);

    if base.is_empty() {
        return short.to_string();
    }

    format!("{}-{}", base, short)
}

fn slugify(raw: &str) -> String {
    let mut out = String::new();
    let mut last_dash = true; // suppress a leading dash

    for c in raw.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }

        if out.len() >= 60 {
            break;
        }
    }

    out.trim_matches('-').to_string()
}

fn short(id: &str) -> &str {
    if id.len() >= 8 {
        &id[..8]
    } else {
        id
    }
}

/// `.fur` stores attachment paths either absolute or relative to project root.
fn resolve_relative(raw: &str) -> PathBuf {
    let p = PathBuf::from(raw);
    if p.is_absolute() {
        p
    } else {
        PathBuf::from(".").join(raw)
    }
}

fn hash_file(path: &Path) -> Option<String> {
    let resolved = resolve_relative(path.to_str()?);
    let bytes = fs::read(&resolved).ok()?;

    let mut hasher = Sha256::new();
    hasher.update(&bytes);

    Some(format!("{:x}", hasher.finalize()))
}

/// Read a JSON array of strings, treating an absent key as an empty list.
///
/// extracted from `document_from_thread`, which had this shape
/// inline for `tags` and now needs it three times. Absence is deliberately
/// not an error — thread files written before lineage existed carry no
/// `parents`/`children`, and they are simply conversations with no edges yet.
fn string_list(value: &Value, key: &str) -> Vec<String> {
    value[key]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn read_json(path: &Path) -> Result<Value, String> {
    let content = crate::security::io::read_text_file(path)
        .ok_or_else(|| format!("cannot read {}", path.display()))?;

    serde_json::from_str(&content).map_err(|e| format!("invalid JSON in {}: {}", path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::document::FurDocument;

    fn doc_titled(title: &str, id: &str) -> FurDocument {
        FurDocument::new(id, title, "2026-08-08T18:11:40-05:00")
    }

    #[test]
    fn slug_is_readable_and_disambiguated() {
        let doc = doc_titled(
            "Schema Proposal Changes FUR",
            "8f0c4a2e-1b3d-4f5a-9c7e-2d8b6a1f0e33",
        );
        assert_eq!(slug_for(&doc), "schema-proposal-changes-fur-8f0c4a2e");
    }

    #[test]
    fn slug_collapses_punctuation_and_spaces() {
        let doc = doc_titled("GPT-5   Experiments!! (v2)", "aabbccddeeff");
        assert_eq!(slug_for(&doc), "gpt-5-experiments-v2-aabbccdd");
    }

    #[test]
    fn untitled_conversations_fall_back_to_the_id() {
        let doc = doc_titled("!!!", "aabbccddeeff");
        assert_eq!(slug_for(&doc), "aabbccdd");
    }

    #[test]
    fn message_body_comes_from_text() {
        let msg = serde_json::json!({
            "id": "m1",
            "avatar": "andrew",
            "timestamp": "2026-08-08T18:11:40-05:00",
            "text": "Symbolic regression tests using KAN",
            "markdown": null,
            "attachment": null
        });

        let out = message_from_json("m1", &msg);

        assert_eq!(out.avatar, "andrew");
        assert_eq!(out.body, "Symbolic regression tests using KAN");
        assert!(out.link.is_none());
    }

    #[test]
    fn markdown_attachment_becomes_a_link_with_basename_only() {
        let msg = serde_json::json!({
            "id": "m2",
            "avatar": "gpt5",
            "timestamp": "2026-08-08T18:11:57-05:00",
            "text": null,
            "markdown": "chats/CHAT-20260808-181158.md",
            "markdown_meta": { "hash": "9f2c", "size": 12, "filename": "CHAT-20260808-181158.md" },
            "attachment": null
        });

        let out = message_from_json("m2", &msg);

        assert_eq!(out.link.as_deref(), Some("CHAT-20260808-181158.md"));
        assert_eq!(out.sha256.as_deref(), Some("9f2c"));
        assert!(out.body.is_empty());
    }
}