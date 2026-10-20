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

/// libFuzzer `edit_yaml` crash-3dd60cad: the same tag-only line, one position further - a block
/// sequence item.
///
/// `- !o` renders an item whose value is nothing, so a note written after its tag has no body to sit
/// on: the reader files it as the item's *leading* note and the writer spells a leading note on the
/// first item as a line of its own above the dash. The first emission `- !fuzz2  # fuzz 47` came back
/// as `# fuzz 47\n- !fuzz2` - nothing lost, one round short of a fixed point. The mapping side of this
/// line has an answer (`close_pending_tag_line`); the item side did not.
#[test]
fn a_tagged_empty_sequence_item_with_a_note_settles_in_one_round() {
    use pyrs_yaml_core::ast::{Comment, Tag};
    use pyrs_yaml_core::editing::Segment;
    use pyrs_yaml_core::editing::plan::{set_comment_path, set_tag_path};

    let src = "- !o\n";
    let mut node = parse(src, Schema::Core).expect("the pinned input parses");
    let path = [Segment::Index(0)];
    set_tag_path(&mut node, &path, Tag::local("fuzz2"), src, None).expect("the tag is set");
    set_comment_path(
        &mut node,
        &path,
        Comment {
            text: "fuzz 47".into(),
            standalone: false,
        },
        src,
        None,
    )
    .expect("the note is set");

    let one = to_yaml(&node);
    assert_eq!(
        one, "- !fuzz2 ~  # fuzz 47\n",
        "the note needs a spelled body to sit on, here and in a flow member alike"
    );
    let again = emit(&one);
    assert_eq!(
        again, one,
        "not a fixed point in one round: {one:?} -> {again:?}"
    );
}

/// libFuzzer `edit_yaml` crash-a58caa38: an edit that empties a block sequence leaves its flag behind.
///
/// An empty container has no block form, so the writer always spells it `[]` on the key's line - the
/// compactness rule was vetoing a compactness the emission does not have - and the reader answers `[]`
/// with the flow flag set. The first emission was `-\n  t: []`, and the second pulled the mapping up
/// under the dash.
#[test]
fn an_emptied_block_sequence_keeps_the_dash_line() {
    use pyrs_yaml_core::editing::Segment;
    use pyrs_yaml_core::editing::plan::delete_path;
    use std::borrow::Cow;

    let src = "- t:\n  - \n";
    let mut node = parse(src, Schema::Core).expect("the pinned input parses");
    let path = [
        Segment::Index(0),
        Segment::Key(Cow::Borrowed("t")),
        Segment::Index(0),
    ];
    delete_path(&mut node, &path, src, None).expect("the last item is removed");

    let one = to_yaml(&node);
    assert_eq!(
        one, "- t: []\n",
        "an empty container is spelled on the key's line"
    );
    let again = emit(&one);
    assert_eq!(
        again, one,
        "not a fixed point in one round: {one:?} -> {again:?}"
    );
}

/// libFuzzer `edit_yaml` crash-63749496: a scalar with no text and no properties.
///
/// An empty plain scalar renders as nothing, and the reader does not hand the same node back from
/// an empty line: `k: ` and `- ` both re-read as a null whose own spelling is `~`, so the first
/// emission was one round short of a fixed point. The route the fuzzer reached was a folded item
/// whose style was set to plain; the hole was in the scalar writer itself, so every shape that
/// renders empty — nested value, root value, sequence item, flow pair value, flow item — is asked
/// here, each pinned to the text the reader answers with. A tagged empty is deliberately not in
/// this table: the tag suppresses resolution, so its body is spelled `''` instead, and
/// `a_tagged_empty_flow_member_with_a_note_settles_in_one_round` holds that one.
#[test]
fn an_empty_plain_scalar_settles_as_the_null_text() {
    use pyrs_yaml_core::ast::ScalarStyle;
    use pyrs_yaml_core::editing::Segment;
    use pyrs_yaml_core::editing::plan::{set_anchor_path, set_scalar_style_path};
    use std::borrow::Cow;

    let k = || Segment::Key(Cow::Borrowed("k"));
    let a = || Segment::Key(Cow::Borrowed("a"));
    for (src, path, expected) in [
        ("- >\n", vec![Segment::Index(0)], "- ~\n"),
        ("k: >\n", vec![k()], "k: ~\n"),
        ("k:\n  a: >\n", vec![k(), a()], "k:\n  a: ~\n"),
        ("k: {a: ''}\n", vec![k(), a()], "k: {a: ~}\n"),
        ("k: ['']\n", vec![k(), Segment::Index(0)], "k: [~]\n"),
    ] {
        let mut node = parse(src, Schema::Core).expect("the pinned input parses");
        set_scalar_style_path(&mut node, &path, ScalarStyle::Plain, src, None)
            .expect("the style is set");
        let one = to_yaml(&node);
        assert_eq!(one, expected, "the empty body has to be spelled: {src:?}");
        let again = emit(&one);
        assert_eq!(
            again, one,
            "not a fixed point in one round: {one:?} -> {again:?}"
        );
    }

    let src = "- >\n";
    let path = [Segment::Index(0)];
    let mut node = parse(src, Schema::Core).expect("the pinned input parses");
    set_scalar_style_path(&mut node, &path, ScalarStyle::Plain, src, None)
        .expect("the style is set");
    set_anchor_path(&mut node, &path, "a3", src, None).expect("the anchor is set");
    let one = to_yaml(&node);
    assert_eq!(
        one, "- &a3 ~\n",
        "an anchor names the node, it does not change what the node resolves to: {one:?}"
    );
    assert_eq!(emit(&one), one, "not a fixed point in one round: {one:?}");
}

