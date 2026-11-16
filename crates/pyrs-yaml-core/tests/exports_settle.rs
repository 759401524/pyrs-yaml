//! How far a number's source spelling survives an export spoke.
//!
//! `fuzz/fuzz_targets/edit_yaml.rs` asks the settlement question of whatever it explores and says so
//! itself: an edited tree reaches the JSON5 and TOML writers through no other tier. Both cases below
//! come from that oracle. The TOML one settles because the reader carries a float token's whole text,
//! sign included, through the hub; the JSON5 one still drifts in type, because a JSON5 number and a
//! YAML string that looks like one are the same node: `from_json5` yields
//! `Scalar { style: Plain, tag: None, fidelity: None }`, and so does a YAML plain scalar whose text
//! overflows the integer range.
//!
//! The remaining pinned answer is today's, not the wanted one. quality-ledger (je) names the
//! remedies and the decision among them; the assertion states how to invert it when one lands.

use pyrs_json::{from_json5, to_json5_text};
use pyrs_toml::{from_toml, to_toml};
use pyrs_yaml_core::ast::CustomNode;
use pyrs_yaml_core::parser::parse;
use pyrs_yaml_core::parser::yaml::Schema;

fn tree(source: &str) -> CustomNode {
    parse(source, Schema::Core).expect("the pinned input parses")
}

/// libFuzzer `edit_yaml` crash-3291d29b, minimized to one key.
///
/// 26 hex digits overflow the integer range, so `Schema::Core` resolves this text to a string, while
/// the JSON5 fast path asks only whether the text matches a JSON5 number grammar (`is_json5_number`)
/// and emits it bare. The export changes the value's type on its very first round - the fuzz finding
/// (quoted once, bare twice, on an edited tree) is the same test seen from the style the edit left.
#[test]
fn a_hex_plain_scalar_outside_the_integer_range_exports_bare_and_then_stays_bare() {
    let source = "k: 0Xeeeeeeeeeeeeeeeeeeeeeeee4e6\n";
    let once = to_json5_text(&tree(source)).expect("the tree exports");
    assert_eq!(
        once, "{\"k\":0Xeeeeeeeeeeeeeeeeeeeeeeee4e6}",
        "the JSON5 fast path no longer echoes number-shaped text. If the resolved type was made to \
         win, the emission quotes the value - invert this to expect `\"0Xeeee...4e6\"` quoted"
    );
    let twice =
        to_json5_text(&from_json5(&once).expect("its own export re-reads")).expect("and re-writes");
    assert_eq!(
        once, twice,
        "the spoke stopped settling in one round: {once:?} -> {twice:?}"
    );
}

/// libFuzzer `edit_yaml` crash-d161db83, minimized to one key.
///
/// TOML takes a leading `+` on a float; the reader now records the token's whole text, sign
/// included, so both emissions keep `+83e6`. It used to record the digits alone, and the second pass
/// wrote the canonical `83e6`: the value never moved, so no equality tier saw it, and the loss was
/// the spelling. The same drift reached through a YAML source is crash-8a14f730.
#[test]
fn a_leading_plus_on_a_float_survives_every_toml_round() {
    let source = "k: +83e6\n";
    let once = to_toml(&tree(source)).expect("the tree exports");
    assert_eq!(once, "k = +83e6\n", "the source spelling is kept");
    let twice =
        to_toml(&from_toml(&once).expect("its own export re-reads")).expect("and re-writes");
    assert_eq!(
        once, twice,
        "the spoke stopped settling in one round: {once:?} -> {twice:?}"
    );
}
