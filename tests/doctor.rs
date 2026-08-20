//! Lineage diagnostics: report, never repair.
use std::fs;

use fur_cli::commands::doctor::{find_loops_for_test, Lineage};

const A: &str = "aaaaaaaa-1111-4111-8111-111111111111";
const B: &str = "bbbbbbbb-2222-4222-8222-222222222222";
const C: &str = "cccccccc-3333-4333-8333-333333333333";

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
                id, title, quoted(parents), quoted(children)
            ),
        )
        .unwrap();
    }

    Lineage::load(&root.join(".fur")).unwrap();
    root
}

fn loops(root: &std::path::Path) -> Vec<Vec<String>> {
    let lineage = Lineage::load(&root.join(".fur")).unwrap();
    find_loops_for_test(&lineage)
}

#[test]
fn a_clean_archive_has_no_loops() {
    let root = boot(
        "fur_doc_clean",
        &[
            (A, "Top", vec![], vec![B]),
            (B, "Below", vec![A], vec![]),
        ],
    );
    assert!(loops(&root).is_empty());
}

#[test]
fn a_two_node_loop_is_found_once() {
    let root = boot(
        "fur_doc_loop2",
        &[
            (A, "First", vec![B], vec![B]),
            (B, "Second", vec![A], vec![A]),
        ],
    );

    let found = loops(&root);
    assert_eq!(found.len(), 1, "got: {:?}", found);
    assert_eq!(found[0], vec![A.to_string(), B.to_string()]);
}

#[test]
fn a_three_node_loop_reports_its_path() {
    let root = boot(
        "fur_doc_loop3",
        &[
            (A, "One", vec![C], vec![B]),
            (B, "Two", vec![A], vec![C]),
            (C, "Three", vec![B], vec![A]),
        ],
    );

    let found = loops(&root);
    assert_eq!(found.len(), 1, "got: {:?}", found);
    assert_eq!(found[0], vec![A.to_string(), B.to_string(), C.to_string()]);
}

#[test]
fn a_diamond_is_not_a_loop() {
    let root = boot(
        "fur_doc_diamond",
        &[
            (A, "Top", vec![], vec![B, C]),
            (B, "Left", vec![A], vec![]),
            (C, "Right", vec![A], vec![]),
        ],
    );
    assert!(loops(&root).is_empty());
}