//! Canonical FUR conversation document: serialize / parse.
//!
//! `.fur/` is operational state. This module defines the durable format: a
//! conversation is a Markdown file that a person can read and a machine can
//! parse, and `tests/roundtrip.rs` proves the trip is lossless.
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
//!
//! # Lineage (schema 2)
//!
//! `parents` and `children` record edges to *other conversations*, by
//! `conversation_id`. Together they form a directed graph across the archive:
//! a methodology discussion informs a forecast, which informs a revision.
//!
//! Three properties are deliberate, and each has a reason:
//!
//! - **Edges are ids, never folder slugs.** `bridge::slug_for` embeds the
//!   title, and `sync_conversation` renames the folder when the title changes.
//!   A slug-based edge would break on the first rename, and would not survive
//!   registry import at all.
//! - **Dangling edges are legal.** A conversation imported from a registry can
//!   reference conversations the importer has never seen. An unresolvable edge
//!   is *partial knowledge*, not corruption — unlike a missing `link=` file,
//!   which is a broken document.
//! - **Cycles are legal here, and handled by traversal.** Two people can
//!   independently publish A→B and B→A; the cycle only exists once both are in
//!   one archive. Rejecting it at parse time would make that archive
//!   unrepresentable. Every walker carries a visited set — which it needs for
//!   diamonds regardless.
//!
//! Self-reference *is* rejected: it is never meaningful and always a mistake.

use std::fmt::Write as _;

/// Version of the *document* format. Deliberately an integer, and deliberately
/// separate from `schema::SCHEMA_VERSION` (which versions the `.fur/` JSON).
/// Integers also avoid the lexicographic trap of comparing "0.10" to "0.3".
///
/// 1 → 2: added `parents` / `children`, and inline flow sequences in front
/// matter. A schema-1 build rejects unknown front-matter keys outright, so
/// documents carrying lineage must announce themselves as 2.
pub const FUR_DOC_SCHEMA: u32 = 2;

/// Oldest document version this build still reads.
pub const FUR_DOC_SCHEMA_MIN: u32 = 1;

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
/// Messages are flat and their order is positional — the order they appear in
/// the document *is* the sequence. Structure between conversations lives in
/// `parents` / `children`; there is no structure within one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FurDocument {
    pub schema: u32,
    pub conversation_id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub tags: Vec<String>,
    /// Conversations this one draws on, by `conversation_id`.
    pub parents: Vec<String>,
    /// Conversations that draw on this one, by `conversation_id`.
    pub children: Vec<String>,
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
            parents: Vec::new(),
            children: Vec::new(),
            messages: Vec::new(),
        }
    }

    /// Version this document must be written as.
    ///
    /// A schema-1 document with no lineage stays schema 1, so archives written
    /// before this change re-serialize byte-identically. The moment it gains an
    /// edge it becomes schema 2, because a schema-1 reader would reject the new
    /// keys as unknown rather than reporting a version it cannot handle.
    pub fn effective_schema(&self) -> u32 {
        if self.parents.is_empty() && self.children.is_empty() {
            self.schema
        } else {
            self.schema.max(2)
        }
    }

    /// Check the invariants `parse` enforces, for code that builds documents in
    /// memory rather than reading them.
    ///
    /// `serialize` does not validate — it is infallible by design — so a
    /// command that mutates lineage should call this before writing.
    pub fn validate(&self) -> Result<(), String> {
        check_refs(&self.parents, &self.conversation_id, "parents")?;
        check_refs(&self.children, &self.conversation_id, "children")?;

        let mut seen: Vec<&str> = Vec::with_capacity(self.messages.len());
        for msg in &self.messages {
            if msg.id.is_empty() {
                return Err("message has an empty id".to_string());
            }
            if seen.contains(&msg.id.as_str()) {
                return Err(format!("duplicate message id '{}'", msg.id));
            }
            seen.push(&msg.id);
        }

        Ok(())
    }
}

// ======================================================
//  LINEAGE HELPERS
// ======================================================

/// Reject the things that are never meaningful, whatever the source.
///
/// Dangling and cyclic references pass: see the module docs for why.
fn check_refs(refs: &[String], conversation_id: &str, field: &str) -> Result<(), String> {
    for reference in refs {
        if reference.trim().is_empty() {
            return Err(format!("empty conversation reference in '{}'", field));
        }
        if reference == conversation_id {
            return Err(format!(
                "conversation {} lists itself in '{}'",
                conversation_id, field
            ));
        }
    }
    Ok(())
}

/// Sort and de-duplicate, so the same edge set always produces the same bytes.
///
/// viceroy: this is what makes concurrent edits merge cleanly. Two people
/// adding different parents to the same document produce a Git conflict whose
/// resolution is a union, in an order neither of them chose.
fn canonical_refs(refs: &[String]) -> Vec<String> {
    let mut out: Vec<String> = refs.iter().map(|r| r.trim().to_string()).collect();
    out.sort();
    out.dedup();
    out
}

