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

/// libFuzzer `yaml_roundtrip` crash-8b98eb75: the first mapping entry's inline note, under an
/// anchor/tag header.
///
/// `lift_first_entry_notes` moved the first key's *leading* notes above the header line, where the
/// reader reports them, but never read the key's inline slot. The `<<` key's note therefore stayed
/// below the `!` bare-tag header, and the re-read - which reports that note on the root node -
/// hoisted it above the header on the second round. The extraction now sees the inline slot too,
/// so the first emission already places it where the next read will.
#[test]
fn a_first_key_inline_note_under_a_header_settles_in_one_round() {
    let src = "! #& #&\n#e\n? #! #&\n#g: !:\t\n<<: #: #*b\n  ::e? #! #&\n#e\n? #!#&\n#e\n? #";
    let one = emit(src);
    assert!(one.contains("#*b"), "the note was dropped: {one:?}");
    let again = emit(&one);
    assert_eq!(
        again, one,
        "not a fixed point in one round: {one:?} -> {again:?}"
    );
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

/// libFuzzer `edit_yaml` crash-b23eed05: a flow member whose value is a tag with no content.
///
/// Nothing was lost across the round, which is why no other tier saw it — the note simply changed
/// owner. An empty plain scalar renders as nothing, so `{a: !t` is a tag-only entry, and a note
/// written after it has no value on its line: the reader hands it to the enclosing pair, and the
/// second emission moves the tag behind the note. `edit_yaml` reaches it by setting a tag and then a
/// comment on the same member; the pinned text below is the tree it produced. The writer's remedy is
/// the one it already uses for an empty null before a note — spell the body — so `{a: !t ''  # n`
/// re-reads as the document it was written from and the first emission is the fixed point.
#[test]
fn a_tagged_empty_flow_member_with_a_note_settles_in_one_round() {
    use pyrs_yaml_core::ast::{Comment, Tag};
    use pyrs_yaml_core::editing::Segment;
    use pyrs_yaml_core::editing::plan::{set_comment_path, set_tag_path};
    use std::borrow::Cow;

    let src = "k: {a: !o }\n";
    let mut node = parse(src, Schema::Core).expect("the pinned input parses");
    let path = [
        Segment::Key(Cow::Borrowed("k")),
        Segment::Key(Cow::Borrowed("a")),
    ];
    set_tag_path(&mut node, &path, Tag::local("t"), src, None).expect("the tag is set");
    set_comment_path(
        &mut node,
        &path,
        Comment {
            text: "n".into(),
            standalone: false,
        },
        src,
        None,
    )
    .expect("the note is set");

    let one = to_yaml(&node);
    assert_eq!(
        one, "k: {a: !t ''  # n\n    }\n",
        "the note needs a spelled body to sit on"
    );
    let again = emit(&one);
    assert_eq!(
        again, one,
        "not a fixed point in one round: {one:?} -> {again:?}"
    );
    assert_eq!(
        to_yaml(&parse(&one, Schema::Core).expect("the emission re-parses")),
        one,
        "the settled text is not itself stable"
    );
}
