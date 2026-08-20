//! Conversation lineage, derived from `.fur/threads/`.
//!
//! # Assertions, not records
//!
//! `parents` and `children` are independent claims. A holds "B is my child"
//! and B holds "A is my parent"; either can exist without the other, because
//! one of them may have been imported from a registry without the other. The
//! traversal graph is therefore the *union* of every claim in the project, and
//! an edge asserted from one side only is a real edge.
//!
//! The raw claims are kept alongside the union, because the union deliberately
//! erases which side made the claim — and that is exactly what asymmetry
//! detection needs.
//!
//! # Cycles are data, not errors
//!
//! Nothing here rejects a cycle. `fur link` refuses to create one locally, but
//! two people can independently publish A→B and B→A and the cycle only exists
//! once both land in one archive. Every walk carries a visited set — which it
//! needs for diamonds regardless — so a node reached twice is emitted once and
//! flagged, never expanded again.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::Path;

use serde_json::Value;

/// One row of a rendered lineage tree.
pub struct TreeEntry {
    pub id: String,
    /// Nesting level; 0 is a root.
    pub depth: usize,
    /// Already shown higher up — the second arm of a diamond. Not expanded.
    pub repeat: bool,
    /// Has parents, none of which are in this project. Sits at the margin
    /// because there is nothing local to nest it under.
    pub orphan_parent: bool,
}

pub struct Lineage {
    titles: BTreeMap<String, String>,
    parents: BTreeMap<String, BTreeSet<String>>,
    children: BTreeMap<String, BTreeSet<String>>,
    claimed_parents: BTreeMap<String, BTreeSet<String>>,
    claimed_children: BTreeMap<String, BTreeSet<String>>,
}

// `title`, `asymmetric` and `dangling` are the reporting surface: `fur doctor`
// and the visual export read them, the conversation table does not.
#[allow(dead_code)]
impl Lineage {
    /// Build from `.fur/threads/`, reading the directory rather than the index
    /// so a stale `index.json` cannot hide a conversation.
    pub fn load(fur_dir: &Path) -> Result<Lineage, String> {
        let threads = fur_dir.join("threads");
        let entries = fs::read_dir(&threads)
            .map_err(|e| format!("cannot read {}: {}", threads.display(), e))?;

        let mut lineage = Lineage {
            titles: BTreeMap::new(),
            parents: BTreeMap::new(),
            children: BTreeMap::new(),
            claimed_parents: BTreeMap::new(),
            claimed_children: BTreeMap::new(),
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }

            let Some(content) = crate::security::io::read_text_file(&path) else {
                continue;
            };
            let Ok(convo) = serde_json::from_str::<Value>(&content) else {
                continue;
            };
            let Some(id) = convo["id"].as_str() else {
                continue;
            };

            lineage.titles.insert(
                id.to_string(),
                convo["title"].as_str().unwrap_or("Untitled").to_string(),
            );

            for parent in string_list(&convo, "parents") {
                lineage
                    .claimed_parents
                    .entry(id.to_string())
                    .or_default()
                    .insert(parent.clone());
                lineage.add_edge(&parent, id);
            }
            for child in string_list(&convo, "children") {
                lineage
                    .claimed_children
                    .entry(id.to_string())
                    .or_default()
                    .insert(child.clone());
                lineage.add_edge(id, &child);
            }
        }

