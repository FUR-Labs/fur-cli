//! Provenance documents: dependency order, diamonds once, absent sources named.
use std::fs;

use fur_cli::commands::provenance::{render, Options, Scope};

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
    fs::write(root.join(".fur/avatars.json"), r#"{"main":"andrew"}"#).unwrap();

    for (id, title, parents, children) in threads {
        let mid = format!("{}-m", &id[..8]);
        fs::write(
            root.join(format!(".fur/messages/{}.json", mid)),
            format!(
                r#"{{"id":"{}","avatar":"andrew","timestamp":"2026-08-17T00:00:00Z","text":"body of {}","markdown":null,"attachment":null}}"#,
                mid, title
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
                id,
                title,
                mid,
                quoted(parents),
                quoted(children)
            ),
        )
        .unwrap();
    }

    root
}

fn opts(scope: Scope) -> Options {
    Options {
        scope,
        contents: false,
    }
}

fn position(body: &str, needle: &str) -> usize {
    body.find(needle)
        .unwrap_or_else(|| panic!("'{}' not in document:\n{}", needle, body))
}

#[test]
fn ancestors_come_before_the_record_itself() {
    let root = boot(
        "fur_prov_order",
        &[
            (A, "Plan", vec![], vec![B]),
            (B, "Weak Points", vec![A], vec![]),
        ],
    );

    let (body, count) = render(&root.join(".fur"), B, &opts(Scope::Ancestors)).unwrap();

    assert_eq!(count, 2);
    assert!(position(&body, "## Plan") < position(&body, "## Weak Points"));
    assert!(body.contains("body of Plan"));
}

#[test]
fn a_diamond_includes_each_conversation_once() {
    let root = boot(
        "fur_prov_diamond",
        &[
            (A, "Top", vec![], vec![B, C]),
            (B, "Left", vec![A], vec![D]),
            (C, "Right", vec![A], vec![D]),
            (D, "Bottom", vec![B, C], vec![]),
        ],
    );

    let (body, count) = render(&root.join(".fur"), D, &opts(Scope::Ancestors)).unwrap();

    assert_eq!(count, 4);
    assert_eq!(body.matches("body of Top").count(), 1);
    assert!(position(&body, "## Top") < position(&body, "## Left"));
    assert!(position(&body, "## Left") < position(&body, "## Bottom"));
}

#[test]
fn descendants_are_excluded_unless_full() {
    let root = boot(
        "fur_prov_scope",
        &[
            (A, "Plan", vec![], vec![B]),
            (B, "Weak Points", vec![A], vec![]),
        ],
    );
    let fur = root.join(".fur");

    let (up, _) = render(&fur, A, &opts(Scope::Ancestors)).unwrap();
    assert!(!up.contains("body of Weak Points"));

    let (both, count) = render(&fur, A, &opts(Scope::Full)).unwrap();
    assert_eq!(count, 2);
    assert!(both.contains("body of Weak Points"));
}

#[test]
fn full_scope_keeps_descendants_in_dependency_order() {
    // A diamond read from the top: Bottom draws on both arms, so it must come
    // after both — which a plain descent-order walk gets wrong.
    let root = boot(
        "fur_prov_full_order",
        &[
            (A, "Top", vec![], vec![B, C]),
            (B, "Left", vec![A], vec![D]),
            (C, "Right", vec![A], vec![D]),
            (D, "Bottom", vec![B, C], vec![]),
        ],
    );

    let (body, count) = render(&root.join(".fur"), A, &opts(Scope::Full)).unwrap();

    assert_eq!(count, 4);
    assert_eq!(body.matches("body of Bottom").count(), 1);
    assert!(position(&body, "## Top") < position(&body, "## Left"));
    assert!(position(&body, "## Left") < position(&body, "## Bottom"));
    assert!(position(&body, "## Right") < position(&body, "## Bottom"));
}

#[test]
fn an_absent_source_is_named_rather_than_silently_dropped() {
    let root = boot("fur_prov_absent", &[(A, "Imported", vec![ABSENT], vec![])]);

    let (body, count) = render(&root.join(".fur"), A, &opts(Scope::Ancestors)).unwrap();

    assert_eq!(count, 1);
    assert!(body.contains("## Not included"));
    assert!(body.contains(ABSENT));
}

#[test]
fn an_imported_cycle_terminates() {
    let root = boot(
        "fur_prov_cycle",
        &[
            (A, "First", vec![B], vec![B]),
            (B, "Second", vec![A], vec![A]),
        ],
    );

    let (body, count) = render(&root.join(".fur"), A, &opts(Scope::Ancestors)).unwrap();

    assert_eq!(count, 2);
    assert_eq!(body.matches("body of First").count(), 1);
}

#[test]
fn a_lone_conversation_has_no_lineage_map() {
    let root = boot("fur_prov_lone", &[(A, "Alone", vec![], vec![])]);
    let (body, count) = render(&root.join(".fur"), A, &opts(Scope::Ancestors)).unwrap();

    assert_eq!(count, 1);
    assert!(!body.contains("## Lineage"));
    assert!(body.contains("body of Alone"));
}

#[test]
fn a_conversation_outside_the_project_is_refused() {
    let root = boot("fur_prov_missing", &[(A, "Here", vec![], vec![])]);
    let err = render(&root.join(".fur"), ABSENT, &opts(Scope::Ancestors)).unwrap_err();
    assert!(err.contains("not in this project"), "got: {}", err);
}