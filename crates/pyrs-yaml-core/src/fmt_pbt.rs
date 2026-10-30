//! Property-based fuzz tests for the TOML / JSON / JSONC / JSON5 formats.
//!
//! `pbt.rs` fuzzes the YAML spine; this module gives the four other formats the
//! same treatment, covering the properties the objective calls for:
//!
//! - **no-panic on hostile input** - each parser is fed arbitrary (often invalid)
//!   bytes and must return `Ok` or `Err`, never panic.
//! - **no-panic serialization** - each writer is driven over arbitrary ASTs and
//!   must never panic even on nodes the format cannot represent (it returns `Err`).
//! - **re-parseability + writer fixed point** - whatever a writer emits must
//!   re-parse, and re-serializing the re-parsed value is a fixed point (the
//!   writer is idempotent on its own output).

#![cfg(test)]

use crate::ast::proptest_strategies::*;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use proptest::prelude::*;
use pyrs_json as json;
use pyrs_toml as toml;

/// Domain filter for the JSON-family fixed points. RFC 8259 objects have
/// unique names, but a hand-built AST can hold two *distinct* keys that
/// spell the same JSON name (the Null variant and a `"null"` string scalar,
/// or the same key carrying different tags). No JSON text represents that
/// shape losslessly — the writer emits duplicate names and the parser's
/// last-wins rule folds them, so the second pass differs. Those inputs are
/// outside the object domain; the fixed-point properties assume them away
/// rather than pretending the drift is a bug.
fn json_object_domain(node: &crate::ast::CustomNode) -> bool {
    use pyrs_json::key_text;
    match node {
        crate::ast::CustomNode::Mapping { pairs, .. } => {
            let mut names: Vec<String> = Vec::new();
            for (k, v) in pairs {
                match key_text(k) {
                    Ok(name) => {
                        if names.contains(&name) {
                            return false;
                        }
                        names.push(name);
                    }
                    // Non-string keys make the writer fail outright; the
                    // `let Ok(text)` guard in each property skips those.
                    Err(_) => return true,
                }
                if !json_object_domain(v) {
                    return false;
                }
            }
            true
        }
        crate::ast::CustomNode::Sequence { items, .. } => items.iter().all(json_object_domain),
        _ => true,
    }
}

/// Domain filter for the TOML fixed point. Native TOML parsing places a
/// document's first standalone note on the first key/table, never on the
/// root, so a *parsed* non-empty document's root carries no leading comment.
/// A hand-built AST can put a standalone note on BOTH the root and the first
/// element; the writer renders them as two adjacent `# …` lines that TOML
/// cannot attribute to two owners — re-parsing folds them into one. That
/// shape has no faithful TOML text, so the fixed-point property assumes it
/// away rather than treating the fold as a writer bug. (My `from_toml` change
/// attaches a leftover note to the root only when the document is empty,
/// which stays round-trip stable and is not filtered here.)
fn toml_root_note_ok(node: &crate::ast::CustomNode) -> bool {
    !matches!(
        node,
        crate::ast::CustomNode::Mapping { pairs, .. }
            if !pairs.is_empty() && node.leading_comment().is_some()
    )
}

/// Whether the tree carries no note anywhere — on a node, on a key, or in any
/// descendant.
///
/// The TOML writer targets 1.1, which is the dialect that allows a newline (and so a comment) inside
/// an inline table. A document with no note at all has no reason to need that freedom, and emitting
/// one would be the writer choosing 1.1-only text where 1.0 text was available — a reader on 1.0
/// would then refuse output this library produced. Blank-line hints are not filtered: blank lines
/// between key-values and sections are legal in 1.0.
fn toml_undecorated(node: &crate::ast::CustomNode) -> bool {
    if node.comment().is_some() || !node.leading_comments().is_empty() {
        return false;
    }
    match node {
        crate::ast::CustomNode::Mapping { pairs, .. } => pairs.iter().all(|(k, v)| {
            k.comment().is_none()
                && k.leading_comments().is_empty()
                && toml_undecorated(k)
                && toml_undecorated(v)
        }),
        crate::ast::CustomNode::Sequence { items, .. } => items.iter().all(toml_undecorated),
        _ => true,
    }
}

/// Every note the tree carries, as it must appear in the emitted TOML.
///
/// A reach check, not an equality check: it asks only that no note silently disappears, which is the
/// claim the writer owns. Who owns a note once the document re-parses is a different question —
/// `toml_root_note_ok` exists because two adjacent `# …` lines have no two-owner spelling in TOML —
/// and the fixed-point properties above cannot see a drop at all, because text that lost a note on
/// the first emission loses it identically on the second, so the two passes match.
///
/// Restated here rather than borrowed from the serializer, so a writer that stopped flattening
/// reddens this check instead of agreeing with it: a TOML comment runs to the line end and has no
/// spelling for a break, so what must reach the document is the one-line form; padding after the
/// marker is not content any reader recovers, so the text is trimmed first. A note that is blank
/// once trimmed carries nothing a comment can hold, and is exempt rather than tolerated silently.
fn toml_missing_notes(node: &crate::ast::CustomNode, emitted: &str) -> Vec<String> {
    let mut missing = Vec::new();
    collect_note_texts(node, emitted, &mut missing);
    missing
}

