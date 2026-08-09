//! Phase A gate: does a conversation survive the trip through Markdown?
//!
//! Nothing here touches `.fur/`. If these fail, the document format is wrong
//! and the storage inversion is not safe to attempt yet.

use fur_cli::schema::document::{parse, serialize, FurDocument, FurMessage};

/// A jot-heavy conversation — the case where `chats/` currently recovers
/// nothing, and the one most likely to strain the format.
fn jot_heavy() -> FurDocument {
    let mut doc = FurDocument::new(
        "8f0c4a2e-1b3d-4f5a-9c7e-2d8b6a1f0e33",
        "Symbolic regression scratchpad",
        "2026-08-08T18:11:40-05:00",
    );
    doc.tags = vec!["research".to_string(), "deep-learning".to_string()];

    let jots = [
        ("andrew", "Symbolic regression tests using KAN"),
        ("gpt5", "What basis are you using for the spline layer?"),
        ("andrew", "B-splines, order 3. Grid is adaptive."),
        ("gpt5", "Then watch the grid update schedule during fine-tuning."),
        ("andrew", "noted"),
        ("andrew", "actually — it diverges past epoch 40"),
        ("gpt5", "Freeze the grid after warmup and re-run."),
        ("andrew", "that fixed it"),
    ];

    for (i, (avatar, text)) in jots.iter().enumerate() {
        doc.messages.push(
            FurMessage::new(
                format!("00000000-0000-0000-0000-{:012}", i),
                *avatar,
                format!("2026-08-08T18:{:02}:00-05:00", 11 + i),
            )
            .with_body(*text),
        );
    }

    doc
}

/// A conversation whose long-form bodies live in sibling files.
fn long_form() -> FurDocument {
    let mut doc = FurDocument::new(
        "1c9d55b0-77aa-4e31-9f02-aa8c1d3e7710",
        "Schema Proposal Changes FUR",
        "2026-08-08T18:11:40-05:00",
    );
    doc.updated_at = Some("2026-08-08T19:02:15-05:00".to_string());
    doc.tags = vec!["schema".to_string(), "architecture".to_string()];

    doc.messages = vec![
        FurMessage::new("3a7f9c21", "andrew", "2026-08-08T18:11:40-05:00")
            .with_body("I think that I may have a better idea..."),
        FurMessage::new("b2e14d80", "gpt5", "2026-08-08T18:11:57-05:00").with_link(
            "CHAT-20260808-181158.md",
            Some("9f2c1e4b7a0d6f3c8e5b2a91d4f7c0e3b6a9d2f5c8e1b4a7d0f3c6e9b2a5d8f1".to_string()),
        ),
        FurMessage::new("c17aa903", "andrew", "2026-08-08T18:14:02-05:00").with_body("badass"),
    ];

    doc
}

fn assert_round_trips(doc: &FurDocument) {
    let once = serialize(doc);

    let parsed = match parse(&once) {
        Ok(d) => d,
        Err(e) => panic!("parse failed: {}\n---\n{}", e, once),
    };

    assert_eq!(
        *doc, parsed,
        "structure changed on the way back:\n{}",
        once
    );
    assert_eq!(
        once,
        serialize(&parsed),
        "serialization is not idempotent:\n{}",
        once
    );
}

#[test]
fn jot_heavy_conversation_round_trips() {
    assert_round_trips(&jot_heavy());
}

#[test]
fn long_form_conversation_round_trips() {
    assert_round_trips(&long_form());
}

#[test]
fn empty_conversation_round_trips() {
    let doc = FurDocument::new("empty-cid", "Nothing here yet", "2026-08-08T18:11:40-05:00");
    assert_round_trips(&doc);
}

#[test]
fn multiline_markdown_body_survives() {
    let mut doc = FurDocument::new("md-cid", "Markdown body", "2026-08-08T18:11:40-05:00");
    doc.messages = vec![FurMessage::new("m1", "gpt5", "2026-08-08T18:11:40-05:00").with_body(
        "# Heading\n\n- one\n- two\n\n```rust\nfn main() {}\n```\n\nTrailing paragraph.",
    )];
    assert_round_trips(&doc);
}

#[test]
fn unicode_and_emoji_survive() {
    let mut doc = FurDocument::new("uni-cid", "Ünïcödé — 🦊", "2026-08-08T18:11:40-05:00");
    doc.messages = vec![
        FurMessage::new("m1", "andrew", "2026-08-08T18:11:40-05:00").with_body("café ☕ 数学"),
    ];
    assert_round_trips(&doc);
}