/// libFuzzer `edit_yaml` does reach the root: the empty path navigates to the document node, so a
/// tag and a note are both verbs the edit layer accepts there.
///
/// A root sequence has no line of its own to host a note — its first line is a tag line and its last
/// belongs to its last item, and granit spans a node through the break that ends its own line. The
/// first emission used to park the note on that item's dash line, the re-read filed it as the item's
/// own, and the second emission moved it above the block: `!fuzz0 \n- !a\n# fuzz 11\n- !a\n` became
/// `# fuzz 11\n!fuzz0 \n- !a\n- !a\n`, one round short of a fixed point. `write_container_node` now
/// asks whether the pending line is the block's last and, for a sequence, writes the note above the
/// block's first line — where `lift_first_item_notes` already reads a sequence's own note from. The
/// shapes whose last line does carry a scalar body keep the inline slot and settle by staying put;
/// there the note's owner moves to that item, which is the quality-ledger (eo) ruling, so the table
/// pins the text rather than the ownership.
#[test]
fn the_roots_own_note_settles_in_one_round() {
    use pyrs_yaml_core::ast::{Comment, Tag};
    use pyrs_yaml_core::editing::Segment;
    use pyrs_yaml_core::editing::plan::{set_comment_path, set_tag_path};
    use std::sync::Arc;

    let path: [Segment; 0] = [];
    for (src, expected) in [
        ("- !a\n- !a\n", "# fuzz 11\n!fuzz0 \n- !a\n- !a\n"),
        ("!a\n- x\n", "!fuzz0 \n- x  # fuzz 11\n"),
        (
            "- &b\n- !a\n- k: v\n",
            "!fuzz0 \n- &b ~\n- !a\n- k: v  # fuzz 11\n",
        ),
    ] {
        let mut node = parse(src, Schema::Core).expect("the pinned input parses");
        set_tag_path(&mut node, &path, Tag::local("fuzz0"), src, None)
            .expect("the root takes a tag");
        let note = "fuzz 11";
        set_comment_path(
            &mut node,
            &path,
            Comment {
                text: Arc::from(note),
                standalone: false,
            },
            src,
            None,
        )
        .expect("the root takes a note");
        let one = to_yaml(&node);
        assert_eq!(one, expected, "the root's note has a pinned line: {src:?}");
        let again = emit(&one);
        assert_eq!(
            again, one,
            "not a fixed point in one round: {one:?} -> {again:?}"
        );
    }
}

/// libFuzzer `edit_yaml` crash-2d978c7b: the first item of an anchored sequence is itself a
/// headed sequence whose own inline note cannot take its last item's tail slot.
///
/// The item's note goes above its own header, i.e. between the outer sequence's `&a3` header
/// and its body. The reader files a note there on the *outer* sequence, so the second emission
/// hoists it above `&a3`: round one wrote `&a3` / `-` / `# fuzz 95` / `&a0 !fuzz2`, round two
/// `# fuzz 95` / `&a3` / `-` / `&a0 !fuzz2`. Above the outer header is where the reader reports
/// it from, so the first emission has to be the fixed point.
#[test]
fn a_headed_items_note_settles_above_the_outer_sequences_header() {
    use pyrs_yaml_core::ast::Comment;
    use pyrs_yaml_core::editing::Segment;
    use pyrs_yaml_core::editing::plan::set_comment_path;
    use std::sync::Arc;

    let src = "- &a3\n  - &a0 !fuzz2\n    - {\"-(.0bb:\": v6}  # fuzz 47\n";
    let mut node = parse(src, Schema::Core).expect("the pinned input parses");
    let item = [Segment::Index(0), Segment::Index(0)];
    set_comment_path(
        &mut node,
        &item,
        Comment {
            text: Arc::from("fuzz 95"),
            standalone: false,
        },
        src,
        None,
    )
    .expect("the note is set");

    let one = to_yaml(&node);
    let again = emit(&one);
    assert_eq!(
        again, one,
        "not a fixed point in one round: {one:?} -> {again:?}"
    );
}

#[test]
fn a_mapping_note_over_a_nested_sequence_settles_in_one_round() {
    use pyrs_yaml_core::ast::Comment;
    use pyrs_yaml_core::editing::Segment;
    use pyrs_yaml_core::editing::plan::set_comment_path;
    use std::sync::Arc;

    let path: [Segment; 0] = [];
    for (src, expected) in [
        ("ss:\n  - !a\n  - !a\n", "# fuzz 35\nss:\n  - !a\n  - !a\n"),
        (
            "a: 1\nss:\n  - !a\n  - !a\n",
            "a: 1\n# fuzz 35\nss:\n  - !a\n  - !a\n",
        ),
        (
            "ss:\n  - !a\n  - !a\nz: 2\n",
            "ss:\n  - !a\n  - !a\nz: 2  # fuzz 35\n",
        ),
    ] {
        let mut node = parse(src, Schema::Core).expect("the pinned input parses");
        set_comment_path(
            &mut node,
            &path,
            Comment {
                text: Arc::from("fuzz 35"),
                standalone: false,
            },
            src,
            None,
        )
        .expect("the root takes a note");
        let one = to_yaml(&node);
        assert_eq!(
            one, expected,
            "the mapping's note has a pinned line: {src:?}"
        );
        let again = emit(&one);
        assert_eq!(
            again, one,
            "not a fixed point in one round: {one:?} -> {again:?}"
        );
    }
}
