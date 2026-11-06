//! How far a number's source spelling survives an export spoke: one round, not two.
//!
//! `fuzz/fuzz_targets/edit_yaml.rs` asks the settlement question of whatever it explores and says so
//! itself: an edited tree reaches the JSON5 and TOML writers through no other tier. Both shapes below
//! are one mechanism seen from two dialects: the writers pass a number's text through (fidelity
//! first), but re-reading a number restores no text, so the second emission canonicalizes. One case
//! drifts in spelling, the other drifts in type - and the reason they cannot both be fixed by asking
//! the text is that a JSON5 number and a YAML string that looks like one are the same node:
//! `from_json5` yields `Scalar { style: Plain, tag: None, fidelity: None }`, and so does a YAML plain
//! scalar whose text overflows the integer range.
//!
//! These assert today's answer, not the wanted one. quality-ledger (je) names the remedies and the
//! decision among them; each assertion states how to invert it when one lands.

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
/// TOML takes a leading `+` on a float, so the first emission keeps `+83e6` - the rule
/// `preserves_exponent_float_source_spelling` pins that behavior deliberately. Re-reading yields the
/// same f64 with no text attached, so the second pass writes the canonical `83e6`. The value never
/// moves, which is why no equality tier sees it; the loss is the spelling, and it is why the
/// settlement claim fails.
#[test]
fn a_leading_plus_on_a_float_survives_one_toml_round_and_not_two() {
    let source = "k: +83e6\n";
    let once = to_toml(&tree(source)).expect("the tree exports");
    let twice =
        to_toml(&from_toml(&once).expect("its own export re-reads")).expect("and re-writes");
    assert_eq!(
        (once.as_str(), twice.as_str()),
        ("k = +83e6\n", "k = 83e6\n"),
        "the float spelling stopped drifting. If the reader now carries the source text both rounds \
         are `k = +83e6`; if canonicalization was chosen both are `k = 83e6` - collapse the pair to \
         one value either way"
    );
}
