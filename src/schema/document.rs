//! Canonical FUR conversation document: serialize / parse.
//!
//! Phase A: `.fur/` is still canonical. This module only proves that a
//! conversation can round-trip losslessly through a human-readable Markdown
//! document, so that `chats/` can later become the source of truth.
//!
//! On-disk layout this format assumes:
//!
//! ```text
//! chats/
//! └── schema-proposal-changes-fur/
//!     ├── convo.md                    <- the spine (this format)
//!     └── CHAT-20260808-181158.md     <- long-form body, referenced by link=
//! ```
//!
//! `link=` is always relative to the conversation folder that holds the spine.
//! Long-form siblings carry no metadata of their own; the folder is the unit of
//! self-containment.

use std::fmt::Write as _;

/// Version of the *document* format. Deliberately an integer, and deliberately
/// separate from `schema::SCHEMA_VERSION` (which versions the `.fur/` JSON).
/// Integers also avoid the lexicographic trap of comparing "0.10" to "0.3".
pub const FUR_DOC_SCHEMA: u32 = 1;

const MARKER_OPEN: &str = "<!-- fur:msg";
const MARKER_CLOSE: &str = "-->";

/// One message in a conversation.
///
/// `body` is the inline text. When `link` is set the body lives in a sibling
/// file and `body` is normally empty — the two are not mutually exclusive, but
/// a linked message with inline text is treated as a caption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FurMessage {
    pub id: String,
    pub avatar: String,
    pub ts: String,
    /// Long-form file, relative to the conversation folder.
    pub link: Option<String>,
    /// SHA-256 of the linked file, for drift detection and repair.
    pub sha256: Option<String>,
    /// Image attachment, relative to the conversation folder.
    pub img: Option<String>,
    pub body: String,
}

impl FurMessage {
    pub fn new(id: impl Into<String>, avatar: impl Into<String>, ts: impl Into<String>) -> Self {
        FurMessage {
            id: id.into(),
            avatar: avatar.into(),
            ts: ts.into(),
            link: None,
            sha256: None,
            img: None,
            body: String::new(),
        }
    }

    pub fn with_body(mut self, body: impl Into<String>) -> Self {
        self.body = body.into();
        self
    }

    pub fn with_link(mut self, link: impl Into<String>, sha256: Option<String>) -> Self {
        self.link = Some(link.into());
        self.sha256 = sha256;
        self
    }
}

/// A whole conversation: front matter plus an ordered list of messages.
///
/// There is no `parent` field yet — absence means the conversation is flat.
/// When branching is switched on, `parent=<id>` appears on messages that have
/// one and every document written today stays valid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FurDocument {
    pub schema: u32,
    pub conversation_id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub tags: Vec<String>,
    pub messages: Vec<FurMessage>,
}

impl FurDocument {
    pub fn new(
        conversation_id: impl Into<String>,
        title: impl Into<String>,
        created_at: impl Into<String>,
    ) -> Self {
        FurDocument {
            schema: FUR_DOC_SCHEMA,
            conversation_id: conversation_id.into(),
            title: title.into(),
            created_at: created_at.into(),
            updated_at: None,
            tags: Vec::new(),
            messages: Vec::new(),
        }
    }
}

// ======================================================
//  SERIALIZE
// ======================================================

/// Render a document to its canonical text form.
///
/// Canonical means byte-stable: `serialize(parse(serialize(d))) == serialize(d)`.
/// That property is what `tests/roundtrip.rs` checks, and it is the whole point
/// of Phase A.
pub fn serialize(doc: &FurDocument) -> String {
    let mut out = String::new();

    out.push_str("---\n");
    let _ = writeln!(out, "fur_schema: {}", doc.schema);
    let _ = writeln!(out, "conversation_id: {}", yaml_scalar(&doc.conversation_id));
    let _ = writeln!(out, "title: {}", yaml_scalar(&doc.title));
    let _ = writeln!(out, "created_at: {}", yaml_scalar(&doc.created_at));
    if let Some(updated) = &doc.updated_at {
        let _ = writeln!(out, "updated_at: {}", yaml_scalar(updated));
    }
    if doc.tags.is_empty() {
        out.push_str("tags: []\n");
    } else {
        out.push_str("tags:\n");
        for tag in &doc.tags {
            let _ = writeln!(out, "  - {}", yaml_scalar(tag));
        }
    }
    out.push_str("---\n");

    for msg in &doc.messages {
        out.push('\n');
        out.push_str(&render_marker(msg));
        out.push('\n');

        let body = msg.body.trim_matches('\n');
        if !body.is_empty() {
            out.push('\n');
            out.push_str(&escape_body(body));
            out.push('\n');
        }
    }

    out
}

