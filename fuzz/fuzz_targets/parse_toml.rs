//! TOML 1.0 / 1.1 parsers and the TOML writer, fuzzed together.
//!
//! Typed errors are passes; panics are bugs, and so is writer output that the current reader cannot
//! re-read. The dialect flag is exercised on both grammar revisions and the depth guard (#166
//! lineage) must reject rather than overflow.
//!
//! Two promises, each asserted exactly once:
//!
//! 1. Whatever gate read the document, the current reader accepts what `to_toml` hands back. Nothing
//!    checked this before: every re-read in the old target was `let _ =`, so the failure this file's
//!    predecessor called a bug could not be observed.
//! 2. A node read under the 1.0 gate writes text the 1.0 gate still accepts — *unless* the node
//!    carries a note. That exception is PR #119's ruling, not a hole: `#` runs to end of line, so a
//!    comment cannot live inside `{ a = 1 }` or `[1, 2]`, and a noted member forces the multi-line
//!    form. Comment survival outranks the older grammar by that change's own decision. `writer.rs`
//!    states both halves in prose ("Undecorated inline tables keep the compact single-line form so
//!    1.0-compatible output is unchanged"), and
//!    `crates/pyrs-toml/src/writer.rs::the_undecorated_1_0_promise_holds_and_a_note_is_what_leaves_it`
//!    pins them.
//!
//! The shape that found the distinction: `#\n  #\n[se.c]\nk = "v"   #x`. A dotted header reads as a
//! nested mapping, which `to_toml` projects onto the documented `[se]` + inline-table form, and the
//! note then breaks that table across lines — intended 1.1 output. Charging it with promise 2 reports
//! a ruling as a crash, and an oracle that fires on a ruling is worse than no oracle: the next person
//! weakens it wholesale.
//!
//! A writer rejection stays a pass. `write_value` refuses shapes the YAML hub can hold and TOML
//! cannot (a timestamp tag whose text is malformed, a null, an alias), which is a documented
//! rejection rather than a round trip the writer broke.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pyrs_ast::ast::CustomNode;
use pyrs_toml::{from_toml, from_toml_v1_0, to_toml};

/// Whether any node in the tree carries a note, which is what makes the writer break an inline
/// table or an array across lines and therefore leave the 1.0 grammar.
fn carries_note(node: &CustomNode) -> bool {
    if node.has_notes() {
        return true;
    }
    match node {
        CustomNode::Mapping { pairs, .. } => pairs
            .iter()
            .any(|(k, v)| carries_note(k) || carries_note(v)),
        CustomNode::Sequence { items, .. } => items.iter().any(carries_note),
        _ => false,
    }
}

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    for (gate, parsed) in [("1.1", from_toml(text)), ("1.0", from_toml_v1_0(text))] {
        let Ok(node) = parsed else { continue };
        let Ok(out) = to_toml(&node) else {
            continue; // documented rejection is a pass
        };
        from_toml(&out).unwrap_or_else(|e| {
            panic!(
                "{gate} read a document, to_toml wrote text the 1.1 reader rejects: {e}\n---\n{out}"
            )
        });
        if gate == "1.0" && !carries_note(&node) {
            from_toml_v1_0(&out).unwrap_or_else(|e| {
                panic!("undecorated output must stay 1.0-loadable: {e}\n---\n{out}")
            });
        }
    }
});
