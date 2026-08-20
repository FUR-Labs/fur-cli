//! Lineage export: deterministic layout, ghost nodes, cycle-safe layering.
use std::fs;

use fur_cli::commands::graph::build;
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
    fs::create_dir_all(root.join(".fur/messages")).unwrap();

    for (id, title, parents, children) in threads {
        let mid = format!("{}-m", &id[..8]);
        fs::write(
            root.join(format!(".fur/messages/{}.json", mid)),
            format!(
                r#"{{"id":"{}","avatar":"andrew","timestamp":"2026-08-17T04:00:00Z","text":"x","markdown":null,"attachment":null}}"#,
                mid
            ),
        )
        .unwrap();

        let quoted = |v: &Vec<&str>| {
            v.iter()
                .map(|s| format!("\"{}\"", s))
                .collect::<Vec<_>>()
                .join(",")
        };
        fs::write(
            root.join(format!(".fur/threads/{}.json", id)),
            format!(
                r#"{{"id":"{}","title":"{}","created_at":"2026-08-17T00:00:00Z","messages":["{}"],"tags":[],"parents":[{}],"children":[{}],"schema_version":"0.3"}}"#,
                id, title, mid, quoted(parents), quoted(children)
            ),
        )
        .unwrap();
    }

    root
}

fn export(root: &std::path::Path) -> serde_json::Value {
    let fur = root.join(".fur");
    build(&fur, &Lineage::load(&fur).unwrap(), true)
}

fn node<'a>(payload: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    payload["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == id)
        .unwrap_or_else(|| panic!("{} not in export", id))
}

#[test]
fn layers_increase_with_depth() {
    let root = boot(
        "fur_graph_layers",
        &[
            (A, "Top", vec![], vec![B]),
            (B, "Middle", vec![A], vec![C]),
            (C, "Bottom", vec![B], vec![]),
        ],
    );
    let payload = export(&root);

    assert_eq!(node(&payload, A)["layer"], 0);
    assert_eq!(node(&payload, B)["layer"], 1);
    assert_eq!(node(&payload, C)["layer"], 2);
    assert!(node(&payload, C)["y"].as_f64().unwrap() > node(&payload, A)["y"].as_f64().unwrap());
}

#[test]
fn a_diamond_puts_the_tip_below_both_arms() {
    let root = boot(
        "fur_graph_diamond",
        &[
            (A, "Top", vec![], vec![B, C]),
            (B, "Left", vec![A], vec![D]),
            (C, "Right", vec![A], vec![D]),
            (D, "Bottom", vec![B, C], vec![]),
        ],
    );
    let payload = export(&root);

    assert_eq!(node(&payload, B)["layer"], node(&payload, C)["layer"]);
    assert_eq!(node(&payload, D)["layer"], 2);
    assert_eq!(payload["edges"].as_array().unwrap().len(), 4);
}

#[test]
fn the_layout_is_deterministic() {
    let root = boot(
        "fur_graph_stable",
        &[
            (A, "Top", vec![], vec![B, C]),
            (B, "Left", vec![A], vec![]),
            (C, "Right", vec![A], vec![]),
        ],
    );

    assert_eq!(export(&root), export(&root));
}

#[test]
fn an_absent_source_becomes_an_unresolved_edge() {
    let root = boot("fur_graph_ghost", &[(A, "Imported", vec![ABSENT], vec![])]);
    let payload = export(&root);

    assert_eq!(node(&payload, ABSENT)["local"], false);
    assert_eq!(node(&payload, ABSENT)["title"], serde_json::Value::Null);
    assert_eq!(payload["dangling"].as_array().unwrap().len(), 1);

    let edge = &payload["edges"].as_array().unwrap()[0];
    assert_eq!(edge["resolved"], false);
    assert_eq!(edge["from"], ABSENT);
}

#[test]
fn an_imported_cycle_does_not_hang_the_layering() {
    let root = boot(
        "fur_graph_cycle",
        &[
            (A, "First", vec![B], vec![B]),
            (B, "Second", vec![A], vec![A]),
        ],
    );
    let payload = export(&root);

    assert_eq!(payload["nodes"].as_array().unwrap().len(), 2);
    assert!(node(&payload, A)["layer"].as_u64().is_some());
}

#[test]
fn metadata_travels_with_each_node() {
    let root = boot("fur_graph_meta", &[(A, "Solo", vec![], vec![])]);
    let payload = export(&root);
    let n = node(&payload, A);

    assert_eq!(n["messages"], 1);
    assert_eq!(n["created_at"], "2026-08-17T00:00:00Z");
    assert_eq!(n["updated_at"], "2026-08-17T04:00:00Z");
    assert_eq!(n["title"], "Solo");
}

#[test]
fn a_one_sided_claim_is_reported() {
    let root = boot(
        "fur_graph_asym",
        &[(A, "Plan", vec![], vec![B]), (B, "Weak", vec![], vec![])],
    );
    let payload = export(&root);

    let asym = payload["asymmetric"].as_array().unwrap();
    assert_eq!(asym.len(), 1);
    assert_eq!(asym[0]["parent"], A);
}