fn collect_note_texts(node: &crate::ast::CustomNode, emitted: &str, missing: &mut Vec<String>) {
    use crate::ast::CustomNode as N;
    for text in node
        .comment()
        .filter(|c| !c.standalone)
        .map(|c| vec![c.text.to_string()])
        .unwrap_or_default()
        .into_iter()
        .chain(node.leading_comments().iter().map(|c| c.text.to_string()))
    {
        let trimmed = text.trim();
        let expected = trimmed.replace(['\n', '\r'], " ");
        if !trimmed.is_empty() && !emitted.contains(&expected) {
            missing.push(format!("note never reached the document: {text:?}"));
        }
    }
    match node {
        N::Mapping { pairs, .. } => {
            for (k, v) in pairs {
                collect_note_texts(k, emitted, missing);
                collect_note_texts(v, emitted, missing);
            }
        }
        N::Sequence { items, .. } => {
            for item in items {
                collect_note_texts(item, emitted, missing);
            }
        }
        _ => {}
    }
}

/// Undecorated trees that are *certain* to contain an inline table.
///
/// `arb_node_meta` attaches a note to a node often enough that a multi-node tree with no note
/// anywhere is rare, and the writer's compact inline-table arm was therefore reached almost never:
/// forcing the multi-line branch left `prop_toml_undecorated_output_loads_as_1_0` green at 20 000
/// generated cases while it reddened the named test in `pyrs-toml` immediately. These shapes close
/// the hole by spelling the document as TOML text first — undecorated by construction, always
/// holding `{ … }` — and handing the parsed tree to the property. The claim stays the writer's: a
/// 1.0-shaped input must still come out as text a 1.0 reader accepts.
fn arb_toml_inline_shapes() -> impl Strategy<Value = crate::ast::CustomNode> {
    (
        "[a-z][a-z0-9_]{0,6}",
        prop::string::string_regex("[A-Za-z0-9 ]{0,8}").unwrap(),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_filter_map("the TOML reader refused its own fixture", |v| {
            let (key, value, nested, arrayed) = v;
            let mut source = format!("{key} = {{ inner = \"{value}\" }}\n");
            if nested {
                source.push_str(&format!("deep = {{ mid = {{ leaf = \"{value}\" }} }}\n"));
            }
            if arrayed {
                source.push_str("list = [ { a = 1 }, { b = 2 } ]\n");
            }
            toml::from_toml(&source).ok()
        })
}

/// Config for the dialect properties: a wider budget for the domain filters.
///
/// `json_object_domain` and `toml_root_note_ok` are carve-outs around shapes that have no
/// faithful text in the target format (two distinct AST keys that spell one JSON name; a
/// standalone note on both a TOML root and its first key). `prop_assume!` therefore
/// *rejects* those samples, and proptest's default allowance — 1024 global rejects — was
/// calibrated against 256 cases. Run at the elevated tier the properties were aborting on
/// the budget after ~3400 real successes with zero assertion failures, i.e. the harness
/// was reporting its own reject rate as if a writer had drifted. The fix is the budget, not
/// the oracle: `PROPTEST_CASES` stays authoritative for how much is explored, and every
/// non-rejected sample still has to settle byte-for-byte.
fn dialect_config() -> ProptestConfig {
    ProptestConfig {
        max_global_rejects: 250_000,
        ..Default::default()
    }
}

proptest! {
    #![proptest_config(dialect_config())]

    // -- no-panic: parsers on arbitrary (mostly invalid) input ---------------
    #[test]
    fn prop_dialect_parsers_never_panic(bytes in prop::collection::vec(any::<u8>(), 0..512)) {
        let s = String::from_utf8_lossy(&bytes);
        let _ = json::from_json(&s);
        let _ = json::from_jsonc(&s);
        let _ = json::from_json5(&s);
        let _ = toml::from_toml(&s);
    }

    // -- no-panic: writers on arbitrary ASTs (may legitimately error) --------
    #[test]
    fn prop_dialect_writers_never_panic(node in arb_custom_node()) {
        let _ = json::to_json_text(&node);
        let _ = json::to_jsonc_text(&node);
        let _ = json::to_json5_text(&node);
        let _ = toml::to_toml(&node);
    }

    // -- re-parseability -----------------------------------------------------
    // Whatever a writer emits (for a node it can represent) must parse back with
    // no error. Comment fidelity through the dialect round-trip is exercised by
    // the dialect unit tests; here we assert the writer never emits unparseable
    // output, using arbitrary generated ASTs.
    #[test]
    fn prop_json_output_reparses(node in arb_custom_node()) {
        let Ok(text) = json::to_json_text(&node) else { return Ok(()) };
        prop_assert!(json::from_json(&text).is_ok(), "JSON writer output must re-parse: {text}");
    }

    #[test]
    fn prop_jsonc_output_reparses(node in arb_custom_node()) {
        let Ok(text) = json::to_jsonc_text(&node) else { return Ok(()) };
        prop_assert!(json::from_jsonc(&text).is_ok(), "JSONC writer output must re-parse: {text}");
    }

    #[test]
    fn prop_json5_output_reparses(node in arb_custom_node()) {
        let Ok(text) = json::to_json5_text(&node) else { return Ok(()) };
        prop_assert!(json::from_json5(&text).is_ok(), "JSON5 writer output must re-parse: {text}");
    }

    #[test]
    fn prop_toml_output_reparses(node in arb_custom_node()) {
        let Ok(text) = toml::to_toml(&node) else { return Ok(()) };
        prop_assert!(toml::from_toml(&text).is_ok(), "TOML writer output must re-parse: {text}");
    }

    // -- writer fixed point --------------------------------------------------
    // The module header promises idempotency: re-serializing what a writer's
    // own output re-parses to must reproduce that output byte-for-byte. A
    // drift here means the writer emits spellings its parser normalises
    // (number forms, escaping, layout) — text that survives round-trip only
    // until the second pass.
    #[test]
    fn prop_json_writers_are_fixed_points(node in arb_custom_node()) {
        prop_assume!(json_object_domain(&node));
        let Ok(text) = json::to_json_text(&node) else { return Ok(()) };
        let reparsed = json::from_json(&text)?;
        let text2 = json::to_json_text(&reparsed)?;
        prop_assert_eq!(text, text2, "JSON writer not idempotent on own output");

        let Ok(text) = json::to_jsonc_text(&node) else { return Ok(()) };
        let reparsed = json::from_jsonc(&text)?;
        let text2 = json::to_jsonc_text(&reparsed)?;
        prop_assert_eq!(text, text2, "JSONC writer not idempotent on own output");
    }

    #[test]
    fn prop_json5_writer_is_fixed_point(node in arb_custom_node()) {
        prop_assume!(json_object_domain(&node));
        let Ok(text) = json::to_json5_text(&node) else { return Ok(()) };
        let reparsed = json::from_json5(&text)?;
        let text2 = json::to_json5_text(&reparsed)?;
        prop_assert_eq!(text, text2, "JSON5 writer not idempotent on own output");
    }

    #[test]
    fn prop_pretty_json_writers_are_fixed_points(node in arb_custom_node()) {
        prop_assume!(json_object_domain(&node));
        // The fixed point watched the compact route only, which left the pretty route's placement of
        // comments - the part that writes them on lines of their own - pinned by hand-built tables.
        // Generated trees are the stronger oracle, and this is what the (cg) entry named as the gap.
        let Ok(text) = json::to_json_text_pretty(&node, 2) else { return Ok(()) };
        let text2 = json::to_json_text_pretty(&json::from_json(&text)?, 2)?;
        prop_assert_eq!(text, text2, "pretty JSON writer not idempotent on own output");

        let Ok(text) = json::to_jsonc_text_pretty(&node, 2) else { return Ok(()) };
        let text2 = json::to_jsonc_text_pretty(&json::from_jsonc(&text)?, 2)?;
        prop_assert_eq!(text, text2, "pretty JSONC writer not idempotent on own output");

        let Ok(text) = json::to_json5_text_pretty(&node, 2) else { return Ok(()) };
        let text2 = json::to_json5_text_pretty(&json::from_json5(&text)?, 2)?;
        prop_assert_eq!(text, text2, "pretty JSON5 writer not idempotent on own output");
    }

    #[test]
    fn prop_toml_writer_is_fixed_point(node in arb_custom_node()) {
        prop_assume!(toml_root_note_ok(&node));
        let Ok(text) = toml::to_toml(&node) else { return Ok(()) };
        let reparsed = toml::from_toml(&text)?;
        let text2 = toml::to_toml(&reparsed)?;
        prop_assert_eq!(text, text2, "TOML writer not idempotent on own output");
    }

    // -- note conservation ---------------------------------------------------
    // The three properties above answer "does it settle", "does it re-parse", "does the tree come
    // back" — and a writer that drops a note on the floor passes all three, because the note is
    // absent from both passes. This is the claim PR #676 fixed: a container's own leading note never
    // reached the document, and the tier stayed green until a hand-built table named the shape.
    #[test]
    fn prop_toml_writer_carries_every_note(node in arb_custom_node()) {
        let Ok(text) = toml::to_toml(&node) else { return Ok(()) };
        let missing = toml_missing_notes(&node, &text);
        prop_assert!(
            missing.is_empty(),
            "{} note(s) lost, on:\n{text}\nlosses:\n{}",
            missing.len(),
            missing.join("\n")
        );
    }

    #[test]
    fn prop_toml_undecorated_output_loads_as_1_0(
        node in prop_oneof![arb_custom_node(), arb_toml_inline_shapes()],
    ) {
        prop_assume!(toml_undecorated(&node));
        let Ok(text) = toml::to_toml(&node) else { return Ok(()) };
        // The fixed point asks whether the writer settles; it says nothing about *which dialect* the
        // settled text belongs to. A tree with no note anywhere has no claim on 1.1's freedom to put a
        // newline inside an inline table, so text that 1.0 refuses is the writer taking a dialect it
        // was not asked for — and a 1.0 reader would then not be able to open what we emitted.
        //
        // The second half of the input is measured, not decorative: with `arb_custom_node` alone,
        // forcing the writer's multi-line inline-table branch left this property green at 20 000
        // cases while it reddened `an_undecorated_inline_table_writes_compact_and_loads_as_1_0` in
        // `pyrs-toml` at once — the general generator reaches a note-free `flow_style` mapping only
        // occasionally. `arb_toml_inline_shapes` guarantees that arm, and the named test keeps the
        // shape on the record.
        let reloaded = toml::from_toml_v1_0(&text);
        prop_assert!(
            reloaded.is_ok(),
            "an undecorated tree wrote text TOML 1.0 refuses ({:?}):\n{}",
            reloaded.err().map(|e| e.to_string()),
            text
        );
    }

    #[test]
    fn prop_toml_roundtrip_preserves_the_tree(
        node in prop_oneof![arb_custom_node(), arb_toml_inline_shapes()],
    ) {
        prop_assume!(toml_root_note_ok(&node));
        // What the two text-level properties cannot say: a document may settle byte-for-byte and still
        // have lost a value, because both ask about the *text* and never about what came back. This is
        // the equality claim the matrix's property row names as its blind spot ("invariants weaker than
        // equality"), and it is the one that found the YAML losses. Scope: values, shape and order —
        // style and metadata belong to the dialect and note tiers, so `toml_tree_diff` ignores them.
        let Ok(text) = toml::to_toml(&node) else { return Ok(()) };
        let back = toml::from_toml(&text)?;
        if let Some(diff) = toml_tree_diff(&node, &back) {
            prop_assert!(false, "TOML round trip changed the tree: {diff}\n--- emitted ---\n{text}");
        }
    }
}

/// The first value, shape or order a TOML round trip changed, ignoring style and metadata.
fn toml_tree_diff(a: &crate::ast::CustomNode, b: &crate::ast::CustomNode) -> Option<String> {
    use crate::ast::CustomNode as N;
    fn kind(n: &crate::ast::CustomNode) -> &'static str {
        match n {
            N::Scalar { .. } => "scalar",
            N::Mapping { .. } => "mapping",
            N::Sequence { .. } => "sequence",
            N::Alias { .. } => "alias",
            N::Null { .. } => "null",
        }
    }
    match (a, b) {
        (N::Scalar { value: av, .. }, N::Scalar { value: bv, .. }) => {
            (av != bv).then(|| format!("scalar text {av:?} -> {bv:?}"))
        }
        (N::Mapping { pairs: ap, .. }, N::Mapping { pairs: bp, .. }) => {
            if ap.len() != bp.len() {
                return Some(format!("mapping size {} -> {}", ap.len(), bp.len()));
            }
            for ((ak, av), (bk, bv)) in ap.iter().zip(bp.iter()) {
                if let Some(d) = toml_tree_diff(ak, bk) {
                    return Some(format!("key {d}"));
                }
                if let Some(d) = toml_tree_diff(av, bv) {
                    return Some(format!("value {d}"));
                }
            }
            None
        }
        (N::Sequence { items: ai, .. }, N::Sequence { items: bi, .. }) => {
            if ai.len() != bi.len() {
                return Some(format!("sequence size {} -> {}", ai.len(), bi.len()));
            }
            for (i, (x, y)) in ai.iter().zip(bi.iter()).enumerate() {
                if let Some(d) = toml_tree_diff(x, y) {
                    return Some(format!("item {i}: {d}"));
                }
            }
            None
        }
        _ if kind(a) == kind(b) => None,
        _ => Some(format!("{} -> {}", kind(a), kind(b))),
    }
}
