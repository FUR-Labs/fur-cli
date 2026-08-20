//! Conversation-level lineage: `fur link` / `fur unlink`.
//!
//! Edges are stored in `.fur/threads/<id>.json` under `parents` / `children`
//! and carried out to `chats/<slug>/convo.md` by the usual sync. `.fur/` is
//! written first and remains authoritative; the document is regenerated from
//! it, exactly as `jot` does.
//!
//! # Reciprocity
//!
//! `parents` and `children` are *assertions*, not two halves of one record.
//! When both conversations are local, linking writes both ends. When the other
//! end is not local — imported from a registry, or simply not pulled yet — only
//! this end is written, and that is a complete, valid state rather than a
//! failure. `fur doctor` reports asymmetry; it does not treat it as damage.

use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;
use colored::*;
use serde_json::{json, Value};

use crate::schema::bridge::sync_conversation;

#[derive(Parser, Debug)]
pub struct LinkArgs {
    /// Conversation to modify (id or unique prefix). Defaults to the active one.
    pub id: Option<String>,

    /// Conversation this one derives from
    #[arg(long, conflicts_with = "child")]
    pub parent: Option<String>,

    /// Conversation that derives from this one
    #[arg(long)]
    pub child: Option<String>,
}

/// Which side of the edge the user named.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Parent,
    Child,
}

impl Edge {
    /// Field on the conversation being modified.
    fn field(self) -> &'static str {
        match self {
            Edge::Parent => "parents",
            Edge::Child => "children",
        }
    }

    /// Field on the conversation at the far end of the edge.
    fn inverse_field(self) -> &'static str {
        match self {
            Edge::Parent => "children",
            Edge::Child => "parents",
        }
    }

    fn describe(self) -> &'static str {
        match self {
            Edge::Parent => "parent",
            Edge::Child => "child",
        }
    }
}

/// What a single-sided edit actually did.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Added,
    Removed,
    AlreadyPresent,
    NotPresent,
}

pub fn run_link(args: LinkArgs) {
    dispatch(args, false);
}

pub fn run_unlink(args: LinkArgs) {
    dispatch(args, true);
}

fn dispatch(args: LinkArgs, remove: bool) {
    let edge = match (&args.parent, &args.child) {
        (Some(_), None) => Edge::Parent,
        (None, Some(_)) => Edge::Child,
        (None, None) => {
            eprintln!("❌ Specify --parent <id> or --child <id>.");
            return;
        }
        (Some(_), Some(_)) => {
            eprintln!("❌ Pass one of --parent or --child, not both.");
            return;
        }
    };

    let other_ref = args
        .parent
        .as_deref()
        .or(args.child.as_deref())
        .unwrap_or_default();

    match apply(Path::new("."), args.id.as_deref(), other_ref, edge, remove) {
        Ok(report) => report.print(),
        Err(e) => eprintln!("❌ {}", e),
    }
}

#[derive(Debug)]
pub struct LinkReport {
    edge: &'static str,
    target: String,
    target_title: String,
    other: String,
    other_title: Option<String>,
    near: Outcome,
    far: Option<Outcome>,
}

impl LinkReport {
    fn print(&self) {
        let other_label = match &self.other_title {
            Some(title) => format!("{} \"{}\"", short(&self.other), title),
            None => format!("{} {}", short(&self.other), "(no local copy)".bright_black()),
        };

        match self.near {
            Outcome::Added => println!(
                "🔗 {} \"{}\" — {} → {}",
                short(&self.target).bright_yellow(),
                self.target_title,
                self.edge.bright_cyan(),
                other_label
            ),
            Outcome::Removed => println!(
                "✂️  {} \"{}\" — removed {} {}",
                short(&self.target).bright_yellow(),
                self.target_title,
                self.edge.bright_cyan(),
                other_label
            ),
            Outcome::AlreadyPresent => println!(
                "• Already linked: {} is a {} of {}",
                short(&self.other),
                self.edge,
                short(&self.target)
            ),
            Outcome::NotPresent => println!(
                "• Not linked: {} is not a {} of {}",
                short(&self.other),
                self.edge,
                short(&self.target)
            ),
        }

        match &self.far {
            Some(Outcome::Added) | Some(Outcome::Removed) => {
                println!("   {}", "reciprocal edge updated".bright_black())
            }
            None => println!(
                "   {}",
                "one-sided: the other conversation is not in this project".bright_black()
            ),
            _ => {}
        }
    }
}

