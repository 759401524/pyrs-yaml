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

/// libFuzzer `yaml_roundtrip` crash-6cfbaa57: the same inline slot, on a container that is itself
/// an explicit key.
///
/// The header lift moved the note above the `&x` line *inside* the `?` body, where the next read
/// hands it to the key mapping - and an explicit pair writes a key mapping's own note above the
/// marker. Round one therefore disagreed with round two. `hoist_marker_note` now takes the slot
/// wherever the container writes a header, so the first emission already spells the note where a
/// re-read puts it: above the `?`, which is the marker-level fixed point.
///
/// The header is one condition, not the marker alone: a headerless container key keeps that note
/// inside its body while the note can stay on the entry's own line, because a note is the only
/// thing telling two otherwise equal mapping keys apart
/// (`pbt::tests::the_key_rule_agrees_with_the_writer_and_the_reader`). When the entry's value
/// takes the line's trailing slot the note cannot stay there - see the test below.
#[test]
fn a_first_entry_inline_note_under_a_marker_settles_in_one_round() {
    let one = emit("? &x\n ? a  # c\n");
    assert_eq!(one, "# c\n?\n  &x \n  a: ~\n:\n  ~\n");
    assert_eq!(emit(&one), one, "not a fixed point in one round");
    let src = "bg: &b\n ? &!) yaAmlx> 2\n?  &?  \n ? &!) y &?  #!!\"#!\"chg0#~-.I";
    let one = emit(src);
    assert!(
        one.contains("!!\"#!\"chg0#~-.I"),
        "the note was dropped: {one:?}"
    );
    let again = emit(&one);
    assert_eq!(
        again, one,
        "not a fixed point in one round: {one:?} -> {again:?}"
    );
}

