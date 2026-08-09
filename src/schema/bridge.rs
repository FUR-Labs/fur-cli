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

    doc.tags = convo["tags"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

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
        out.body = text.to_string();
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

        out.link = Some(filename);
        out.sha256 = hash;
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

        fs::copy(src, &dst)
            .map_err(|e| format!("cannot copy {} → {}: {}", src.display(), dst.display(), e))?;
    }

    Ok(folder)
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