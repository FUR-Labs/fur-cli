//! Export the project's conversation lineage as JSON, with layout.
//!
//! The CLI computes the graph; it does not draw it. Layering and crossing
//! reduction are the expensive, reusable part, so they ship in the payload —
//! a front end can honour `x`/`y` directly, use `layer` and run its own
//! placement, or ignore both and use the edges alone.
//!
//! # Determinism
//!
//! The same archive always produces the same coordinates. Every ordering step
//! breaks ties by conversation id, and no step consults the clock or a random
//! source. A layout that reshuffles between runs cannot be diffed, cached, or
//! trusted by anything downstream.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::Path;

use clap::Parser;
use colored::*;
use serde_json::{json, Value};

use crate::schema::lineage::Lineage;

#[derive(Parser, Debug)]
pub struct GraphArgs {
    /// Write to a file instead of stdout
    #[arg(long)]
    pub out: Option<String>,

    /// Omit computed x/y coordinates, keeping layers and edges
    #[arg(long)]
    pub no_layout: bool,
}

/// Abstract units. A renderer scales these; they are not pixels.
const NODE_W: f64 = 210.0;
const NODE_H: f64 = 54.0;
const COL_GAP: f64 = 34.0;
const ROW_GAP: f64 = 118.0;
const MARGIN: f64 = 48.0;

pub struct Placed {
    pub id: String,
    pub layer: usize,
    pub x: f64,
    pub y: f64,
}

pub fn run_graph(args: GraphArgs) {
    let fur_dir = Path::new(".fur");

    if !fur_dir.join("threads").exists() {
        eprintln!("🚨 .fur/ not found. Run `fur new` first.");
        return;
    }

    let lineage = match Lineage::load(fur_dir) {
        Ok(l) => l,
        Err(e) => return eprintln!("❌ {}", e),
    };

    if lineage.ids().is_empty() {
        eprintln!("📭 No conversations yet.");
        return;
    }

    let payload = build(fur_dir, &lineage, !args.no_layout);
    let text = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string());

    let Some(path) = args.out else {
        println!("{}", text);
        return;
    };

    match fs::write(&path, format!("{}\n", text)) {
        Ok(_) => println!(
            "{}",
            format!(
                "🌐 Lineage: {} conversation(s) → {}",
                lineage.ids().len(),
                path
            )
            .bright_green()
            .bold()
        ),
        Err(e) => eprintln!("❌ Could not write {}: {}", path, e),
    }
}

/// Assemble the export document.
pub fn build(fur_dir: &Path, lineage: &Lineage, with_layout: bool) -> Value {
    let placed = layout(lineage);
    let meta = read_metadata(fur_dir, lineage);

    let nodes: Vec<Value> = placed
        .iter()
        .map(|node| {
            let mut entry = json!({
                "id": node.id,
                "title": lineage.title(&node.id),
                "local": lineage.is_local(&node.id),
                "layer": node.layer,
                "parents": lineage.all_parents(&node.id),
                "children": lineage.all_children(&node.id)
            });

            if with_layout {
                entry["x"] = json!(node.x);
                entry["y"] = json!(node.y);
            }

            if let Some(info) = meta.get(node.id.as_str()) {
                entry["created_at"] = info.created.clone();
                entry["updated_at"] = info.updated.clone();
                entry["messages"] = json!(info.messages);
                entry["tags"] = info.tags.clone();
            }

            entry
        })
        .collect();

    let mut edges: Vec<(String, String)> = Vec::new();
    for node in &placed {
        for child in lineage.all_children(&node.id) {
            edges.push((node.id.clone(), child));
        }
    }
    edges.sort();
    edges.dedup();

    let edges: Vec<Value> = edges
        .into_iter()
        .map(|(from, to)| {
            json!({
                "from": from,
                "to": to,
                // False when either end is absent: the edge is real, but one
                // side of it is not in this project.
                "resolved": lineage.is_local(&from) && lineage.is_local(&to)
            })
        })
        .collect();

    let mut payload = json!({
        "graph_schema": "fur.lineage.v1",
        "nodes": nodes,
        "edges": edges,
        "dangling": lineage.dangling(),
        "asymmetric": lineage
            .asymmetric()
            .into_iter()
            .map(|(parent, child)| json!({ "parent": parent, "child": child }))
            .collect::<Vec<_>>()
    });

    if with_layout {
        let width = placed.iter().map(|n| n.x + NODE_W).fold(0.0, f64::max) + MARGIN;
        let height = placed.iter().map(|n| n.y + NODE_H).fold(0.0, f64::max) + MARGIN;

        payload["canvas"] = json!({
            "width": width,
            "height": height,
            "node_width": NODE_W,
            "node_height": NODE_H
        });
    }

    payload
}

struct Meta {
    created: Value,
    updated: Value,
    messages: usize,
    tags: Value,
}

/// Per-conversation detail the lineage index does not carry.
fn read_metadata<'a>(fur_dir: &Path, lineage: &'a Lineage) -> HashMap<String, Meta> {
    let mut out = HashMap::new();

    for id in lineage.ids() {
        let path = fur_dir.join("threads").join(format!("{}.json", id));
        let Some(content) = crate::security::io::read_text_file(&path) else {
            continue;
        };
        let Ok(convo) = serde_json::from_str::<Value>(&content) else {
            continue;
        };

        let messages = convo["messages"].as_array().map(|a| a.len()).unwrap_or(0);

        // `updated_at` is derived from the last message when the spine is
        // written, so it is recomputed here rather than read from `.fur/`,
        // which does not store it.
        let updated = last_message_ts(fur_dir, &convo);

        out.insert(
            id.clone(),
            Meta {
                created: convo["created_at"].clone(),
                updated,
                messages,
                tags: convo["tags"].clone(),
            },
        );
    }

    out
}