/// libFuzzer `yaml_roundtrip` crash-430f4f0e, minimized to 85 bytes: a first entry's inline note
/// inside a headerless marker body, where the entry's *value* owns the line's trailing slot.
///
/// The pair writer then has no slot for the key's note and spells it on a line of its own above
/// the entry. Inside a `?` body the reader reports that line on the key mapping, not on the entry,
/// so round two wrote it above the marker - the first emission was not the fixed point. The marker
/// lift takes the slot whenever the value takes the line, without needing an anchor or tag.
#[test]
fn a_first_entrys_note_climbs_above_the_marker_when_its_value_takes_the_line() {
    let src = "?\n\n#?\n\n ?  \n#?\n  !?\n\n ?  1   |)?\r?  \n b<<: #*{+ &b?  !?*\n  ?eI? {+ &b *.. #&\n#ef irst";
    let one = emit(src);
    let again = emit(&one);
    assert_eq!(
        again, one,
        "not a fixed point in one round: {one:?} -> {again:?}"
    );
    assert!(
        one.contains("# *{+ &b?  !?*\n?\n  b<<: \"?eI? {+ &b *..\"  # &\n"),
        "the key note belongs above the marker the reader files it on: {one:?}"
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

/// libFuzzer `yaml_roundtrip` crash-094d4d6a, crash-8492f5ee and crash-701f76bf: a tagged block
/// sequence whose first item rides a note on its `?`/`-` marker spine. `lift_first_entry_notes`
/// hoists the whole spine for a mapping container, but the sequence lift read only the first item's
/// own stack, so a nested marker note stayed *below* the header line — where the reader hands it to
/// the sequence — and the emission only settled on its second pass. Now the sequence lift walks the
/// spine too. `crash-701f76bf` is the same leak at its smallest, eight bytes under a tag; its `&a`
/// twin is the anchor spelling of the same header. Equality alone would pass an emission that
/// dropped or doubled the note, so those two shapes also assert the note lands above the header,
/// exactly once.
#[test]
fn a_tagged_sequences_marker_note_settles_above_its_header() {
    for src in [
        "!\n-\n#?\n ?   #[\n ? &.?-\n#?\n ?   #[\n ? &.?# ~# ~",
        "!\n-\n#?\n ?   #[ ?-\n#?\n ?   #[ ?    &.?# ~",
    ] {
        let one = emit(src);
        let again = emit(&one);
        assert_eq!(
            again, one,
            "{src:?} not a fixed point in one round: {one:?} -> {again:?}"
        );
    }
    for src in ["!\n- ?\n#~", "&a\n- ?\n#~"] {
        let one = emit(src);
        assert!(
            one.starts_with("# ~\n"),
            "the note must be lifted above the header: {src:?} -> {one:?}"
        );
        assert_eq!(
            one.matches("# ~").count(),
            1,
            "the note appears exactly once: {src:?} -> {one:?}"
        );
        let again = emit(&one);
        assert_eq!(
            again, one,
            "{src:?} not a fixed point in one round: {one:?} -> {again:?}"
        );
    }
}

/// libFuzzer `edit_yaml` crash-439f5d2c: a tag on the sequence nested inside another sequence, then
/// a note on that sequence's first item, which is a block mapping.
///
/// `lift_first_item_notes` moved the first item's own leading stack above the sequence's tag line.
/// For an item the writer spells on lines of its own that is one level too high: round one put
/// `# fuzz 47` below the item's `-`, the re-read reported it on the item - which is where it
/// belongs - and the lift then wrote it above `!fuzz2`, where the reader files it on the *sequence*
/// and the emission only settles on its second pass. The lift leaves that stack in the body now for
/// an item that has a body line, and hoists it only for the compact spells, where the dash line is
/// the item's only line and the sequence is the only node that can report a note above it.
#[test]
fn a_body_items_note_stays_under_its_dash_beneath_a_tag_line() {
    use pyrs_yaml_core::ast::{Comment, Tag};
    use pyrs_yaml_core::editing::Segment;
    use pyrs_yaml_core::editing::plan::{set_comment_path, set_tag_path};
    use std::borrow::Cow;
    use std::sync::Arc;

    let src = "s:\n\n0X3:\n-   !a :s\n- - : a:  ! ";
    let mut node = parse(src, Schema::Core).expect("the pinned input parses");
    let tagged = [Segment::Key(Cow::Borrowed("0X3")), Segment::Index(1)];
    set_tag_path(&mut node, &tagged, Tag::local("fuzz2"), src, None).expect("the tag is set");
    let item = [
        Segment::Key(Cow::Borrowed("0X3")),
        Segment::Index(1),
        Segment::Index(0),
    ];
    set_comment_path(
        &mut node,
        &item,
        Comment {
            text: Arc::from("fuzz 47"),
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
    assert!(
        one.contains("    !fuzz2 \n    - \n      # fuzz 47\n      ~:\n        a: !\n"),
        "the note belongs to the item, below its own dash: {one:?}"
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

/// libFuzzer `yaml_roundtrip` crash-2c76568e, minimized by hand to 23 bytes.
///
/// The emission was not merely unstable, it was text this engine's own reader refuses: a mapping
/// whose first value is empty under a tag leaves its line pending, and closing that line inserts
/// the value's null text *before* the line start the next pair had already captured. `write_pair`
/// then placed the following key's inline note at that stale offset, splitting the line -
/// `~: ! ~` became `~: ! # b` plus a bare `~` at column zero, which re-reads as
/// `simple key expected ':'`. `note_line_above` now treats "is this a line start" as an invariant
/// rather than an assumption, because every insertion that moves a line moves its start by the same
/// amount, so the line a stale capture meant is the next one.
#[test]
fn a_key_note_above_a_pair_with_a_pending_tag_line_reparses_and_settles() {
    for (src, notes) in [(": !\n<<: #b\n<<: #b\n 3 #~", 3), (": !\nq: #b\n 3 #~", 2)] {
        let one = emit(src);
        let reparsed = parse(&one, Schema::Core);
        assert!(
            reparsed.is_ok(),
            "the writer emitted text its own reader refuses ({one:?}): {}",
            reparsed.err().map(|e| e.to_string()).unwrap_or_default()
        );
        assert_eq!(
            one.matches("# ").count(),
            notes,
            "one note per source comment survives to the emission: {one:?}"
        );
        let again = emit(&one);
        assert_eq!(
            again, one,
            "not a fixed point in one round: {one:?} -> {again:?}"
        );
    }
}

/// libFuzzer `yaml_roundtrip` crash-76e05a78, minimized to 25 bytes: a key's inline note that
/// cannot share its pair line with the value's own note, placed above a pair whose pending
/// tag-only line the key's leading note had already closed.
///
/// A pair captures the position its key line starts at before writing anything. Writing the key's
/// leading note runs `write_note_line`, which closes the pending tag-only line above the pair by
/// inserting the value's `~` at an offset *before* that position, and then appends the note line -
/// so the key's line has moved. The key's inline note, which has to go above the pair because the
/// value keeps the only trailing slot, was written at the stale offset: `# b` split the pending
/// value's line and displaced its `~` onto a bare line of its own, which this reader refuses
/// (`simple key expected ':'`). The key line is re-anchored after its leading notes now.
#[test]
fn a_key_note_above_a_closed_pending_tag_line_settles_in_one_round() {
    let minimal = ":\n#*\n<<:\n: !\n<<: #b\n e #~";
    let one = emit(minimal);
    assert_eq!(
        one, "~: ! ~\n# *\n# b\n<<: e  # ~\n",
        "the minimised shape has a pinned emission"
    );
    let again = emit(&one);
    assert_eq!(
        again, one,
        "not a fixed point in one round: {one:?} -> {again:?}"
    );

    let original = "gb:\n-<<:\n: !:\t\n<<: +\n#?*)b\n\t\t\n<<:\n\n?: #*\n : !:\t\n<<: #*b\n  \
                    :rue?  ? 0&3# #!!\"#~? 0? y:: \r\n\r\r\rbg: !::\t-)\t\n";
    let one = emit(original);
    let again = emit(&one);
    assert_eq!(
        again, one,
        "not a fixed point in one round: {one:?} -> {again:?}"
    );
}

/// libFuzzer `edit_yaml` crash-ad42b9cc, minimized to 14 bytes: a note the reader holds as a
/// container's *standalone* leading note when a later edit empties that container.
///
/// The reader never spells this shape — the note is written above the container's first pair, so
/// only emptying the body leaves it standing over nothing — and `is_compact_item` refused the
/// compact `- key: value` line for any value with a standalone note. An empty container is the
/// exception `write_mapping_pair` already makes for itself: it inlines `{}` and demotes the note to
/// the pair-line tail. So round one wrote the dash alone and round two, reading the demoted note,
/// took the dash line. The compactness test now follows the writer's own rule.
#[test]
fn a_note_on_an_empty_container_value_settles_in_one_round() {
    use pyrs_yaml_core::ast::Comment;
    use pyrs_yaml_core::editing::Segment;
    use pyrs_yaml_core::editing::plan::{delete_path, set_comment_path, set_flow_style_path};
    use std::sync::Arc;

    let src = "- : a:  !o?--- ";
    let mut text = src.to_string();
    let mut node = parse(src, Schema::Core).expect("the pinned input parses");
    for verb in 0..3 {
        let path = match verb {
            2 => vec![
                Segment::Index(0),
                Segment::Key("~".into()),
                Segment::Key("a".into()),
            ],
            _ => vec![Segment::Index(0), Segment::Key("~".into())],
        };
        let accepted = match verb {
            0 => set_comment_path(
                &mut node,
                &path,
                Comment {
                    text: Arc::from("fuzz 47"),
                    standalone: false,
                },
                &text,
                None,
            ),
            1 => set_flow_style_path(&mut node, &path, true, &text, None),
            _ => delete_path(&mut node, &path, &text, None),
        };
        accepted.expect("the pinned edit sequence is accepted");
        let emitted = to_yaml(&node);
        node = parse(&emitted, Schema::Core).expect("the emission re-reads");
        text = emitted;
    }

    let one = text;
    assert_eq!(
        one, "- ~: {}  # fuzz 47\n",
        "the dash line is taken on the first round"
    );
    let again = emit(&one);
    assert_eq!(
        again, one,
        "not a fixed point in one round: {one:?} -> {again:?}"
    );
}

/// The 2026-10-31 report's marker-header shape: a note under an `?`-keyed body's anchor/tag header
/// belongs above the marker, and the first emission already says so.
///
/// libFuzzer `yaml_roundtrip` crash-1bbd4d7f, minimized to the 12 bytes `? &b` above an indented
/// `? > #I`. The placement landed with the marker-note work that followed the run, and no named
/// test owned these five shapes, so a later rule that moves a marker body's notes had nothing to
/// break here; the input is archived as a `former-crash-*` seed for the fuzz job as well.
#[test]
fn a_note_under_a_marker_header_settles_above_the_marker() {
    for (src, emitted, notes) in [
        ("? &b\n ? > #I", "# I\n?\n  &b \n  \"\": ~\n:\n  ~\n", 1),
        ("? !t\n ? > #I", "# I\n?\n  !t \n  \"\": ~\n:\n  ~\n", 1),
        (
            "? &b !t\n ? > #I",
            "# I\n?\n  &b !t \n  \"\": ~\n:\n  ~\n",
            1,
        ),
        (
            "? &b\n # I\n ? k > #J",
            "# I\n# J\n?\n  &b \n  k >: ~\n:\n  ~\n",
            2,
        ),
        ("?\n ? > #I", "?\n  \"\": ~  # I\n:\n  ~\n", 1),
    ] {
        let one = emit(src);
        assert_eq!(
            one, emitted,
            "{src:?} did not emit the hoisted spelling: {one:?}"
        );
        assert_eq!(
            one.matches("# ").count(),
            notes,
            "the notes are placed, not copied or lost: {one:?}"
        );
        let again = emit(&one);
        assert_eq!(
            again, one,
            "not a fixed point in one round: {one:?} -> {again:?}"
        );
    }
}

/// libFuzzer `yaml_roundtrip` crash-2c76568e: the same tag-only line, with a key's inline note
/// arriving right after it.
///
/// `bg: !` writes no scalar text, so the pair's line stays open until something closes it by
/// inserting the value's null text; `close_pending_tag_line` does that from `write_note_line`. Those
/// bytes move the pair line right, and the offset remembered as that line's start had been taken
/// before the leading stack was written — so the key's inline note, which cannot take the tail slot
/// because its value carries one, was placed "above the pair" at a stale byte position, i.e. inside
/// the previous line: `bg: !: # *b` / `~` / `# *b` / `<<: …`. A reader refuses that text outright
/// (`simple key expected ':'`), which is worse than the one-round drift the tag-only line usually
/// costs. Recapturing the line start after the leading stack — the same fix the `?`-marker branch
/// needs — puts the note on a line of its own, and the first emission both re-parses and settles.
#[test]
fn a_key_note_behind_a_tag_only_line_settles_in_one_round() {
    for (src, emitted) in [
        (
            ": !\n<<: #*\n<<: #b\n :# #!\n",
            "~: ! ~\n# *\n# b\n<<: \":#\"  # !\n",
        ),
        (
            "bg: !:\t\n<<: #*b\n  :rue999999999: !:\t\n<<: #*b\n  :rue99999999999999999999999999999?  ? 0&3# #!!\"#~? 0?  &3# #!!\"#~",
            "bg: !: ~\n# *b\n# *b\n<<: \":rue99999999999999999999999999999?  ? 0&3#\"  # !!\"#~? 0?  &3# #!!\"#~\n",
        ),
    ] {
        let one = emit(src);
        assert_eq!(one, emitted, "the note has to sit on a line of its own");
        let again = emit(&one);
        assert_eq!(
            again, one,
            "not a fixed point in one round: {one:?} -> {again:?}"
        );
        assert_eq!(
            one.lines()
                .filter(|line| line.trim_start().starts_with("# "))
                .count(),
            2,
            "both notes that had to become lines of their own survive; an emission that folds one \
             is a loss even when it is stable: {one:?}"
        );
    }
}

/// libFuzzer `yaml_roundtrip` crash-49f44d18 (issue #739): an alias standing as a key is inlined
/// into the anchored null a folded merge entry left behind, and that null key then sits beside the
/// mapping's own null key.
///
/// `<<: &b` defines an anchor on a null the duplicate-merge fold drops, `<<: {a}` supplies the
/// merge, `:` is the mapping's null key and `? *b` names the dropped definition from key position.
/// `repair_orphaned_anchors` inlines it there, so one mapping ends up holding two null keys —
/// though the reader keeps exactly one (the last) and folds the other away on the next read. The
/// first emission therefore was not its own fixed point: `~: ~` vanished on round two.
#[test]
fn an_inlined_alias_key_folds_into_the_null_key_it_lands_on() {
    for (src, emitted) in [
        // libFuzzer `yaml_roundtrip` crash-49f44d18, 33 bytes, whose `?  ` fragment is the
        // mapping's own null key and whose `? *b` is the alias key.
        (
            "<<: &b\n<<:\r\n\n\n ?  !? |2 \r?  \n? *b",
            "!? \"\": ~\n&b ~: ~\n",
        ),
        // The same mechanism hand-reduced to 20 bytes, where the merge value is a flow mapping.
        ("<<: &b\n<<: {a}\n:\n? *b\n", "a: ~\n&b ~: ~\n"),
        ("<<: &b\n<<: {a: 1}\n:\n? *b", "a: 1\n&b ~: ~\n"),
    ] {
        let one = emit(src);
        assert_eq!(one, emitted, "the alias key folds where it lands");
        assert_eq!(
            one.matches("~: ~").count(),
            1,
            "one mapping holds one null key: {one:?}"
        );
        let again = emit(&one);
        assert_eq!(
            again, one,
            "not a fixed point in one round: {one:?} -> {again:?}"
        );
    }
}
