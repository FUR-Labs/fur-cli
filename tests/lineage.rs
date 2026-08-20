//! Lineage forest: unions assertions, nests children, terminates on cycles.
use std::fs;
use std::path::Path;

use fur_cli::schema::lineage::Lineage;

const A: &str = "aaaaaaaa-1111-4111-8111-111111111111";
const B: &str = "bbbbbbbb-2222-4222-8222-222222222222";
const C: &str = "cccccccc-3333-4333-8333-333333333333";
const D: &str = "dddddddd-4444-4444-8444-444444444444";
const ABSENT: &str = "ffffffff-9999-4999-8999-999999999999";

fn boot(name: &str, threads: &[(&str, &str, Vec<&str>, Vec<&str>)]) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join(".fur/threads")).unwrap();

    for (id, title, parents, children) in threads {
        let quoted = |v: &Vec<&str>| {
            v.iter()
                .map(|s| format!("\"{}\"", s))
                .collect::<Vec<_>>()
                .join(",")
        };
        fs::write(
            root.join(format!(".fur/threads/{}.json", id)),
            format!(
                r#"{{"id":"{}","title":"{}","created_at":"2026-08-17T00:00:00Z","messages":[],"tags":[],"parents":[{}],"children":[{}],"schema_version":"0.3"}}"#,
                id,
                title,
                quoted(parents),
                quoted(children)
            ),
        )
        .unwrap();
    }

    root
}

fn load(root: &Path) -> Lineage {
    Lineage::load(&root.join(".fur")).unwrap()
}

fn order() -> Vec<String> {
    vec![A.to_string(), B.to_string(), C.to_string(), D.to_string()]
}

#[test]
fn children_nest_under_their_parent() {
    let root = boot(
        "fur_lin_nest",
        &[
            (A, "Plan", vec![], vec![B]),
            (B, "Weak Points", vec![A], vec![]),
        ],
    );
    let forest = load(&root).forest(&order());

    assert_eq!(forest.len(), 2);
    assert_eq!(forest[0].id, A);
    assert_eq!(forest[0].depth, 0);
    assert_eq!(forest[1].id, B);
    assert_eq!(forest[1].depth, 1);
}

#[test]
fn an_edge_asserted_from_one_side_still_nests() {
    // A claims B as a child; B says nothing.
    let root = boot(
        "fur_lin_union",
        &[(A, "Plan", vec![], vec![B]), (B, "Weak", vec![], vec![])],
    );
    let lineage = load(&root);

    assert_eq!(lineage.forest(&order())[1].depth, 1);
    assert_eq!(lineage.asymmetric(), vec![(A.to_string(), B.to_string())]);
}

#[test]
fn a_diamond_appears_twice_but_expands_once() {
    let root = boot(
        "fur_lin_diamond",
        &[
            (A, "Top", vec![], vec![B, C]),
            (B, "Left", vec![A], vec![D]),
            (C, "Right", vec![A], vec![D]),
            (D, "Bottom", vec![B, C], vec![]),
        ],
    );
    let forest = load(&root).forest(&order());

    let bottoms: Vec<&_> = forest.iter().filter(|e| e.id == D).collect();
    assert_eq!(bottoms.len(), 2);
    assert_eq!(bottoms.iter().filter(|e| e.repeat).count(), 1);
}

#[test]
fn a_cycle_terminates_and_nothing_vanishes() {
    let root = boot(
        "fur_lin_cycle",
        &[
            (A, "First", vec![B], vec![B]),
            (B, "Second", vec![A], vec![A]),
        ],
    );
    let forest = load(&root).forest(&order());

    // Neither is a root, so both surface via the unreached sweep.
    assert!(forest.iter().any(|e| e.id == A));
    assert!(forest.iter().any(|e| e.id == B));
    assert!(forest.iter().any(|e| e.repeat));
}

#[test]
fn an_absent_parent_leaves_the_conversation_at_the_margin() {
    let root = boot("fur_lin_orphan", &[(A, "Imported", vec![ABSENT], vec![])]);
    let forest = load(&root).forest(&order());

    assert_eq!(forest.len(), 1);
    assert_eq!(forest[0].depth, 0);
    assert!(forest[0].orphan_parent);
}

#[test]
fn unlinked_conversations_stay_flat() {
    let root = boot(
        "fur_lin_flat",
        &[(A, "One", vec![], vec![]), (B, "Two", vec![], vec![])],
    );
    let lineage = load(&root);

    assert!(lineage.is_empty());
    assert!(lineage.forest(&order()).iter().all(|e| e.depth == 0));
}

#[test]
fn a_loop_is_predicted_before_it_is_written() {
    let root = boot(
        "fur_lin_would_cycle",
        &[
            (A, "One", vec![], vec![B]),
            (B, "Two", vec![A], vec![]),
        ],
    );
    let lineage = load(&root);

    assert!(lineage.would_cycle(B, A), "B→A closes the existing A→B");
    assert!(!lineage.would_cycle(A, B));
}