fn last_message_ts(fur_dir: &Path, convo: &Value) -> Value {
    let Some(ids) = convo["messages"].as_array() else {
        return Value::Null;
    };

    let mut best: Option<String> = None;

    for mid in ids.iter().filter_map(|v| v.as_str()) {
        let path = fur_dir.join("messages").join(format!("{}.json", mid));
        let Some(content) = crate::security::io::read_text_file(&path) else {
            continue;
        };
        let Ok(msg) = serde_json::from_str::<Value>(&content) else {
            continue;
        };
        let Some(ts) = msg["timestamp"].as_str() else {
            continue;
        };

        let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(ts) else {
            continue;
        };

        let better = best
            .as_deref()
            .and_then(|b| chrono::DateTime::parse_from_rfc3339(b).ok())
            .map(|current| parsed > current)
            .unwrap_or(true);

        if better {
            best = Some(ts.to_string());
        }
    }

    match best {
        Some(ts) => json!(ts),
        None => Value::Null,
    }
}

// ======================================================
//  LAYOUT
// ======================================================

/// Assign every node a layer and a position within it.
pub fn layout(lineage: &Lineage) -> Vec<Placed> {
    // Local conversations plus the absent ones they reference, so a renderer
    // can draw an imported conversation's missing sources rather than dropping
    // edges that lead nowhere.
    let mut ids: Vec<String> = lineage.ids();
    ids.extend(lineage.dangling());
    ids.sort();
    ids.dedup();

    let layers = assign_layers(lineage, &ids);
    let depth = layers.values().copied().max().unwrap_or(0) + 1;

    let mut rows: Vec<Vec<String>> = vec![Vec::new(); depth];
    for id in &ids {
        rows[layers[id]].push(id.clone());
    }
    for row in rows.iter_mut() {
        row.sort();
    }

    order_rows(lineage, &mut rows);

    let widest = rows.iter().map(|r| r.len()).max().unwrap_or(1) as f64;
    let canvas_w = widest * NODE_W + (widest - 1.0).max(0.0) * COL_GAP;

    let mut out = Vec::new();

    for (layer, row) in rows.iter().enumerate() {
        let count = row.len() as f64;
        let row_w = count * NODE_W + (count - 1.0).max(0.0) * COL_GAP;
        let start = MARGIN + (canvas_w - row_w) / 2.0;

        for (i, id) in row.iter().enumerate() {
            out.push(Placed {
                id: id.clone(),
                layer,
                x: start + i as f64 * (NODE_W + COL_GAP),
                y: MARGIN + layer as f64 * (NODE_H + ROW_GAP),
            });
        }
    }

    out
}

/// `layer(n) = 0` when nothing points at `n`, else one past its deepest parent.
///
/// The in-progress guard makes an imported cycle terminate: a back edge
/// contributes nothing rather than recursing forever.
fn assign_layers(lineage: &Lineage, ids: &[String]) -> BTreeMap<String, usize> {
    let mut layers: BTreeMap<String, usize> = BTreeMap::new();
    let mut visiting: HashSet<String> = HashSet::new();

    for id in ids {
        depth_of(lineage, id, &mut layers, &mut visiting);
    }

    layers
}

fn depth_of(
    lineage: &Lineage,
    id: &str,
    layers: &mut BTreeMap<String, usize>,
    visiting: &mut HashSet<String>,
) -> usize {
    if let Some(&known) = layers.get(id) {
        return known;
    }
    if !visiting.insert(id.to_string()) {
        return 0;
    }

    let mut best = 0usize;
    for parent in lineage.all_parents(id) {
        best = best.max(depth_of(lineage, &parent, layers, visiting) + 1);
    }

    visiting.remove(id);
    layers.insert(id.to_string(), best);
    best
}

/// Reduce edge crossings by pulling each node toward the average position of
/// its parents. Two passes is enough at this scale, and ties break by id so the
/// result stays deterministic.
fn order_rows(lineage: &Lineage, rows: &mut [Vec<String>]) {
    for _ in 0..2 {
        for layer in 1..rows.len() {
            let above: HashMap<&str, usize> = rows[layer - 1]
                .iter()
                .enumerate()
                .map(|(i, id)| (id.as_str(), i))
                .collect();

            let mut keyed: Vec<(f64, String)> = rows[layer]
                .iter()
                .map(|id| {
                    let positions: Vec<usize> = lineage
                        .all_parents(id)
                        .iter()
                        .filter_map(|p| above.get(p.as_str()).copied())
                        .collect();

                    let key = if positions.is_empty() {
                        f64::MAX
                    } else {
                        positions.iter().sum::<usize>() as f64 / positions.len() as f64
                    };

                    (key, id.clone())
                })
                .collect();

            keyed.sort_by(|a, b| {
                a.0.partial_cmp(&b.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.1.cmp(&b.1))
            });

            rows[layer] = keyed.into_iter().map(|(_, id)| id).collect();
        }
    }
}