//! Every emitted document is a fixed point: `to_yaml(parse(to_yaml(x))) == to_yaml(x)`.
//!
//! That is the fuzz tier's contract (`fuzz/fuzz_targets/yaml_roundtrip.rs`), asserted there over
//! whatever the fuzzer explores and here over named shapes - because a bounded fuzz budget can miss
//! a rare drift for a run, and a change to a placement rule should break a named test rather than an
//! unlucky draw. Each input below was reached by libFuzzer and minimized with `cargo fuzz tmin`;
//! quality-ledger (ds) carries the mechanism and the two remedies that were measured away before the
//! one that holds.

use pyrs_yaml_core::ast::{CustomNode, NodeMap};
use pyrs_yaml_core::parser::parse;
use pyrs_yaml_core::parser::yaml::Schema;
use pyrs_yaml_core::serializer::{NotePlacement, SerializeOptions, to_yaml, to_yaml_with_options};

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

/// libFuzzer `edit_yaml` crash-92f3fc65 and crash-e392c402: a note on the entry under a tag header
/// whose block body is not indented.
///
/// `!base:` is a tag on the root, and its sequence starts on the next line at the same column. A
/// note written on that first entry has to take a line of its own, and the line the writer chose was
/// the gap between the header and the body - where granit does not report it: the re-read files it
/// as the root node's leading note, and the next emission lifts it above `!base: `. `crash-e392c402`
/// is the same shape with the root also carrying an anchor (`!base: &xuzz2eUt:`).
#[test]
fn a_note_in_the_gap_under_a_tag_header_settles_in_one_round() {
    use pyrs_yaml_core::ast::Comment;
    use pyrs_yaml_core::editing::Segment;
    use pyrs_yaml_core::editing::plan::{set_anchor_path, set_comment_path};
    use std::borrow::Cow;

    let src = "!base:\n  - etss: !s:\n\n";
    let item = [Segment::Index(0)];
    let value = [Segment::Index(0), Segment::Key(Cow::Borrowed("etss"))];
    let note = || Comment {
        text: "fuzz 47".into(),
        standalone: false,
    };

    let mut node = parse(src, Schema::Core).expect("the pinned input parses");
    set_anchor_path(&mut node, &value, "a0", src, None).expect("the anchor is set");
    let text = to_yaml(&node);

    let mut node = parse(&text, Schema::Core).expect("the intermediate text re-parses");
    set_comment_path(&mut node, &item, note(), &text, None).expect("the entry note is set");
    let text = to_yaml(&node);

    let mut node = parse(&text, Schema::Core).expect("the intermediate text re-parses");
    set_comment_path(&mut node, &value, note(), &text, None).expect("the value note is set");

    let one = to_yaml(&node);
    let again = emit(&one);
    assert_eq!(
        again, one,
        "not a fixed point in one round: {one:?} -> {again:?}"
    );
    assert_eq!(
        one.matches("# ").count(),
        2,
        "both notes survive the placement, and no round may invent or fold one: {one:?}"
    );
}