// ======================================================
//  SERIALIZE
// ======================================================

/// Render a document to its canonical text form.
///
/// Canonical means byte-stable: `serialize(parse(serialize(d))) == serialize(d)`.
/// That property is what `tests/roundtrip.rs` checks, and it is the whole point
/// of the storage inversion.
pub fn serialize(doc: &FurDocument) -> String {
    let mut out = String::new();

    out.push_str("---\n");
    let _ = writeln!(out, "fur_schema: {}", doc.effective_schema());
    let _ = writeln!(out, "conversation_id: {}", yaml_scalar(&doc.conversation_id));
    let _ = writeln!(out, "title: {}", yaml_scalar(&doc.title));
    let _ = writeln!(out, "created_at: {}", yaml_scalar(&doc.created_at));
    if let Some(updated) = &doc.updated_at {
        let _ = writeln!(out, "updated_at: {}", yaml_scalar(updated));
    }
    write_scalar_list(&mut out, "tags", &doc.tags);
    write_scalar_list(&mut out, "parents", &canonical_refs(&doc.parents));
    write_scalar_list(&mut out, "children", &canonical_refs(&doc.children));
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

/// Empty lists render inline as `[]`; non-empty ones as a block sequence.
///
/// The keys are always emitted, empty or not. They are part of the format's
/// visible surface: someone hand-editing a `convo.md` should be able to see
/// that lineage exists and fill it in without consulting documentation.
fn write_scalar_list(out: &mut String, key: &str, values: &[String]) {
    if values.is_empty() {
        let _ = writeln!(out, "{}: []", key);
        return;
    }

    let _ = writeln!(out, "{}:", key);
    for value in values {
        let _ = writeln!(out, "  - {}", yaml_scalar(value));
    }
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
        && !raw.starts_with('-')
        && !raw.starts_with('[');

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

/// Which block sequence the front-matter parser is currently inside.
///
/// viceroy: this replaces the `in_tags` boolean, which could only ever track
/// one list. Three keys now take sequences.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ListKey {
    Tags,
    Parents,
    Children,
}

impl ListKey {
    fn from_key(key: &str) -> Option<Self> {
        match key {
            "tags" => Some(ListKey::Tags),
            "parents" => Some(ListKey::Parents),
            "children" => Some(ListKey::Children),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            ListKey::Tags => "tags",
            ListKey::Parents => "parents",
            ListKey::Children => "children",
        }
    }
}

/// Parse a canonical document. Errors are strings so this can sit under the
/// existing `eprintln!` style without dragging in an error crate.
pub fn parse(text: &str) -> Result<FurDocument, String> {
    let lines: Vec<&str> = text.lines().collect();

    let (front_matter, body_start) = split_front_matter(&lines)?;
    let mut doc = parse_front_matter(&front_matter)?;
    doc.messages = parse_messages(&lines[body_start..])?;

    // Canonicalise on read as well as on write, so a hand-edited document with
    // duplicate or unsorted edges still satisfies `parse(serialize(d)) == d`.
    doc.parents = canonical_refs(&doc.parents);
    doc.children = canonical_refs(&doc.children);

    doc.validate()?;

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
    let mut parents: Vec<String> = Vec::new();
    let mut children: Vec<String> = Vec::new();
    let mut current_list: Option<ListKey> = None;

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }

        if let Some(item) = line.trim_start().strip_prefix("- ") {
            let Some(list) = current_list else {
                return Err(format!("unexpected list item in front matter: {}", line));
            };
            let value = unquote_scalar(item.trim());
            match list {
                ListKey::Tags => tags.push(value),
                ListKey::Parents => parents.push(value),
                ListKey::Children => children.push(value),
            }
            continue;
        }

        let Some((key, value)) = line.split_once(':') else {
            return Err(format!("malformed front matter line: {}", line));
        };

        let key = key.trim();
        let value = value.trim();
        current_list = None;

        if let Some(list) = ListKey::from_key(key) {
            let target = match list {
                ListKey::Tags => &mut tags,
                ListKey::Parents => &mut parents,
                ListKey::Children => &mut children,
            };

            if value.is_empty() {
                // A block sequence follows on the next lines.
                current_list = Some(list);
            } else if value.starts_with('[') && value.ends_with(']') {
                *target = parse_flow_sequence(&value[1..value.len() - 1], list.name())?;
            } else {
                return Err(format!("unsupported value for '{}': '{}'", list.name(), value));
            }
            continue;
        }

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
    if schema < FUR_DOC_SCHEMA_MIN {
        return Err(format!(
            "document schema {} is older than this build supports ({})",
            schema, FUR_DOC_SCHEMA_MIN
        ));
    }

    Ok(FurDocument {
        schema,
        conversation_id: conversation_id.ok_or("front matter missing 'conversation_id'")?,
        title: title.ok_or("front matter missing 'title'")?,
        created_at: created_at.ok_or("front matter missing 'created_at'")?,
        updated_at,
        tags,
        parents,
        children,
        messages: Vec::new(),
    })
}