        Ok(lineage)
    }

    /// Record one edge in both directions, so a claim from either end is
    /// visible from both.
    fn add_edge(&mut self, parent: &str, child: &str) {
        self.children
            .entry(parent.to_string())
            .or_default()
            .insert(child.to_string());
        self.parents
            .entry(child.to_string())
            .or_default()
            .insert(parent.to_string());
    }

    pub fn is_empty(&self) -> bool {
        self.parents.is_empty() && self.children.is_empty()
    }

    pub fn is_local(&self, id: &str) -> bool {
        self.titles.contains_key(id)
    }

    pub fn title(&self, id: &str) -> Option<&str> {
        self.titles.get(id).map(|s| s.as_str())
    }

    /// Would linking `parent` → `child` close a loop? True when `child` can
    /// already reach `parent` by following children.
    pub fn would_cycle(&self, parent: &str, child: &str) -> bool {
        let mut seen: HashSet<&str> = HashSet::new();
        let mut stack = vec![child];

        while let Some(id) = stack.pop() {
            if id == parent {
                return true;
            }
            if !seen.insert(id) {
                continue;
            }
            if let Some(kids) = self.children.get(id) {
                stack.extend(kids.iter().map(|s| s.as_str()));
            }
        }

        false
    }

    /// Edges between two local conversations asserted from one side only.
    ///
    /// Compares the raw claims, never the union — in the union a one-sided
    /// claim has already been mirrored for traversal, so checking it there
    /// could never report anything. Edges whose far end is absent are skipped:
    /// there is no local file that could hold the mirror.
    pub fn asymmetric(&self) -> Vec<(String, String)> {
        let mut out = BTreeSet::new();

        for (parent, kids) in &self.claimed_children {
            for child in kids {
                if !self.is_local(parent) || !self.is_local(child) {
                    continue;
                }
                let mirrored = self
                    .claimed_parents
                    .get(child)
                    .map(|set| set.contains(parent))
                    .unwrap_or(false);
                if !mirrored {
                    out.insert((parent.clone(), child.clone()));
                }
            }
        }

        for (child, parents) in &self.claimed_parents {
            for parent in parents {
                if !self.is_local(parent) || !self.is_local(child) {
                    continue;
                }
                let mirrored = self
                    .claimed_children
                    .get(parent)
                    .map(|set| set.contains(child))
                    .unwrap_or(false);
                if !mirrored {
                    out.insert((parent.clone(), child.clone()));
                }
            }
        }

        out.into_iter().collect()
    }

    /// Conversation ids that nothing local holds as a parent.
    pub fn dangling(&self) -> Vec<String> {
        self.parents
            .keys()
            .chain(self.children.keys())
            .chain(self.parents.values().flatten())
            .chain(self.children.values().flatten())
            .filter(|id| !self.is_local(id))
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// Flatten the project into display order: roots first, children nested
    /// beneath them, siblings in the order given by `order`.
    ///
    /// `order` is the caller's preferred sequence (newest first, typically);
    /// it decides which root comes first and how siblings sort, while the
    /// lineage decides the nesting. Conversations no root reaches — every
    /// member of a cycle — are appended at the margin so nothing disappears.
    pub fn forest(&self, order: &[String]) -> Vec<TreeEntry> {
        let mut out = Vec::new();
        let mut visited: HashSet<String> = HashSet::new();

        for id in order {
            if !self.is_local(id) || self.has_local_parent(id) {
                continue;
            }
            self.descend(id, 0, order, &mut visited, &mut out);
        }

        for id in order {
            if self.is_local(id) && !visited.contains(id) {
                self.descend(id, 0, order, &mut visited, &mut out);
            }
        }

        out
    }

    fn descend(
        &self,
        id: &str,
        depth: usize,
        order: &[String],
        visited: &mut HashSet<String>,
        out: &mut Vec<TreeEntry>,
    ) {
        let repeat = !visited.insert(id.to_string());

        out.push(TreeEntry {
            id: id.to_string(),
            depth,
            repeat,
            orphan_parent: depth == 0 && self.has_only_absent_parents(id),
        });

        if repeat {
            return;
        }

        for child in self.ordered_local_children(id, order) {
            self.descend(&child, depth + 1, order, visited, out);
        }
    }

    fn has_local_parent(&self, id: &str) -> bool {
        self.parents
            .get(id)
            .map(|set| set.iter().any(|p| self.is_local(p)))
            .unwrap_or(false)
    }

    fn has_only_absent_parents(&self, id: &str) -> bool {
        match self.parents.get(id) {
            Some(set) if !set.is_empty() => set.iter().all(|p| !self.is_local(p)),
            _ => false,
        }
    }

    fn ordered_local_children(&self, id: &str, order: &[String]) -> Vec<String> {
        let Some(kids) = self.children.get(id) else {
            return Vec::new();
        };

        let mut out: Vec<String> = kids.iter().filter(|k| self.is_local(k)).cloned().collect();

        out.sort_by_key(|k| order.iter().position(|o| o == k).unwrap_or(usize::MAX));
        out
    }
}

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