/// libFuzzer `yaml_roundtrip` crash-543366c4 and its siblings in the same batch: a note hoisted off
/// a nested marker lands in the gap under the enclosing pair's own header line.
///
/// `hoist_marker_note` brings a note that rides an inner marker up to the marker's level, which is
/// where the reader reports it. The lifted note was then appended at the writer's tail - the gap
/// below `&&~ !y5kg: ` - and granit spans a node through the break that ends its header line, so the
/// re-read files that note one level higher and the second emission lifts it above the header.
#[test]
fn a_hoisted_marker_note_does_not_land_in_the_header_gap() {
    let src = "!y5%6bg: &&~\n?\t\n\n z#<A : #*b\n  &: 1:&&z #*b\n\t";
    let one = emit(src);
    let again = emit(&one);
    assert_eq!(
        again, one,
        "not a fixed point in one round: {one:?} -> {again:?}"
    );
    assert_eq!(
        one.matches("# ").count(),
        2,
        "two notes are carried in, and a placement may not fold one away: {one:?}"
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

/// libFuzzer `yaml_roundtrip` crash-abc541ff and crash-c06e8a8f (2026-11-02 batch, issue #733): an
/// anchor whose name ends in `"` comes back under a different name.
///
/// The writer emits the recovered name verbatim (`&*..f!ny&!#\"`), and the key line's trailing
/// comment (`# &!`) sits behind it. On the next read the recovery scanner treated the name's closing
/// `"` as opening a quoted node, because `opens_quoted_node` accepted a quote flush against an anchor
/// token. The `#` behind it therefore no longer looked like a comment opener, the `&` inside that
/// comment was taken as the anchor, and the document re-emitted as `&!` one round after the first
/// emission.
#[test]
fn an_anchor_name_ending_in_a_quote_settles_in_one_round() {
    for src in [
        "<>: #&!\n &*..f!ny&!#\\\"\n\n :",
        "<&: #&!\n &*..f!ny&!#\\\"\n\n :",
    ] {
        let one = emit(src);
        let again = emit(&one);
        assert_eq!(
            again, one,
            "not a fixed point in one round: {one:?} -> {again:?}"
        );
        assert!(
            one.contains("&*..f!ny&!#\\\""),
            "the anchor name must survive the round: {one:?}"
        );
    }
}
/// libFuzzer `yaml_roundtrip` crash-fce042d0 (2026-11-02 batch, issue #733) and its 22-byte line
/// reduction: an alias key that resolves to a null the fold already displaced lands beside the
/// mapping's own null key, because the rebuild folded only keys it had rewritten.
///
/// `? &a` / `? *a` / `? &b ~` holds one anchored empty key, an alias to it, and an anchored null
/// key. The receiver folds the two anchored null keys and drops `&a`'s only definition, so
/// `repair_orphaned_anchors` inlines the alias key - and the rebuild then inserted that pair and
/// the plain pair without folding them against each other. Both are null keys, the writer spells
/// them `~:`, and the re-read kept only the last, so the first emission was not its own fixed
/// point: `&a ~: ~` / `&b ~: ~` came back as `&b ~: ~`.
#[test]
fn an_inlined_alias_key_folds_the_plain_null_key_that_follows_it() {
    for src in [
        "? &a
? *a
? &b ~
",
        " ? &.?-
 ? *.?-
 ?  &.",
    ] {
        let one = emit(src);
        let again = emit(&one);
        assert_eq!(
            again, one,
            "not a fixed point in one round: {one:?} -> {again:?}"
        );
        assert_eq!(
            one.matches("~: ~").count(),
            1,
            "one mapping holds one null key: {one:?}"
        );
    }
}

/// libFuzzer `edit_yaml` crash-547a9705, minimized to 18 bytes with `cargo fuzz tmin`.
///
/// The pair writer has one trailing slot per line and the value took it, so the key's note went
/// above the pair's own line - which is the first line of the `&a0` header's gap. granit spans a
/// node from its header, so that note re-reads as the *anchored sequence's* leading note and the
/// next emission lifts it above the header. `note_first` (the default) writes it where that read
/// already files it, so the edit settles in one round.
#[test]
fn a_note_above_a_pair_inside_a_header_gap_settles_in_one_round() {
    let emitted = gap_tree_emitted(NotePlacement::NoteFirst);
    assert_eq!(
        emitted, "- \n  - \n    # fuzz 47\n    &a0 \n    - ~: ! ~  # fuzz 47\n",
        "the lifted line is where the reader files the note"
    );
    let again = to_yaml(&parse(&emitted, Schema::Core).expect("the emission parses"));
    assert_eq!(
        emitted, again,
        "an edit's emission did not settle in one round\nfirst:\n{emitted}\nsecond:\n{again}"
    );
}

/// The two values answer differently, and only one of them is a fixed point. That is the whole
/// content of the choice handed back to the caller: `header_first` is the placement this writer used
/// to pick, kept reachable rather than quietly deleted, with its non-idempotence stated.
#[test]
fn note_placement_answers_differently_and_only_note_first_settles() {
    let lifted = gap_tree_emitted(NotePlacement::NoteFirst);
    let inside = gap_tree_emitted(NotePlacement::HeaderFirst);
    assert_ne!(lifted, inside, "two values, two answers");
    assert_eq!(
        inside, "- \n  - \n    &a0 \n    # fuzz 47\n    - ~: ! ~  # fuzz 47\n",
        "header_first keeps the note inside the gap"
    );
    let again = to_yaml(&parse(&inside, Schema::Core).expect("the emission parses"));
    assert_ne!(
        inside, again,
        "the gap placement is documented as not settling: {inside:?} -> {again:?}"
    );
}

/// The minimized crash script: anchor a block sequence, note its item, then note the item's value,
/// re-reading after each accepted edit exactly as the fuzz tier does, and emit under `placement`.
fn gap_tree_emitted(placement: NotePlacement) -> String {
    use pyrs_yaml_core::ast::Comment;
    use pyrs_yaml_core::editing::Segment;
    use pyrs_yaml_core::editing::plan::{set_anchor_path, set_comment_path};
    use std::borrow::Cow;
    use std::sync::Arc;

    let note = || Comment {
        text: Arc::from("fuzz 47"),
        standalone: false,
    };
    let source = "-  - - : !\n";
    let mut node = parse(source, Schema::Core).expect("the pinned input parses");
    let anchored = vec![Segment::Index(0), Segment::Index(0)];
    set_anchor_path(&mut node, &anchored, "a0", source, None).expect("the anchor is accepted");

    let text = to_yaml(&node);
    let mut node = parse(&text, Schema::Core).expect("the emission parses");
    let inner = vec![Segment::Index(0), Segment::Index(0), Segment::Index(0)];
    set_comment_path(&mut node, &inner, note(), &text, None).expect("the note is accepted");

    let text = to_yaml(&node);
    let mut node = parse(&text, Schema::Core).expect("the emission parses");
    let keyed = vec![
        Segment::Index(0),
        Segment::Index(0),
        Segment::Index(0),
        Segment::Key(Cow::Owned("~".to_string())),
    ];
    set_comment_path(&mut node, &keyed, note(), &text, None).expect("the note is accepted");

    to_yaml_with_options(
        &node,
        &SerializeOptions {
            note_placement: placement,
            ..Default::default()
        },
    )
    .expect("the tree serializes")
}

/// libFuzzer `edit_yaml` crash-b7e16929, minimized to 25 bytes with `cargo fuzz tmin`.
///
/// Three null keys, two of which fold by design, and the pair the fold drops is the one that
/// defined the anchor `*x` binds. The name still has a definition somewhere in the tree - it rides
/// the key of the `!k` pair, which lands *after* the alias - so the repair pass's "no remaining
/// definition" test left the alias alone, and the emission put `s: *x` above every `&x`. YAML binds
/// an alias to the nearest preceding definition, so that text is one this library's own reader
/// refuses.
#[test]
fn a_fold_that_drops_the_only_preceding_definition_emits_text_its_reader_accepts() {
    let src = ": &x\ns: *x\n!k &x :\n:\n";
    let emitted = emit(src);
    let back = parse(&emitted, Schema::Core);
    assert!(
        back.is_ok(),
        "the emission of {src:?} is text its own reader refuses: {emitted:?}"
    );
    let again = to_yaml(&back.expect("asserted above"));
    assert_eq!(
        again, emitted,
        "the accepted emission did not settle in one round\nfirst:\n{emitted}\nsecond:\n{again}"
    );
}

/// libFuzzer `yaml_roundtrip` crash-aa8a0447 and crash-95b66309: a content-less block scalar with
/// a note line in front of it.
///
/// The reader finds a block header by backing up from the content to the last non-blank line. With
/// no content line at all that backup stopped on the note, found no sigil there and answered Clip,
/// so the author's `+` survived the first emission and not the next. The pair that isolates it is
/// the same document with and without the note: `">+4\r#"` settled while
/// `" #Irrr\n>+4\r#"` did not. The scan now falls back to the sigil's own line, and only after the
/// backup finds no header above it.
#[test]
fn a_content_less_block_scalar_keeps_its_header_under_a_note_line() {
    for src in [" #Irrr\n>+4\r#", "#\"sq\r>+8\r#/:  ~~"] {
        let one = emit(src);
        let again = emit(&one);
        assert_eq!(
            again, one,
            "not a fixed point in one round: {one:?} -> {again:?}"
        );
    }
}

/// The writer's reserved-indicator set is `serializer::leading_indicator`, and the decision it
/// drives is pinned differentially in `serializer.rs::tests` against an independent restatement.
/// What nothing asked end to end is the claim the corpus used to stand for:
/// `fuzz/seeds/yaml_roundtrip/special-keys.seed` carries a bare `@literal`, and since `@` cannot
/// open a plain token the reader refuses the whole document, so the seed replays clean without ever
/// reaching the target's assertion (`.ci/seed-reach.txt` names it). A node built in memory — not
/// parsed from quoted source, which would merely preserve the quoting it was given — must be emitted
/// so that it re-reads as the same text on either side of the pair, in one round.
#[test]
fn a_plain_scalar_leading_with_a_reserved_indicator_survives_the_round_trip() {
    const INDICATORS: [char; 17] = [
        '-', '{', '}', '[', ']', '*', '&', '!', '?', '%', '@', '`', '\'', '"', '|', '>', ',',
    ];
    let scalar_text = |node: &CustomNode| match node {
        CustomNode::Scalar { value, .. } => value.as_ref().to_string(),
        other => panic!("expected a scalar, got {other:?}"),
    };

    for ch in INDICATORS {
        let text = format!("{ch}x");
        for (side, key_side) in [("key", true), ("value", false)] {
            let mut pairs = NodeMap::default();
            let (key, value) = if key_side {
                (
                    CustomNode::plain_scalar(text.as_str()),
                    CustomNode::plain_scalar("v"),
                )
            } else {
                (
                    CustomNode::plain_scalar("k"),
                    CustomNode::plain_scalar(text.as_str()),
                )
            };
            pairs.insert(key, value);
            let node = CustomNode::Mapping {
                pairs,
                flow_style: false,
                meta: Default::default(),
            };

            let one = to_yaml(&node);
            let again = parse(&one, Schema::Core).unwrap_or_else(|e| {
                panic!("{text:?} on the {side} side emitted {one:?}, which our reader refuses: {e}")
            });
            assert_eq!(
                to_yaml(&again),
                one,
                "{text:?} on the {side} side is not a fixed point in one round: {one:?}"
            );

            let CustomNode::Mapping { pairs, .. } = &again else {
                panic!("expected a mapping, got {one:?}");
            };
            let (got_key, got_value) = pairs
                .iter()
                .next()
                .unwrap_or_else(|| panic!("expected one pair, got {one:?}"));
            let got = scalar_text(if key_side { got_key } else { got_value });
            assert_eq!(
                got, text,
                "{text:?} on the {side} side re-read as {got:?} from {one:?}"
            );
        }
    }
}

/// libFuzzer `edit_yaml` crash-35c8809a / crash-e26004a5: a container note whose block ends on an
/// alias value's line.
///
/// `set_comment` on the mapping attaches an inline note to the container. The block's last pair
/// value is an alias, which carries no `NodeMeta`, so a note riding that pair's trailing slot is
/// re-read onto the pair's *key* - and the key's writer lifts it above the enclosing `&a2` header
/// (`note_line_above` measures the gap from the pair's line start, crossing the indented header
/// that the container path's tail-based scan cannot). Riding the slot inline therefore cost a
/// round. The container path now asks whether the block ended on an alias and, under a header,
/// writes the note above it on the first emission - the position the re-read files it under.
#[test]
fn a_container_note_over_an_alias_tail_settles_above_the_header() {
    use pyrs_yaml_core::ast::Comment;
    use pyrs_yaml_core::editing::Segment;
    use pyrs_yaml_core::editing::plan::set_comment_path;

    let src = "- &x v\n- &a2\n  k: *x\n";
    let container = [Segment::Index(1)];
    let note = Comment {
        text: "fuzz 239".into(),
        standalone: false,
    };

    let mut node = parse(src, Schema::Core).expect("the pinned input parses");
    set_comment_path(&mut node, &container, note, src, None).expect("the container note is set");

    let one = to_yaml(&node);
    let again = emit(&one);
    assert_eq!(
        again, one,
        "not a fixed point in one round: {one:?} -> {again:?}"
    );
    assert_eq!(
        one.matches("# ").count(),
        1,
        "the note survives the placement, and no round may invent or fold one: {one:?}"
    );
}

/// libFuzzer `edit_yaml` crash-12c96086: a column-0 note between two block-sequence items, the
/// first item's value a nested mapping whose `s:` is an empty tagged scalar `&a0 !s:`.
///
/// granit defers the empty scalar's event past the following comment line, so the reader used to
/// drain the pending `# fuzz 47` onto that scalar as a *leading* note even though the note sits a
/// line below it. The writer then hoisted the note above the `&a0 !s:` header. That hoisted form is
/// itself a stable fixed point, so a round-trip equality check alone passes on the buggy tree - the
/// guard must pin the note's *position*: it belongs at column 0 on the line before the next `- s:`
/// item, not above the first item's header.
///
/// The signal is whether a sibling follows, not the break character: a CR spelling of the same
/// document ingest to the same tree and must emit byte-for-byte the same. A reader guard keyed on
/// LF alone passed the LF seed yet hoisted the note on the CR spelling, so the CR arm is pinned here.
#[test]
fn a_column_zero_note_between_items_settles_in_one_round() {
    let lf = "- b- - s: &a0 !s:\n# fuzz 47\n- s: ~  # fuzz 47\n";
    let one = emit(lf);
    assert!(
        one.contains("\n# fuzz 47\n- s: ~"),
        "the note must ride the next item at column 0, not the empty scalar above it: {one:?}"
    );
    let again = emit(&one);
    assert_eq!(
        again, one,
        "not a fixed point in one round: {one:?} -> {again:?}"
    );
    let cr = lf.replace('\n', "\r");
    assert_eq!(
        emit(&cr),
        one,
        "a CR-only break must not change where the note lands: {one:?}"
    );
}

/// libFuzzer `yaml_roundtrip` crash-7eb273bc / crash-9733643a: a note on the line after an empty
/// tag-only value, with the document ending (`...`) right below it.
///
/// This is the doc-end arm of the same deferred-note decision as crash-12c96086 above: granit emits
/// the empty scalar's event after the comment line beneath it, and here no sibling follows, so the
/// note is the value's own leading note. A reader guard keyed on an LF break alone drained the CR
/// seed correctly but left the LF spelling's note pending - the writer then homed it inline on the
/// root, a different tree. A document differs from its CR/LF spelling only in how its lines break,
/// so both must emit byte-for-byte the same; that equality is the fence.
#[test]
fn a_doc_end_note_after_an_empty_value_settles_in_one_round() {
    for raw in [
        include_bytes!("../../../fuzz/seeds/yaml_roundtrip/former-crash-7eb273bc.seed"),
        include_bytes!("../../../fuzz/seeds/yaml_roundtrip/former-crash-9733643a.seed"),
    ] {
        let cr = std::str::from_utf8(raw).expect("seed is utf-8");
        let one = emit(cr);
        assert_eq!(emit(&one), one, "not a fixed point in one round: {one:?}");
        let lf = cr.replace('\r', "\n");
        assert_eq!(
            emit(&lf),
            one,
            "a CR-only break must not change where the note lands: {one:?}"
        );
    }
    assert_eq!(
        emit(": !\r#U\r... #-"),
        "# -\n~:\n  # U\n  !\n",
        "the below-note is the empty value's leading note at doc-end, not the root's inline slot"
    );
}

/// libFuzzer `edit_yaml` crash-92e7b87c: a leading note inside a deeply nested block sequence,
/// under an outer `&a0` header.
#[test]
fn a_nested_leading_note_settles_in_one_round() {
    let emitted = "- \n  # fuzz 47\n  # fuzz 119\n  &a0 \n  - \n    - \n      # fuzz 47\n      - !base: \"-?ss\": !s: ~  # fuzz 119\n";
    let again = emit(emitted);
    assert_eq!(
        again, emitted,
        "not a fixed point in one round: {emitted:?} -> {again:?}"
    );
}

/// libFuzzer `yaml_roundtrip` crash-cd7fab01, minimized to 26 bytes: an alias standing as a key in
/// a block mapping that is a sequence item.
///
/// The compact `- key: value` form writes every key through `write_scalar_for_key`, whose catch-all
/// spells any non-scalar `null`, so the alias key `*.?-` came out as a literal `null:` beside the
/// item's own `~:` null key. The re-read folded the two null keys into one (the duplicate-null-key
/// collapse) and dropped a pair, so the first emission was not its own fixed point:
/// `- ~: ~\n  null: ~` came back as `- null: ~`. `is_compact_item` admitted the alias key because its
/// guard excluded only mapping and sequence keys; an alias is not a simple key either (`*x: v` is
/// refused by this engine's reader), so it now takes the block-body form, where `write_mapping_pair`
/// emits `? *.?-` and the alias keeps its identity. The three other yaml_roundtrip inputs triaged
/// into this class (04f90374, 0733f4ec, 948c5dbf) already settle on main — class b/d closed them —
/// and are archived as seeds beside this one.
#[test]
fn an_alias_key_in_a_compact_sequence_item_settles_in_one_round() {
    // Item 0 anchors a null key `&.?-`; item 1 holds that null key plus an alias to it. The compact
    // form spelled the alias `null` and folded it into the `~` key on the next read.
    let src = "-\n ? &.?-\n-\n ? #[\n ? *.?-\n";
    let one = emit(src);
    assert_eq!(
        one, "- &.?- ~: ~\n- \n  # [\n  ~: ~\n  ? *.?-\n  : ~\n",
        "the alias key takes the `? *name` block-body form, not a folded `null:`"
    );
    assert_eq!(emit(&one), one, "not a fixed point in one round: {one:?}");

    // The full crash input, archived as a seed: the same alias-key fold across several items.
    let raw = include_bytes!("../../../fuzz/seeds/yaml_roundtrip/former-crash-cd7fab01.seed");
    let full = std::str::from_utf8(raw).expect("seed is utf-8");
    let one = emit(full);
    assert!(
        one.contains("? *.?-"),
        "the alias key must survive as an explicit marker, not be spelled null: {one:?}"
    );
    assert_eq!(emit(&one), one, "not a fixed point in one round: {one:?}");
}