fn render_marker(msg: &FurMessage) -> String {
    let mut s = String::from(MARKER_OPEN);
    let _ = write!(s, " id={}", attr_value(&msg.id));
    let _ = write!(s, " avatar={}", attr_value(&msg.avatar));
    let _ = write!(s, " ts={}", attr_value(&msg.ts));
    if let Some(link) = &msg.link {
        let _ = write!(s, " link={}", attr_value(link));
    }
    if let Some(hash) = &msg.sha256 {
        let _ = write!(s, " sha256={}", attr_value(hash));
    }
    if let Some(img) = &msg.img {
        let _ = write!(s, " img={}", attr_value(img));
    }
    s.push(' ');
    s.push_str(MARKER_CLOSE);
    s
}

/// Quote an attribute value only when it must be quoted, so the common case
/// stays readable and the output stays deterministic.
fn attr_value(raw: &str) -> String {
    let needs_quotes = raw.is_empty()
        || raw
            .chars()
            .any(|c| c.is_whitespace() || c == '"' || c == '\\' || c == '=');

    if !needs_quotes {
        return raw.to_string();
    }

    let mut out = String::with_capacity(raw.len() + 2);
    out.push('"');
    for c in raw.chars() {
        if c == '"' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// Same idea for the YAML front matter. Always-quote would also be
/// deterministic, but bare scalars read better in Explorer and Obsidian.
fn yaml_scalar(raw: &str) -> String {
    let safe = !raw.is_empty()
        && !raw.starts_with(' ')
        && !raw.ends_with(' ')
        && raw
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.' | '+' | ':' | '/'))
        && !raw.contains(": ")
        && !raw.starts_with('-');

    if safe {
        return raw.to_string();
    }

    let mut out = String::with_capacity(raw.len() + 2);
    out.push('"');
    for c in raw.chars() {
        if c == '"' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// A body line that looks like a marker would silently split the message in
/// two on the next read. Escaping is reversible: one backslash is added on
/// write, one is removed on read.
fn escape_body(body: &str) -> String {
    body.lines()
        .map(|line| {
            if is_escapable_marker_line(line) {
                format!("\\{}", line)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn unescape_body(body: &str) -> String {
    body.lines()
        .map(|line| {
            if let Some(rest) = line.strip_prefix('\\') {
                if is_escapable_marker_line(rest) {
                    return rest.to_string();
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// True for `<!-- fur:msg ...` and for any already-escaped run of backslashes
/// in front of it, so escaping can nest without losing information.
fn is_escapable_marker_line(line: &str) -> bool {
    let stripped = line.trim_start_matches('\\');
    stripped.starts_with(MARKER_OPEN)
}

// ======================================================
//  PARSE
// ======================================================

/// Parse a canonical document. Errors are strings so this can sit under the
/// existing `eprintln!` style without dragging in an error crate.
pub fn parse(text: &str) -> Result<FurDocument, String> {
    let lines: Vec<&str> = text.lines().collect();

    let (front_matter, body_start) = split_front_matter(&lines)?;
    let mut doc = parse_front_matter(&front_matter)?;
    doc.messages = parse_messages(&lines[body_start..])?;

    let mut seen: Vec<&str> = Vec::with_capacity(doc.messages.len());
    for msg in &doc.messages {
        if msg.id.is_empty() {
            return Err("message marker has an empty id".to_string());
        }
        if seen.contains(&msg.id.as_str()) {
            return Err(format!("duplicate message id '{}'", msg.id));
        }
        seen.push(&msg.id);
    }

    Ok(doc)
}

/// Returns the front-matter lines and the index of the first body line.
fn split_front_matter<'a>(lines: &[&'a str]) -> Result<(Vec<&'a str>, usize), String> {
    if lines.first().map(|l| l.trim_end()) != Some("---") {
        return Err("missing front matter: document must start with '---'".to_string());
    }

    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim_end() == "---" {
            return Ok((lines[1..i].to_vec(), i + 1));
        }
    }

    Err("unterminated front matter: no closing '---'".to_string())
}

fn parse_front_matter(lines: &[&str]) -> Result<FurDocument, String> {
    let mut schema: Option<u32> = None;
    let mut conversation_id: Option<String> = None;
    let mut title: Option<String> = None;
    let mut created_at: Option<String> = None;
    let mut updated_at: Option<String> = None;
    let mut tags: Vec<String> = Vec::new();
    let mut in_tags = false;

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }

        if let Some(item) = line.trim_start().strip_prefix("- ") {
            if !in_tags {
                return Err(format!("unexpected list item in front matter: {}", line));
            }
            tags.push(unquote_scalar(item.trim()));
            continue;
        }

        let Some((key, value)) = line.split_once(':') else {
            return Err(format!("malformed front matter line: {}", line));
        };

        let key = key.trim();
        let value = value.trim();
        in_tags = false;

        match key {
            "fur_schema" => {
                schema = Some(
                    value
                        .parse::<u32>()
                        .map_err(|_| format!("fur_schema must be an integer, got '{}'", value))?,
                );
            }
            "conversation_id" => conversation_id = Some(unquote_scalar(value)),
            "title" => title = Some(unquote_scalar(value)),
            "created_at" => created_at = Some(unquote_scalar(value)),
            "updated_at" => updated_at = Some(unquote_scalar(value)),
            "tags" => {
                if value == "[]" || value.is_empty() {
                    in_tags = value.is_empty();
                } else {
                    return Err(format!("unsupported inline tags value: '{}'", value));
                }
            }
            _ => return Err(format!("unknown front matter key: '{}'", key)),
        }
    }

    let schema = schema.ok_or("front matter missing 'fur_schema'")?;
    if schema > FUR_DOC_SCHEMA {
        return Err(format!(
            "document schema {} is newer than this build supports ({})",
            schema, FUR_DOC_SCHEMA
        ));
    }

    Ok(FurDocument {
        schema,
        conversation_id: conversation_id.ok_or("front matter missing 'conversation_id'")?,
        title: title.ok_or("front matter missing 'title'")?,
        created_at: created_at.ok_or("front matter missing 'created_at'")?,
        updated_at,
        tags,
        messages: Vec::new(),
    })
}

fn parse_messages(lines: &[&str]) -> Result<Vec<FurMessage>, String> {
    let mut messages: Vec<FurMessage> = Vec::new();
    let mut pending: Option<FurMessage> = None;
    let mut body: Vec<&str> = Vec::new();

    for line in lines {
        if is_marker_line(line) {
            if let Some(mut msg) = pending.take() {
                msg.body = finish_body(&body);
                messages.push(msg);
            }
            body.clear();
            pending = Some(parse_marker(line)?);
            continue;
        }

        if pending.is_none() {
            if line.trim().is_empty() {
                continue;
            }
            return Err(format!(
                "content before the first message marker: {}",
                line.trim()
            ));
        }

        body.push(line);
    }

    if let Some(mut msg) = pending.take() {
        msg.body = finish_body(&body);
        messages.push(msg);
    }

    Ok(messages)
}

/// A marker only counts at column zero on its own line. That rule is what lets
/// `escape_body` be a purely line-level operation.
fn is_marker_line(line: &str) -> bool {
    line.starts_with(MARKER_OPEN) && line.trim_end().ends_with(MARKER_CLOSE)
}

fn finish_body(lines: &[&str]) -> String {
    let joined = lines.join("\n");
    unescape_body(joined.trim_matches('\n'))
}

fn parse_marker(line: &str) -> Result<FurMessage, String> {
    let trimmed = line.trim_end();
    let inner = trimmed[MARKER_OPEN.len()..trimmed.len() - MARKER_CLOSE.len()].trim();

    let mut msg = FurMessage::new(String::new(), String::new(), String::new());
    let mut have_id = false;
    let mut have_avatar = false;
    let mut have_ts = false;

    for (key, value) in parse_attributes(inner)? {
        match key.as_str() {
            "id" => {
                msg.id = value;
                have_id = true;
            }
            "avatar" => {
                msg.avatar = value;
                have_avatar = true;
            }
            "ts" => {
                msg.ts = value;
                have_ts = true;
            }
            "link" => msg.link = Some(value),
            "sha256" => msg.sha256 = Some(value),
            "img" => msg.img = Some(value),
            other => return Err(format!("unknown marker attribute '{}'", other)),
        }
    }

    if !have_id || !have_avatar || !have_ts {
        return Err(format!(
            "marker missing required attribute (id, avatar, ts): {}",
            trimmed
        ));
    }

    Ok(msg)
}

/// Split `key=value key="quoted value"` into pairs. Unquoted values end at the
/// next whitespace; quoted values support `\"` and `\\`.
fn parse_attributes(input: &str) -> Result<Vec<(String, String)>, String> {
    let chars: Vec<char> = input.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }

        let key_start = i;
        while i < chars.len() && chars[i] != '=' && !chars[i].is_whitespace() {
            i += 1;
        }
        let key: String = chars[key_start..i].iter().collect();

        if i >= chars.len() || chars[i] != '=' {
            return Err(format!("attribute '{}' is missing '='", key));
        }
        i += 1;

        let value = if i < chars.len() && chars[i] == '"' {
            i += 1;
            let mut v = String::new();
            let mut closed = false;
            while i < chars.len() {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    v.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                if chars[i] == '"' {
                    closed = true;
                    i += 1;
                    break;
                }
                v.push(chars[i]);
                i += 1;
            }
            if !closed {
                return Err(format!("unterminated quoted value for '{}'", key));
            }
            v
        } else {
            let value_start = i;
            while i < chars.len() && !chars[i].is_whitespace() {
                i += 1;
            }
            chars[value_start..i].iter().collect()
        };

        out.push((key, value));
    }

    Ok(out)
}

fn unquote_scalar(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        let inner: Vec<char> = trimmed[1..trimmed.len() - 1].chars().collect();
        let mut out = String::with_capacity(inner.len());
        let mut i = 0;
        while i < inner.len() {
            if inner[i] == '\\' && i + 1 < inner.len() {
                out.push(inner[i + 1]);
                i += 2;
                continue;
            }
            out.push(inner[i]);
            i += 1;
        }
        return out;
    }
    trimmed.to_string()
}

// ======================================================
//  TESTS
// ======================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> FurDocument {
        let mut doc = FurDocument::new(
            "8f0c4a2e-1b3d-4f5a-9c7e-2d8b6a1f0e33",
            "Schema Proposal Changes FUR",
            "2026-08-08T18:11:40-05:00",
        );
        doc.updated_at = Some("2026-08-08T19:02:15-05:00".to_string());
        doc.tags = vec!["schema".to_string(), "architecture".to_string()];
        doc.messages = vec![
            FurMessage::new("3a7f9c21", "andrew", "2026-08-08T18:11:40-05:00")
                .with_body("I think that I may have a better idea..."),
            FurMessage::new("b2e14d80", "gpt5", "2026-08-08T18:11:57-05:00")
                .with_link("CHAT-20260808-181158.md", Some("9f2c".to_string())),
            FurMessage::new("c17aa903", "andrew", "2026-08-08T18:14:02-05:00").with_body("badass"),
        ];
        doc
    }

    #[test]
    fn round_trip_is_byte_stable() {
        let once = serialize(&sample());
        let parsed = parse(&once).expect("parses");
        assert_eq!(once, serialize(&parsed));
        assert_eq!(sample(), parsed);
    }

    #[test]
    fn body_containing_a_marker_survives() {
        let mut doc = FurDocument::new("cid", "Escaping", "2026-08-08T00:00:00Z");
        let evil = "Here is the format:\n<!-- fur:msg id=fake avatar=x ts=y -->\ndone";
        doc.messages = vec![FurMessage::new("m1", "andrew", "2026-08-08T00:00:00Z").with_body(evil)];

        let text = serialize(&doc);
        let parsed = parse(&text).expect("parses");

        assert_eq!(parsed.messages.len(), 1);
        assert_eq!(parsed.messages[0].body, evil);
        assert_eq!(text, serialize(&parsed));
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let mut doc = FurDocument::new("cid", "Dupes", "2026-08-08T00:00:00Z");
        doc.messages = vec![
            FurMessage::new("same", "andrew", "2026-08-08T00:00:00Z").with_body("one"),
            FurMessage::new("same", "gpt5", "2026-08-08T00:00:01Z").with_body("two"),
        ];

        let err = parse(&serialize(&doc)).unwrap_err();
        assert!(err.contains("duplicate message id"), "got: {}", err);
    }

    #[test]
    fn values_with_spaces_round_trip() {
        let mut doc = FurDocument::new("cid", "Title: with punctuation", "2026-08-08T00:00:00Z");
        doc.tags = vec!["deep-learning".to_string()];
        doc.messages = vec![FurMessage::new("m1", "karen from hr", "2026-08-08T00:00:00Z")
            .with_link("CHAT with spaces.md", None)];

        let text = serialize(&doc);
        let parsed = parse(&text).expect("parses");

        assert_eq!(parsed.title, "Title: with punctuation");
        assert_eq!(parsed.messages[0].avatar, "karen from hr");
        assert_eq!(parsed.messages[0].link.as_deref(), Some("CHAT with spaces.md"));
        assert_eq!(text, serialize(&parsed));
    }

    #[test]
    fn missing_front_matter_is_an_error() {
        assert!(parse("just some markdown\n").is_err());
    }
}