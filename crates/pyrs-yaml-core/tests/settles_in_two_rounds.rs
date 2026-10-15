//! Shapes whose emission needs a second round, pinned so that a fix is judged rather than remembered.
//!
//! The fuzz tier's contract is that `to_yaml` output is a fixed point, and that one assertion has
//! driven every comment-placement fix in this engine. These are the inputs where it fails today and
//! the defect stays unfixed: the two remedies that were tried each settled the pinned shape and
//! reddened six properties instead, so neither landed (quality-ledger (ds) carries both residuals).
//! Pinning the observed behaviour is what keeps the shape in the tree - and each test here asserts
//! the drift, so the day the placement rule is repaired this file goes red by name and must be
//! rewritten to require `one == again`, which is the point of a pin rather than a to-do note.
//!
//! The inputs live here rather than in `fuzz/seeds/`, because a seed is replayed by the fuzz gate on
//! every PR and these still fail it.

use pyrs_yaml_core::parser::parse;
use pyrs_yaml_core::parser::yaml::Schema;
use pyrs_yaml_core::serializer::to_yaml;

fn emit(src: &str) -> String {
    to_yaml(&parse(src, Schema::Core).expect("the pinned input parses"))
}

/// libFuzzer `yaml_roundtrip` crash-c2343438, minimized by `cargo fuzz tmin` to 14 bytes as
/// crash-2eaa5599: a key's inline note over a block body.
///
/// Round one parks the *document's* note on the body's last line, and granit spans a node through
/// the break that ends its own line, so the re-read reports that note as the value's. `write_pair`
/// then asks who owns the line's trailing slot, gets a different answer, and hoists the key's note
/// above the pair. Nothing is lost on the way - hence the third assertion - which is why the
/// property tier (tree against tree, one cycle) and `note_survival` (does a note appear) both pass
/// these inputs.
#[test]
fn a_key_note_over_a_block_body_moves_on_the_second_round() {
    for src in ["<<: #*&\n#&&*!\n ..f!ny#*&\n#&:", ": #*&\n#!\n .\n#&"] {
        let one = emit(src);
        let again = emit(&one);
        let third = emit(&again);
        assert_ne!(
            one, again,
            "the drift below settled in one round, so the placement rule is fixed: {one:?} - \
             rewrite this pin to require `one == again`"
        );
        assert_eq!(
            third, again,
            "the emission never converges: {one:?} -> {again:?}"
        );
        assert_eq!(
            one.matches("# ").count(),
            again.matches("# ").count(),
            "a note was lost or doubled across the rounds: {one:?} -> {again:?}"
        );
    }
}
