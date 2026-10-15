//! Every emitted document is a fixed point: `to_yaml(parse(to_yaml(x))) == to_yaml(x)`.
//!
//! That is the fuzz tier's contract (`fuzz/fuzz_targets/yaml_roundtrip.rs`), asserted there over
//! whatever the fuzzer explores and here over named shapes - because a bounded fuzz budget can miss
//! a rare drift for a run, and a change to a placement rule should break a named test rather than an
//! unlucky draw. Each input below was reached by libFuzzer and minimized with `cargo fuzz tmin`;
//! quality-ledger (ds) carries the mechanism and the two remedies that were measured away before the
//! one that holds.

use pyrs_yaml_core::parser::parse;
use pyrs_yaml_core::parser::yaml::Schema;
use pyrs_yaml_core::serializer::to_yaml;

fn emit(src: &str) -> String {
    to_yaml(&parse(src, Schema::Core).expect("the pinned input parses"))
}

/// libFuzzer `yaml_roundtrip` crash-c2343438, minimized to 14 bytes as crash-2eaa5599: a key's
/// inline note over a block body.
///
/// The emission used to need a second round. Round one parked the *document's* note on the body's
/// last line, and granit spans a node through the break that ends its own line, so the re-read
/// reported that note as the value's; `write_pair` then asked the value who owns the line's
/// trailing slot, got the other answer, and hoisted the key's note above the pair. Nothing was lost
/// on the way, which is why neither the property tier (tree against tree, one cycle) nor
/// `note_survival` (does a note appear) saw it. `write_pair` asks the line now: a scalar body that
/// starts below leaves the pair line's slot free, so the note stays where it was written and the
/// first emission is the fixed point.
#[test]
fn a_key_note_over_a_block_body_settles_in_one_round() {
    for (src, notes) in [
        ("<<: #*&\n#&&*!\n ..f!ny#*&\n#&:", 3),
        (": #*&\n#!\n .\n#&", 3),
    ] {
        let one = emit(src);
        let again = emit(&one);
        assert_eq!(
            again, one,
            "not a fixed point in one round: {one:?} -> {again:?}"
        );
        assert_eq!(
            one.matches("# ").count(),
            notes,
            "the shape carries {notes} notes; an emission that folds or invents one is a loss even \
             when it is stable: {one:?}"
        );
    }
}

/// A note that follows a CR-only line break belongs to the item after it, not the container
/// before it.
///
/// `line_break_between` - the test that decides whether a trailing note binds backwards or
/// carries forward - scanned the gap for `\n` alone, so a document broken only by `\r` read as
/// one long line. The note bound back onto the mapping, the writer homed a container note by
/// its own rule, and the re-read gave the same text to the pair's value, so the first emission
/// was not the fixed point; the fuzz tier reports exactly that as drift. The break vocabulary
/// is now the one granit splits lines on, and the last assertion is the sharper oracle: the two
/// spellings of one document ingest to one emission.
#[test]
fn a_note_after_a_cr_only_break_annotates_the_item_that_follows() {
    let cr = "!-\r\r\r- :\r\r\r\r- #-";
    let one = emit(cr);
    assert_eq!(emit(&one), one, "not a fixed point in one round: {one:?}");
    assert_eq!(
        one.matches("# ").count(),
        1,
        "the shape carries one note; a migrated note is still a note, so idempotence alone \
         cannot tell a move from a copy: {one:?}"
    );
    let lf = emit(&cr.replace('\r', "\n"));
    assert_eq!(
        one, lf,
        "a document differs from its LF spelling only in how its lines break: {one:?} vs {lf:?}"
    );
}