/// Add or remove one edge, writing both ends when both are local.
///
/// `root` is explicit so this is testable without changing the process
/// directory.
pub fn apply(
    root: &Path,
    target_ref: Option<&str>,
    other_ref: &str,
    edge: Edge,
    remove: bool,
) -> Result<LinkReport, String> {
    let fur_dir = root.join(".fur");
    let index = read_json(&fur_dir.join("index.json"))?;
    let ids = thread_ids(&index);

    let target = match target_ref {
        Some(prefix) => resolve_local(&ids, prefix)?
            .ok_or_else(|| format!("no conversation matches '{}'", prefix))?,
        None => index["active_thread"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or("no active conversation; pass a conversation id")?
            .to_string(),
    };

    let (other, other_is_local) = match resolve_local(&ids, other_ref)? {
        Some(id) => (id, true),
        None if looks_like_conversation_id(other_ref) => (other_ref.to_string(), false),
        None => {
            return Err(format!(
                "no conversation matches '{}'; pass a full conversation id to link one that is not in this project",
                other_ref
            ))
        }
    };

    if target == other {
        return Err(format!(
            "{} \"{}\" cannot be its own parent or child — name the other conversation explicitly, e.g. `fur link <id> --child {}`",
            short(&target),
            title_of(&fur_dir, &target).unwrap_or_else(|| "Untitled".to_string()),
            short(&other)
        ));
    }

    // A loop can only be *created* locally, so it is refused here rather than in
    // the format. An archive can still contain one after importing two halves
    // published independently, which is why every walk keeps a visited set.
    if !remove {
        let (parent, child) = match edge {
            Edge::Parent => (other.as_str(), target.as_str()),
            Edge::Child => (target.as_str(), other.as_str()),
        };

        if let Ok(lineage) = crate::schema::lineage::Lineage::load(&fur_dir) {
            if lineage.would_cycle(parent, child) {
                return Err(format!(
                    "that would loop: {} already leads back to {}",
                    short(child),
                    short(parent)
                ));
            }
        }
    }

    let near = edit_edge(&fur_dir, &target, edge.field(), &other, remove)?;
    let far = if other_is_local {
        Some(edit_edge(
            &fur_dir,
            &other,
            edge.inverse_field(),
            &target,
            remove,
        )?)
    } else {
        None
    };

    // Regenerate only the documents that actually changed.
    if matches!(near, Outcome::Added | Outcome::Removed) {
        sync_conversation(root, &fur_dir, &target)?;
    }
    if matches!(far, Some(Outcome::Added) | Some(Outcome::Removed)) {
        sync_conversation(root, &fur_dir, &other)?;
    }

    Ok(LinkReport {
        edge: edge.describe(),
        target_title: title_of(&fur_dir, &target).unwrap_or_else(|| "Untitled".to_string()),
        other_title: title_of(&fur_dir, &other),
        target,
        other,
        near,
        far,
    })
}

/// Mutate one conversation's edge list in `.fur/`.
fn edit_edge(
    fur_dir: &Path,
    tid: &str,
    field: &str,
    value: &str,
    remove: bool,
) -> Result<Outcome, String> {
    let path = thread_path(fur_dir, tid);
    let mut convo = read_json(&path)?;

    let mut list: Vec<String> = convo[field]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let present = list.iter().any(|item| item == value);

    if remove {
        if !present {
            return Ok(Outcome::NotPresent);
        }
        list.retain(|item| item != value);
    } else {
        if present {
            return Ok(Outcome::AlreadyPresent);
        }
        list.push(value.to_string());
    }

    // Canonical on disk as well as in the document, so `.fur/` and `convo.md`
    // never disagree about ordering.
    list.sort();
    list.dedup();
    convo[field] = json!(list);

    fs::write(&path, pretty(&convo))
        .map_err(|e| format!("cannot write {}: {}", path.display(), e))?;

    Ok(if remove { Outcome::Removed } else { Outcome::Added })
}

fn thread_ids(index: &Value) -> Vec<String> {
    index["threads"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// Exact match first, then unique prefix — the same rule `fur convo` uses.
fn resolve_local(ids: &[String], needle: &str) -> Result<Option<String>, String> {
    if needle.trim().is_empty() {
        return Err("empty conversation reference".to_string());
    }

    if let Some(hit) = ids.iter().find(|id| id.as_str() == needle) {
        return Ok(Some(hit.clone()));
    }

    let matches: Vec<&String> = ids.iter().filter(|id| id.starts_with(needle)).collect();

    match matches.as_slice() {
        [] => Ok(None),
        [single] => Ok(Some((*single).clone())),
        _ => Err(format!(
            "ambiguous conversation prefix '{}': {:?}",
            needle,
            matches.iter().map(|m| short(m)).collect::<Vec<_>>()
        )),
    }
}

/// A reference to a conversation this project does not hold is only accepted
/// in full, so a typo cannot quietly become a dangling edge.
fn looks_like_conversation_id(raw: &str) -> bool {
    raw.len() >= 32 && raw.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
}

fn title_of(fur_dir: &Path, tid: &str) -> Option<String> {
    let convo = read_json(&thread_path(fur_dir, tid)).ok()?;
    convo["title"].as_str().map(|s| s.to_string())
}

fn thread_path(fur_dir: &Path, tid: &str) -> PathBuf {
    fur_dir.join("threads").join(format!("{}.json", tid))
}

fn read_json(path: &Path) -> Result<Value, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {}", path.display(), e))?;
    serde_json::from_str(&content)
        .map_err(|e| format!("invalid JSON in {}: {}", path.display(), e))
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string())
}

fn short(id: &str) -> &str {
    if id.len() >= 8 {
        &id[..8]
    } else {
        id
    }
}

// --- test seam -------------------------------------------------------------
//
// `Edge` is an implementation detail of argument parsing, but the
// integration test needs to name a direction without going through clap.
//
// The binary compiles the command modules separately from the library, where
// this seam is intentionally unused — the same `#[allow(dead_code)]` situation
// as `registry::install_pull_value`.

#[allow(dead_code)]
pub fn edge_parent() -> Edge {
    Edge::Parent
}

#[allow(dead_code)]
pub fn edge_child() -> Edge {
    Edge::Child
}

#[allow(dead_code)]
impl LinkReport {
    pub fn near_outcome(&self) -> &Outcome {
        &self.near
    }

    pub fn far_outcome(&self) -> Option<&Outcome> {
        self.far.as_ref()
    }
}