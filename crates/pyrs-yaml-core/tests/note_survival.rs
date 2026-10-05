//! Note survival over the committed fuzz corpus.
//!
//! The round-trip tier's oracle is idempotence — `to_yaml(parse(to_yaml(x))) ==
//! to_yaml(x)` — and a document that is stable and merely short a note passes it.
//! Losing a note is therefore invisible to the assertion that has driven every
//! comment fix in this engine, which is how five silent losses (stacked notes,
//! comment-only documents, a folded null key, a consumed merge key and the mapping
//! it consumed) survived behind green tests. This file asserts the other invariant:
//!
//! **every note the reader recorded for an input appears in the emitted document.**
//!
//! It walks `fuzz/seeds/**.seed` — the regression corpus itself, so the set is
//! closed and grows with each fixed crash rather than with someone's imagination.
//! Inputs that are not valid UTF-8 or that the parser (correctly) rejects are
//! skipped: they have no AST, so there is no note to keep.
//!
//! **What this gate cannot see, stated because a green check invites trust in it.**
//! The measure is the note the *reader recorded*, so a note lost during ingest is
//! invisible: the AST never had it, and nothing is then demanded of the output.
//! Proven by mutation, not by argument — making `attach_inline_comment` report
//! success without attaching anything (the defect that dropped `!x # note`, crash-
//! ce106ccc) reddens `a_note_beside_a_property_only_root_survives_and_settles` and
//! leaves this file **green**. An ingest-side gate would have to count comments in
//! the source text, which cannot be done reliably here: a `#` is a comment only
//! outside quotes, outside a block-scalar body and outside a tag token, and the
//! corpus deliberately contains `!###0 …` — a tag whose suffix is `#` characters.
//! So this file is the writer-side half of the invariant and the fuzz tier plus the
//! pinned per-shape tests are the reader-side half; neither is claimed to cover the
//! other.

use std::path::{Path, PathBuf};

use pyrs_yaml_core::ast::CustomNode;
use pyrs_yaml_core::parser::parse;
use pyrs_yaml_core::parser::yaml::Schema;
use pyrs_yaml_core::serializer::to_yaml;

/// Every note text the reader attached anywhere in `node`, in source order.
///
/// Read through `comment()` / `leading_comments()` rather than the raw fields: a
/// note lives in one of two storage conventions (`decor.leading_comments` or a
/// legacy `comment { standalone: true }`) and the accessors normalise across both,
/// which is the whole reason the writers cannot miss a stack.
fn note_texts(node: &CustomNode, out: &mut Vec<String>) {
    if let Some(c) = node.comment() {
        out.push(c.text.to_string());
    }
    for c in node.leading_comments() {
        out.push(c.text.to_string());
    }
    match node {
        CustomNode::Mapping { pairs, .. } => {
            for (key, value) in pairs.iter() {
                note_texts(key, out);
                note_texts(value, out);
            }
        }
        CustomNode::Sequence { items, .. } => {
            for item in items {
                note_texts(item, out);
            }
        }
        CustomNode::Scalar { .. } | CustomNode::Null { .. } | CustomNode::Alias { .. } => {}
    }
}

fn seed_dirs() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds");
    ["yaml_roundtrip", "parse_yaml"]
        .iter()
        .map(|name| root.join(name))
        .filter(|dir| dir.is_dir())
        .collect()
}

#[test]
fn every_note_in_the_seed_corpus_survives_one_emission() {
    let mut checked = 0usize;
    let mut failures = Vec::new();

    for dir in seed_dirs() {
        let mut entries = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("{} must be readable: {e}", dir.display()))
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "seed"))
            .collect::<Vec<_>>();
        entries.sort();
        for path in entries {
            let raw = std::fs::read(&path).expect("seed must be readable");
            let Ok(src) = std::str::from_utf8(&raw) else {
                continue; // rejected before any note exists
            };
            let Ok(node) = parse(src, Schema::Core) else {
                continue; // a typed rejection has no AST to lose notes from
            };
            let mut texts = Vec::new();
            note_texts(&node, &mut texts);
            if texts.is_empty() {
                continue;
            }
            checked += 1;
            let emitted = to_yaml(&node);
            let dropped: Vec<&String> = texts.iter().filter(|t| !emitted.contains(&***t)).collect();
            if !dropped.is_empty() {
                failures.push(format!(
                    "{}: dropped {:?} from {src:?}, emitted {emitted:?}",
                    path.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                    dropped,
                ));
                continue;
            }
            // The same input also has to settle in one emission, which is the
            // assertion the fuzz tier makes by replaying these files: checking it
            // here too turns the corpus into a deterministic gate that runs on every
            // `cargo nextest`, not only where libFuzzer happens to explore.
            let again =
                to_yaml(&parse(&emitted, Schema::Core).unwrap_or_else(|e| {
                    panic!("{}: emitted text must re-parse: {e}", path.display())
                }));
            if again != emitted {
                failures.push(format!(
                    "{}: not a one-round fixed point: {emitted:?} -> {again:?}",
                    path.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "note survival broken over the committed corpus:\n{}",
        failures.join("\n")
    );
    // The corpus must actually cover the invariant: a vacuous pass would be worse
    // than no test, because it looks like coverage. 30 is the measured count of
    // note-bearing YAML seeds minus slack for inputs being retired; a drop below it
    // means the corpus lost its comments, not that the assertion is merely unused.
    // Measured 2026-10-05: 37 of the corpus's YAML seeds carry a note.
    assert!(
        checked >= 30,
        "expected the seed corpus to carry notes for many inputs, got {checked} note-bearing seeds"
    );
}
