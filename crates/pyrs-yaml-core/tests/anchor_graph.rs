//! Anchor-graph fidelity over the committed fuzz corpus.
//!
//! The round-trip tier's oracle is idempotence and the note tier asks whether every note the
//! reader recorded came back. Neither asks what an anchored document needs: **the tree the read
//! path builds must be internally consistent, and one emission must not move the anchor graph.**
//! A tree can be stable under re-emission while holding an alias that names no anchor in it —
//! that is `engine:read-drops-anchor-keeps-alias`, and the crash-68617eaf input does it before
//! any edit is applied. An alias whose anchor is gone is not a formatting difference: it is a
//! document this library's own reader refuses.
//!
//! Two asks per input, on the closed corpus:
//!
//! 1. read-side self-consistency — every `Alias` name is defined by an anchor in the same tree;
//! 2. emission fidelity — the anchor names the input's tree carries are still present after one
//!    `to_yaml` + re-parse, so a writer cannot drop a definition aliases still address.
//!
//! **What this gate cannot see, stated because a green check invites trust in it.** It reads the
//! *document* half of each seed only, so an anchor discarded by an edit verb is invisible here —
//! that claim belongs to the `edit_yaml` fuzz tier, which the pinned `former-crash-*` seeds
//! replay. Inputs that are not valid UTF-8 or that the parser rejects are skipped: no tree, so
//! nothing to ask.
//!
//! Proven to bite, not argued: while the defect was open, dropping `: abase: &x 1\nuse: *x\n:`
//! into the walked corpus reddened the gate by name — `documents whose alias names no anchor:
//! [("zz_mutation_probe.seed", ["x"])]` — and the same input passes it now that the fold
//! registers every definition leaving the tree. That shape is pinned by
//! `a_definition_folded_away_from_inside_the_dropped_value_is_still_emitted` in this file. The
//! gate asks about the tree the read path built; the `edit_yaml` oracle asks about the document
//! *after* a script, which is why that input cleared the fuzz tier's `-runs=0` replay while this
//! gate refused it. Measured 2026-10-20: 102 curated `.seed` documents, and 1,131 with the
//! extension filter widened, walk clean.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use pyrs_yaml_core::ast::CustomNode;
use pyrs_yaml_core::parser::parse;
use pyrs_yaml_core::parser::yaml::Schema;
use pyrs_yaml_core::serializer::to_yaml;

fn collect_anchor(node: &CustomNode, into: &mut BTreeSet<String>) {
    if let Some(name) = node.anchor() {
        into.insert(name.to_string());
    }
    match node {
        CustomNode::Mapping { pairs, .. } => {
            for (key, value) in pairs.iter() {
                collect_anchor(key, into);
                collect_anchor(value, into);
            }
        }
        CustomNode::Sequence { items, .. } => {
            for item in items {
                collect_anchor(item, into);
            }
        }
        _ => {}
    }
}

fn collect_alias(node: &CustomNode, into: &mut BTreeSet<String>) {
    if let CustomNode::Alias { name } = node {
        into.insert(name.clone());
    }
    match node {
        CustomNode::Mapping { pairs, .. } => {
            for (key, value) in pairs.iter() {
                collect_alias(key, into);
                collect_alias(value, into);
            }
        }
        CustomNode::Sequence { items, .. } => {
            for item in items {
                collect_alias(item, into);
            }
        }
        _ => {}
    }
}

fn anchors_of(node: &CustomNode) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    collect_anchor(node, &mut names);
    names
}

fn aliases_of(node: &CustomNode) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    collect_alias(node, &mut names);
    names
}

/// The document half of a seed. `edit_yaml` inputs are `document NUL script`, and the script is
/// the fuzz tier's business rather than this gate's.
fn document_of(raw: &[u8]) -> Option<&str> {
    let cut = raw.iter().position(|byte| *byte == 0).unwrap_or(raw.len());
    std::str::from_utf8(&raw[..cut]).ok()
}

fn seed_files() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds");
    let mut files = Vec::new();
    for name in ["yaml_roundtrip", "parse_yaml", "edit_yaml"] {
        let dir = root.join(name);
        if !dir.is_dir() {
            continue;
        }
        for entry in
            std::fs::read_dir(&dir).unwrap_or_else(|e| panic!("{} readable: {e}", dir.display()))
        {
            let path = entry.expect("entry").path();
            if path.extension().is_some_and(|ext| ext == "seed") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

#[test]
fn a_definition_folded_away_from_inside_the_dropped_value_is_still_emitted() {
    // The duplicate null key costs the tree `{abase: &x 1}`, and `use: *x` still names that
    // definition. The fold's repair pass holds the dropped entry's *own* anchor; the case that
    // used to survive it sat one level inside, and the engine's promise is that no emission is
    // text its own reader refuses (libFuzzer `edit_yaml` crash-68617eaf).
    let src = ": abase: &x 1\nuse: *x\n:";
    let node = parse(src, Schema::Core).expect("the pinned input parses");
    let emitted = to_yaml(&node);
    let reread = parse(&emitted, Schema::Core)
        .unwrap_or_else(|e| panic!("{emitted:?} must re-read, the reader said {e:?}"));
    assert!(
        anchors_of(&reread).contains("x"),
        "the definition did not reach the output: {emitted:?}"
    );
    assert!(
        aliases_of(&reread).is_empty(),
        "{emitted:?} still holds the orphan alias"
    );
}

#[test]
fn the_anchor_graph_survives_one_emission() {
    let mut checked = 0usize;
    let mut orphaned: Vec<(String, Vec<String>)> = Vec::new();
    let mut lost: Vec<(String, Vec<String>)> = Vec::new();

    for path in seed_files() {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let raw = std::fs::read(&path).expect("seed readable");
        let Some(src) = document_of(&raw) else {
            continue;
        };
        let Ok(node) = parse(src, Schema::Core) else {
            continue;
        };
        checked += 1;

        let anchors = anchors_of(&node);
        let aliases = aliases_of(&node);
        let orphans: Vec<String> = aliases.difference(&anchors).cloned().collect();
        if !orphans.is_empty() {
            orphaned.push((name, orphans));
            continue;
        }

        let emitted = to_yaml(&node);
        let Ok(reread) = parse(&emitted, Schema::Core) else {
            orphaned.push((name, vec!["the emission does not re-parse".to_string()]));
            continue;
        };
        let missing: Vec<String> = anchors.difference(&anchors_of(&reread)).cloned().collect();
        if !missing.is_empty() {
            lost.push((name, missing));
        }
    }

    // The count is asserted, not printed: a corpus directory that moved would otherwise leave
    // this gate green by checking nothing.
    eprintln!("anchor-graph gate: {checked} corpus documents walked");
    assert!(
        checked >= 50,
        "the anchor gate walked only {checked} documents"
    );
    assert!(
        orphaned.is_empty(),
        "documents whose alias names no anchor: {orphaned:?}"
    );
    assert!(
        lost.is_empty(),
        "anchors lost across one emission: {lost:?}"
    );
}