/// Parse the inside of a `[a, b, "c d"]` flow sequence.
///
/// viceroy: schema 1 accepted only the literal `[]` and errored on anything
/// else inline. Lineage lists are short and read far better on one line in a
/// diff, so the real thing is parsed now — for `tags` too, which previously
/// forced block style the moment it had a single entry.
fn parse_flow_sequence(inner: &str, field: &str) -> Result<Vec<String>, String> {
    let trimmed = inner.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let chars: Vec<char> = trimmed.chars().collect();
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut closed_quote = false;
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if quoted {
            if c == '\\' && i + 1 < chars.len() {
                current.push(chars[i + 1]);
                i += 2;
                continue;
            }
            if c == '"' {
                quoted = false;
                closed_quote = true;
                i += 1;
                continue;
            }
            current.push(c);
            i += 1;
            continue;
        }

        match c {
            '"' if current.trim().is_empty() && !closed_quote => {
                current.clear();
                quoted = true;
                i += 1;
            }
            ',' => {
                out.push(std::mem::take(&mut current).trim().to_string());
                closed_quote = false;
                i += 1;
            }
            _ => {
                current.push(c);
                i += 1;
            }
        }
    }

    if quoted {
        return Err(format!("unterminated quoted value in '{}'", field));
    }

    out.push(current.trim().to_string());

    if out.iter().any(|v| v.is_empty()) {
        return Err(format!("empty entry in '{}' list", field));
    }

    Ok(out)
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

    const A: &str = "aaaaaaaa-1111-4111-8111-111111111111";
    const B: &str = "bbbbbbbb-2222-4222-8222-222222222222";
    const C: &str = "cccccccc-3333-4333-8333-333333333333";
    const D: &str = "dddddddd-4444-4444-8444-444444444444";

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

    fn round_trips(doc: &FurDocument) -> FurDocument {
        let once = serialize(doc);
        let parsed = parse(&once).expect("parses");
        assert_eq!(once, serialize(&parsed), "serialization is not idempotent");
        parsed
    }

    #[test]
    fn round_trip_is_byte_stable() {
        let parsed = round_trips(&sample());
        assert_eq!(sample(), parsed);
    }

    #[test]
    fn body_containing_a_marker_survives() {
        let mut doc = FurDocument::new("cid", "Escaping", "2026-08-08T00:00:00Z");
        let evil = "Here is the format:\n<!-- fur:msg id=fake avatar=x ts=y -->\ndone";
        doc.messages = vec![FurMessage::new("m1", "andrew", "2026-08-08T00:00:00Z").with_body(evil)];

        let parsed = round_trips(&doc);
        assert_eq!(parsed.messages.len(), 1);
        assert_eq!(parsed.messages[0].body, evil);
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

        let parsed = round_trips(&doc);
        assert_eq!(parsed.title, "Title: with punctuation");
        assert_eq!(parsed.messages[0].avatar, "karen from hr");
        assert_eq!(parsed.messages[0].link.as_deref(), Some("CHAT with spaces.md"));
    }

    #[test]
    fn missing_front_matter_is_an_error() {
        assert!(parse("just some markdown\n").is_err());
    }

    // ---------- lineage ----------

    #[test]
    fn lineage_round_trips() {
        let mut doc = FurDocument::new(A, "Business plan", "2026-08-17T00:14:20Z");
        doc.children = vec![B.to_string(), C.to_string()];

        let parsed = round_trips(&doc);
        assert_eq!(parsed.children, vec![B.to_string(), C.to_string()]);
        assert!(parsed.parents.is_empty());
        assert_eq!(parsed.schema, 2);
    }

    #[test]
    fn lineage_is_sorted_and_deduplicated() {
        let mut doc = FurDocument::new(A, "Unsorted", "2026-08-17T00:14:20Z");
        doc.parents = vec![C.to_string(), B.to_string(), C.to_string()];

        let parsed = round_trips(&doc);
        assert_eq!(parsed.parents, vec![B.to_string(), C.to_string()]);
    }

    #[test]
    fn self_reference_is_rejected() {
        let mut doc = FurDocument::new(A, "Ouroboros", "2026-08-17T00:14:20Z");
        doc.parents = vec![A.to_string()];

        let err = parse(&serialize(&doc)).unwrap_err();
        assert!(err.contains("lists itself"), "got: {}", err);
    }

    #[test]
    fn dangling_references_are_legal() {
        let mut doc = FurDocument::new(A, "Imported alone", "2026-08-17T00:14:20Z");
        doc.parents = vec!["ffffffff-0000-4000-8000-000000000000".to_string()];

        let parsed = round_trips(&doc);
        assert_eq!(parsed.parents.len(), 1);
    }

    #[test]
    fn a_cycle_between_two_documents_is_representable() {
        let mut first = FurDocument::new(A, "First", "2026-08-17T00:14:20Z");
        first.children = vec![B.to_string()];
        let mut second = FurDocument::new(B, "Second", "2026-08-17T00:15:20Z");
        second.children = vec![A.to_string()];

        // Neither document is invalid on its own; the cycle only exists in the
        // union, and traversal is what has to cope with it.
        assert_eq!(round_trips(&first).children, vec![B.to_string()]);
        assert_eq!(round_trips(&second).children, vec![A.to_string()]);
    }

    #[test]
    fn a_diamond_is_representable() {
        let mut top = FurDocument::new(A, "Top", "2026-08-17T00:14:20Z");
        top.children = vec![B.to_string(), C.to_string()];

        let mut bottom = FurDocument::new(D, "Bottom", "2026-08-17T00:17:20Z");
        bottom.parents = vec![B.to_string(), C.to_string()];

        assert_eq!(round_trips(&top).children.len(), 2);
        assert_eq!(round_trips(&bottom).parents.len(), 2);
    }

    #[test]
    fn inline_flow_sequences_parse() {
        let text = format!(
            "---\nfur_schema: 2\nconversation_id: {}\ntitle: Flow\ncreated_at: 2026-08-17T00:14:20Z\ntags: [research, \"deep learning\"]\nparents: [{}]\nchildren: []\n---\n",
            A, B
        );

        let doc = parse(&text).expect("parses");
        assert_eq!(doc.tags, vec!["research".to_string(), "deep learning".to_string()]);
        assert_eq!(doc.parents, vec![B.to_string()]);
        assert!(doc.children.is_empty());
    }

    #[test]
    fn block_and_flow_forms_agree() {
        let flow = format!(
            "---\nfur_schema: 2\nconversation_id: {}\ntitle: Same\ncreated_at: 2026-08-17T00:14:20Z\ntags: []\nparents: [{}, {}]\nchildren: []\n---\n",
            A, B, C
        );
        let block = format!(
            "---\nfur_schema: 2\nconversation_id: {}\ntitle: Same\ncreated_at: 2026-08-17T00:14:20Z\ntags: []\nparents:\n  - {}\n  - {}\nchildren: []\n---\n",
            A, B, C
        );

        assert_eq!(parse(&flow).unwrap(), parse(&block).unwrap());
        assert_eq!(serialize(&parse(&flow).unwrap()), serialize(&parse(&block).unwrap()));
    }

    #[test]
    fn schema_one_documents_still_parse() {
        let text = format!(
            "---\nfur_schema: 1\nconversation_id: {}\ntitle: Old\ncreated_at: 2026-08-08T18:11:40-05:00\ntags: []\n---\n\n<!-- fur:msg id=m1 avatar=andrew ts=2026-08-08T18:11:40-05:00 -->\n\nhello\n",
            A
        );

        let doc = parse(&text).expect("parses");
        assert_eq!(doc.schema, 1);
        assert!(doc.parents.is_empty());
        assert_eq!(doc.messages.len(), 1);
    }

    #[test]
    fn a_schema_one_document_without_lineage_stays_schema_one() {
        let mut doc = FurDocument::new(A, "Old", "2026-08-08T18:11:40-05:00");
        doc.schema = 1;
        assert!(serialize(&doc).contains("fur_schema: 1"));
    }

    #[test]
    fn adding_lineage_upgrades_the_document() {
        let mut doc = FurDocument::new(A, "Old", "2026-08-08T18:11:40-05:00");
        doc.schema = 1;
        doc.parents = vec![B.to_string()];
        assert!(serialize(&doc).contains("fur_schema: 2"));
    }

    #[test]
    fn a_future_schema_is_refused_by_version_not_by_key() {
        let text = format!(
            "---\nfur_schema: 3\nconversation_id: {}\ntitle: Future\ncreated_at: 2026-08-17T00:14:20Z\ntags: []\n---\n",
            A
        );

        let err = parse(&text).unwrap_err();
        assert!(err.contains("newer than this build"), "got: {}", err);
    }

    #[test]
    fn an_empty_reference_is_rejected() {
        let text = format!(
            "---\nfur_schema: 2\nconversation_id: {}\ntitle: Empty ref\ncreated_at: 2026-08-17T00:14:20Z\ntags: []\nparents: [{}, ]\nchildren: []\n---\n",
            A, B
        );

        let err = parse(&text).unwrap_err();
        assert!(err.contains("empty entry"), "got: {}", err);
    }
}