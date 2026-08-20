//! Assemble a conversation and its lineage into one Markdown document.
//!
//! The unit of value is not the conversation, it is the reasoning behind a
//! conclusion — which routinely spans several conversations and several
//! people. This renders that whole chain as one file a reader can open without
//! knowing FUR exists.
//!
//! Ancestors come first, in dependency order, so the document reads the way
//! the work happened: what it was built on, then the thing itself.

use std::collections::HashSet;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use serde_json::{json, Value};

use crate::renderer::utils::load_message;
use crate::schema::lineage::Lineage;

/// How much of the graph to include.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// The conversation and everything it draws on.
    Ancestors,
    /// Ancestors, plus everything that draws on it.
    Full,
}

pub struct Options {
    pub scope: Scope,
    /// Include the text of long-form attachments inline.
    pub contents: bool,
}

/// Build the document. Returns the Markdown and the conversations it covered.
pub fn render(
    fur_dir: &Path,
    target: &str,
    options: &Options,
) -> Result<(String, usize), String> {
    let lineage = Lineage::load(fur_dir)?;

    if !lineage.is_local(target) {
        return Err(format!("conversation {} is not in this project", short(target)));
    }

    let avatars: Value = fs::read_to_string(fur_dir.join("avatars.json"))
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_else(|| json!({}));

    let mut order = lineage.ancestry(target);

    if options.scope == Scope::Full {
        let seen: HashSet<String> = order.iter().cloned().collect();
        for id in lineage.descendants(target) {
            if !seen.contains(&id) {
                order.push(id);
            }
        }
    }

    let mut out = String::new();

    write_header(&mut out, &lineage, target, &order);
    write_map(&mut out, &lineage, &order, target);

    for id in &order {
        write_conversation(&mut out, fur_dir, &lineage, &avatars, id, target, options);
    }

    write_absent(&mut out, &lineage, target);

    Ok((out, order.len()))
}

fn write_header(out: &mut String, lineage: &Lineage, target: &str, order: &[String]) {
    let title = lineage.title(target).unwrap_or("Untitled");

    let _ = writeln!(out, "# {}", title);
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "_Provenance record — {} conversation(s), assembled by FUR._",
        order.len()
    );
    let _ = writeln!(out);
}

/// A short map at the head, so a reader knows the shape before the content.
fn write_map(out: &mut String, lineage: &Lineage, order: &[String], target: &str) {
    if order.len() < 2 {
        return;
    }

    let _ = writeln!(out, "## Lineage");
    let _ = writeln!(out);

    for id in order {
        let marker = if id == target { " ← this record" } else { "" };
        let parents = lineage.parents_of(id);

        let from = if parents.is_empty() {
            String::new()
        } else {
            let names: Vec<String> = parents
                .iter()
                .map(|p| lineage.title(p).unwrap_or("Untitled").to_string())
                .collect();
            format!(" — from {}", names.join(", "))
        };

        let _ = writeln!(
            out,
            "- **{}** `{}`{}{}",
            lineage.title(id).unwrap_or("Untitled"),
            short(id),
            from,
            marker
        );
    }

    let _ = writeln!(out);
    let _ = writeln!(out, "---");
    let _ = writeln!(out);
}

fn write_conversation(
    out: &mut String,
    fur_dir: &Path,
    lineage: &Lineage,
    avatars: &Value,
    id: &str,
    target: &str,
    options: &Options,
) {
    let title = lineage.title(id).unwrap_or("Untitled");
    let marker = if id == target { "  ← this record" } else { "" };

    let _ = writeln!(out, "## {}{}", title, marker);
    let _ = writeln!(out);
    let _ = writeln!(out, "`{}`", id);
    let _ = writeln!(out);

    let convo_path = fur_dir.join("threads").join(format!("{}.json", id));
    let Some(content) = crate::security::io::read_text_file(&convo_path) else {
        let _ = writeln!(out, "_(unreadable)_");
        let _ = writeln!(out);
        return;
    };
    let Ok(convo) = serde_json::from_str::<Value>(&content) else {
        let _ = writeln!(out, "_(unreadable)_");
        let _ = writeln!(out);
        return;
    };

    let empty: Vec<Value> = Vec::new();
    let message_ids = convo["messages"].as_array().unwrap_or(&empty);

    if message_ids.is_empty() {
        let _ = writeln!(out, "_(no messages)_");
        let _ = writeln!(out);
        return;
    }

    for mid in message_ids {
        let Some(mid) = mid.as_str() else { continue };
        let Some(msg) = load_message(fur_dir, mid, avatars) else {
            continue;
        };

        let _ = writeln!(out, "**{}** · {} {}", msg.name, msg.date_str, msg.time_str);
        let _ = writeln!(out);

        if !msg.text.trim().is_empty() && msg.text != "<no content>" {
            let _ = writeln!(out, "{}", msg.text);
            let _ = writeln!(out);
        }

        if let Some(path) = &msg.markdown {
            if options.contents {
                match fs::read_to_string(path) {
                    Ok(body) => {
                        let _ = writeln!(out, "{}", body.trim_end());
                        let _ = writeln!(out);
                    }
                    Err(_) => {
                        let _ = writeln!(out, "_(attachment missing: {})_", path);
                        let _ = writeln!(out);
                    }
                }
            } else {
                let name = Path::new(path)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or(path);
                let _ = writeln!(out, "_(attachment: {})_", name);
                let _ = writeln!(out);
            }
        }
    }

    let _ = writeln!(out, "---");
    let _ = writeln!(out);
}

/// Name what the document could not include, rather than leaving a silent gap.
fn write_absent(out: &mut String, lineage: &Lineage, target: &str) {
    let absent = lineage.absent_ancestors(target);
    if absent.is_empty() {
        return;
    }

    let _ = writeln!(out, "## Not included");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "These conversations are referenced as sources but are not in this project:"
    );
    let _ = writeln!(out);

    for id in absent {
        let _ = writeln!(out, "- `{}`", id);
    }

    let _ = writeln!(out);
}

fn short(id: &str) -> &str {
    if id.len() >= 8 {
        &id[..8]
    } else {
        id
    }
}