//! The instruction gate's fixtures, pinned to what the tree still does with them.
//!
//! `from_json_medium` and `from_toml_medium` measure the reading half of each cross-format
//! bridge from committed bytes instead of bytes rendered at run time, so that a change in how
//! the writers *spell* their output cannot silently change what the reader is measured on: it
//! has to come through this file, fail, and be re-derived on purpose.
//!
//! What this file does **not** explain is the movement that was found while building it.
//! Rendering the input in setup (so the writer's cost cancels in the subtraction) put
//! `to_json_medium` +1.52% and `to_toml_medium` +0.26% above `main` with no engine code
//! changed - past the gate's 0.5% line on scenarios the addition never enters. The first
//! explanation offered here was the extra call site; committing the bytes removed the call and
//! moved `to_json_medium` +1.489% anyway, so that explanation is refuted by the pair of
//! measurements and the mechanism is recorded as unresolved. What is settled is the property:
//! a harness edit is not a neutral act for the numbers the same harness already produces, so a
//! re-baseline after one separates method from code by being written down, not by being
//! ignored. `serialize_*` and `parse_*` in that same binary stayed inside 0.042%.
use pyrs_json::{
    StreamEventType, from_json, from_json5, from_jsonc, parse_documents, to_json_text,
    to_json5_text, to_jsonc_text,
};
use pyrs_toml::{from_toml, to_toml};
use pyrs_yaml_core::ast::CustomNode;
use pyrs_yaml_core::bench_inputs::{
    ANCHOR_YAML, BLOCK_STYLE_YAML, DIALECT_NOTES_YAML, LONG_RECORDS, LONG_VALUE, MEDIUM_JSON,
    MEDIUM_JSON5, MEDIUM_JSONC, MEDIUM_TOML, MEDIUM_YAML, MERGE_INLINE_YAML, METADATA_EDIT_YAML,
    NOTE_FLOW_YAML, NOTES_JSON, NOTES_TOML, SMALL_YAML, long_string_json, long_string_json5,
    long_string_jsonc, long_string_ndjson, long_string_toml, long_string_yaml, short_string_ndjson,
};
use pyrs_yaml_core::editing::navigate::Segment;
use pyrs_yaml_core::editing::plan::{delete_path, insert_path, set_path};
use pyrs_yaml_core::parser::yaml::Schema;
use pyrs_yaml_core::parser::{parse, parse_stream};
use pyrs_yaml_core::serializer::to_yaml;
use std::borrow::Cow;

#[test]
fn the_json_reader_fixture_is_what_the_writer_emits() {
    let ast = parse(MEDIUM_YAML, Schema::Core).expect("the YAML fixture parses");
    let emitted = to_json_text(&ast).expect("json writes the hub AST");
    assert_eq!(
        emitted, MEDIUM_JSON,
        "MEDIUM_JSON no longer matches `to_json_text`"
    );
    from_json(MEDIUM_JSON).expect("the JSON reader fixture parses");
}

#[test]
fn the_toml_reader_fixture_is_what_the_writer_emits() {
    let ast = parse(MEDIUM_YAML, Schema::Core).expect("the YAML fixture parses");
    let emitted = to_toml(&ast).expect("toml writes the hub AST");
    assert_eq!(
        emitted, MEDIUM_TOML,
        "MEDIUM_TOML no longer matches `to_toml`"
    );
    from_toml(MEDIUM_TOML).expect("the TOML reader fixture parses");
}

/// The two dialect fixtures cannot be pinned to a writer the way the strict ones are: the
/// instruction gate hands `to_jsonc_medium` and `to_json5_medium` the AST built from `MEDIUM_YAML`,
/// which carries no comments, so deriving the fixtures from that writer would emit text with the
/// feature removed - and the feature is the whole reason the scenario exists. What is pinned
/// instead is the three things that make the measurement mean something: the bytes contain the
/// dialect, the reader accepts it, and it reaches the same values as the strict fixture.
#[test]
fn the_jsonc_fixture_carries_comments_and_the_same_values_as_strict_json() {
    let comments = MEDIUM_JSONC.matches("//").count() + MEDIUM_JSONC.matches("/*").count();
    assert!(
        comments >= 3,
        "the JSONC fixture lost its comments ({comments}); it would measure the strict path"
    );
    let dialect = from_jsonc(MEDIUM_JSONC).expect("the JSONC reader fixture parses");
    let strict = from_json(MEDIUM_JSON).expect("the strict JSON fixture parses");
    assert_eq!(
        to_json_text(&dialect).expect("jsonc reads to the hub AST"),
        to_json_text(&strict).expect("json reads to the hub AST"),
        "MEDIUM_JSONC no longer means what MEDIUM_JSON means"
    );
    for needle in [
        "boot sequence",
        "checked against the image",
        "operators touch",
    ] {
        assert!(
            !to_json_text(&dialect).unwrap().contains(needle),
            "comment text {needle:?} leaked into the data"
        );
    }
}

#[test]
fn the_json5_fixture_carries_its_grammar_and_the_same_values_as_strict_json() {
    assert!(
        MEDIUM_JSON5.contains("server:{") && !MEDIUM_JSON5.contains("\"server\""),
        "every JSON5 key went back to being quoted"
    );
    assert!(
        MEDIUM_JSON5.contains("'localhost'"),
        "no single-quoted strings left"
    );
    assert!(
        MEDIUM_JSON5.contains("',]")
            && MEDIUM_JSON5.contains(",\n  features")
            && MEDIUM_JSON5.contains("false,"),
        "the trailing commas were removed; that is the terminator path the scenario measures"
    );
    let dialect = from_json5(MEDIUM_JSON5).expect("the JSON5 reader fixture parses");
    let strict = from_json(MEDIUM_JSON).expect("the strict JSON fixture parses");
    assert_eq!(
        to_json_text(&dialect).expect("json5 reads to the hub AST"),
        to_json_text(&strict).expect("json reads to the hub AST"),
        "MEDIUM_JSON5 no longer means what MEDIUM_JSON means: the bare keys, the single-quoted \
         strings and the hexadecimal port must resolve to the same names and numbers"
    );
}

/// The spelling property that broke the first draft of `MEDIUM_JSON5`, pinned so the day it changes is
/// noticed rather than discovered by a red gate. `3e1` is legal JSON, so the JSON writer hands it
/// back unchanged: the two documents agree in value and differ in bytes. That is the good case - the
/// bad one is a literal YAML has no word for, which is issue #312.
#[test]
fn json5_exponent_spelling_survives_as_a_number_but_not_as_the_same_bytes() {
    let dialect = from_json5("{a: 3e1}").expect("an exponent literal reads");
    let strict = from_json("{\"a\": 30}").expect("the plain spelling reads");
    let written = to_json_text(&dialect).expect("json writes the hub AST");
    assert!(
        written.contains("3e1"),
        "the JSON writer stopped carrying an exponent spelling JSON can express: {written}"
    );
    assert_ne!(
        written,
        to_json_text(&strict).expect("json writes the hub AST"),
        "value equality here goes through a preserved literal, so identical bytes would mean the \
         literal was canonicalised and the fixture's parity claim changed meaning"
    );
}

/// The writers the two new scenarios call, read back by their own readers. Instruction counts say
/// how much the dialect writer costs; nothing here said its output was still valid in its own
/// dialect, which is the fidelity claim the objective makes about JSONC and JSON5.
#[test]
fn the_dialect_writers_emit_text_their_own_readers_accept() {
    let ast = parse(MEDIUM_YAML, Schema::Core).expect("the YAML fixture parses");
    let jsonc = to_jsonc_text(&ast).expect("jsonc writes the hub AST");
    let json5 = to_json5_text(&ast).expect("json5 writes the hub AST");
    let reference = to_json_text(&ast).expect("json writes the hub AST");
    assert_eq!(
        to_json_text(&from_jsonc(&jsonc).expect("the JSONC writer output re-reads"))
            .expect("the re-read JSONC document writes as JSON"),
        reference,
        "to_jsonc_text emitted something that does not read back to the same document"
    );
    assert_eq!(
        to_json_text(&from_json5(&json5).expect("the JSON5 writer output re-reads"))
            .expect("the re-read JSON5 document writes as JSON"),
        reference,
        "to_json5_text emitted something that does not read back to the same document"
    );
}

/// The long-string pair, pinned to the one property its scenarios exist to measure: values of
/// `LONG_VALUE` characters, with no byte the bulk string scan stops on inside one. A stray escape,
/// quote or control byte would hand those bytes back to the per-decision path, and the floor would
/// then silently stop describing the vector - which is the failure mode this file exists to catch.
#[test]
fn the_long_string_fixtures_differ_from_the_medium_ones_in_value_length_only() {
    let json = long_string_json();
    let toml = long_string_toml();

    let json_doc = from_json(&json).expect("the long-string JSON fixture parses");
    let toml_doc = from_toml(&toml).expect("the long-string TOML fixture parses");
    assert_eq!(
        to_json_text(&json_doc).expect("json writes what it read"),
        to_json_text(&toml_doc).expect("toml writes what it read"),
        "the two long-string fixtures no longer mean the same record list",
    );

    for (label, doc) in [("json", &json), ("toml", &toml)] {
        let longest = doc.split('"').map(str::len).max().unwrap_or(0);
        assert_eq!(
            longest, LONG_VALUE,
            "{label}: the longest quoted run is {longest}, not the {LONG_VALUE} the scenario names"
        );
        assert!(
            !doc.contains('\\'),
            "{label}: an escape sequence entered the fixture, so its bytes are no longer all bulk-scanned"
        );
        assert!(
            doc.bytes().all(|b| b >= 0x20 || b == b'\n'),
            "{label}: a control byte other than a line break entered the fixture"
        );
    }

    // The contrast the pair is measured against: the medium fixtures stay short, so a movement
    // between the two pairs of scenarios is a movement in value length and nothing else.
    let medium_longest = MEDIUM_JSON.split('"').map(str::len).max().unwrap_or(0);
    assert!(
        medium_longest * 8 < LONG_VALUE,
        "MEDIUM_JSON's longest value grew to {medium_longest}; from_json_medium now measures the bulk scan too"
    );
}

/// The document the writer halves of the two long-string bridges read. It carries the same record
/// list as the two reader fixtures, and its plain scalars are `LONG_VALUE` characters wide, so
/// `to_json_long_strings` and `to_toml_long_strings` spend their bytes inside the same shape the
/// readers are measured on. Plain YAML scalars carry no quotes, so the width question has a
/// different spelling here: the run after `": "` on each key line is the value.
#[test]
fn the_long_string_yaml_fixture_carries_the_same_records_at_the_same_width() {
    let yaml = long_string_yaml();
    let ast = parse(&yaml, Schema::Core).expect("the long-string YAML fixture parses");
    assert_eq!(
        to_json_text(&ast).expect("yaml writes the hub AST"),
        to_json_text(&from_json(&long_string_json()).expect("the JSON fixture parses"))
            .expect("json writes what it read"),
        "the YAML fixture no longer means the record list the two reader fixtures carry"
    );

    let widest = yaml
        .lines()
        .map(|line| line.len() - line.find(": ").map_or(0, |i| i + 2))
        .max()
        .unwrap_or(0);
    assert_eq!(
        widest, LONG_VALUE,
        "the longest scalar run is {widest}, not the {LONG_VALUE} the scenario names"
    );
    assert!(
        yaml.bytes().all(|b| b >= 0x20 || b == b'\n'),
        "a control byte other than a line break entered the YAML fixture"
    );

    let medium_widest = MEDIUM_YAML
        .lines()
        .map(|line| line.len() - line.find(": ").map_or(0, |i| i + 2))
        .max()
        .unwrap_or(0);
    assert!(
        medium_widest * 8 < LONG_VALUE,
        "MEDIUM_YAML's longest value grew to {medium_widest}; to_json_medium now measures the long-value shape too"
    );
}

/// The two dialect fixtures, pinned the same way and additionally pinned to carrying comments. The
/// point of `from_jsonc_long_strings` and `from_json5_long_strings` is that a reader does the bulk
/// string scan *between* comment skips, and JSON5's body is scanned by the single-quote
/// instantiation of the vector rather than the double-quote one. Strip the comments, or let a value
/// shorten, and the scenario quietly stops measuring the path its name claims.
#[test]
fn the_long_string_dialect_fixtures_carry_comments_at_their_own_width() {
    let reference = to_json_text(&from_json(&long_string_json()).expect("the JSON fixture parses"))
        .expect("json writes what it read");
    let jsonc = long_string_jsonc();
    let json5 = long_string_json5();

    assert_eq!(
        to_json_text(&from_jsonc(&jsonc).expect("the long-string JSONC fixture parses"))
            .expect("jsonc writes what it read"),
        reference,
        "the JSONC fixture no longer means the record list the strict JSON fixture carries"
    );
    assert_eq!(
        to_json_text(&from_json5(&json5).expect("the long-string JSON5 fixture parses"))
            .expect("json5 writes what it read"),
        reference,
        "the JSON5 fixture no longer means the record list the strict JSON fixture carries"
    );

    for (label, doc) in [("jsonc", &jsonc), ("json5", &json5)] {
        assert!(
            doc.contains("//") && doc.contains("/*"),
            "{label}: no comment entered the fixture, so the scenario stopped reading the comment path"
        );
        assert!(
            !doc.contains('\\'),
            "{label}: an escape sequence entered the fixture, so its bytes are no longer all bulk-scanned"
        );
        assert!(
            doc.bytes().all(|b| b >= 0x20 || b == b'\n'),
            "{label}: a control byte other than a line break entered the fixture"
        );
    }

    let double = jsonc.split('"').map(str::len).max().unwrap_or(0);
    assert_eq!(
        double, LONG_VALUE,
        "jsonc: the longest double-quoted run is {double}, not the {LONG_VALUE} the scenario names"
    );
    let single = json5.split('\'').map(str::len).max().unwrap_or(0);
    assert_eq!(
        single, LONG_VALUE,
        "json5: the longest single-quoted run is {single}, not the {LONG_VALUE} the scenario names"
    );
}

/// The two line-delimited fixtures, pinned to the record list the strict JSON fixture already
/// carries. This is the route where a fixture can stop being what its scenario name claims without
/// anything above noticing: the line surface re-enters the scanner once per record, so a record
/// that spilled onto a second line, or a pair whose values stopped differing only in width, would
/// leave the pair's difference measuring something other than the bulk scan. The reassembly
/// assertion is the one that cannot be satisfied by accident.
#[test]
fn the_ndjson_fixtures_are_the_same_records_cut_into_lines_at_two_widths() {
    const SHORT_VALUE: usize = "localhost".len();
    let long = long_string_ndjson();
    let short = short_string_ndjson();

    let inner = long
        .lines()
        .map(|line| {
            line.strip_prefix('{')
                .and_then(|body| body.strip_suffix('}'))
        })
        .collect::<Option<Vec<_>>>()
        .expect("every line of the fixture is one braced record");
    assert_eq!(
        format!("{{{}}}", inner.join(",")),
        long_string_json(),
        "the line-delimited fixture no longer carries the record list the AST scenario reads"
    );

    for (label, doc) in [("long", &long), ("short", &short)] {
        assert_eq!(
            doc.lines().count(),
            LONG_RECORDS,
            "{label}: a record no longer fits one line, so the per-record restart the scenario \
             prices is not its own"
        );
        let events = parse_documents(doc).expect("the line-delimited fixture parses");
        let documents = events
            .iter()
            .filter(|event| matches!(event.event_type, StreamEventType::DocumentEnd))
            .count();
        assert_eq!(
            documents, LONG_RECORDS,
            "{label}: the route reported {documents} documents, not one per line for the \
             {LONG_RECORDS} lines the fixture has"
        );
        assert!(
            !doc.contains('\\'),
            "{label}: an escape sequence entered the fixture, so its bytes are no longer all bulk-scanned"
        );
        assert!(
            doc.bytes().all(|b| b >= 0x20 || b == b'\n'),
            "{label}: a control byte other than a line break entered the fixture"
        );
    }

    let widest = |doc: &str| doc.split('"').map(str::len).max().unwrap_or(0);
    assert_eq!(
        widest(&long),
        LONG_VALUE,
        "long: the widest value is {}, not the {LONG_VALUE} the scenario names",
        widest(&long)
    );
    assert_eq!(
        widest(&short),
        SHORT_VALUE,
        "short: the widest value is {}, so the pair no longer contrasts a short body with a long one",
        widest(&short)
    );
    assert_eq!(
        long.len() - short.len(),
        LONG_RECORDS * (LONG_VALUE - SHORT_VALUE),
        "the two line-delimited fixtures differ by more than value width, so the gap between their \
         scenario numbers is not the price of a long body"
    );
}

/// The three edit scenarios in `benches/ir_gate.rs` price one verb each. A verb whose path stopped
/// resolving would still loop, still return a number, and report it as a speedup - the same
/// one-sided gate the readers above are pinned against. The harness exits 3 if that happens during a
/// run; this asks the question before the run, so the failure names the script instead of a machine.
#[test]
fn the_edit_scripts_reach_their_paths_and_change_the_text() {
    let base = parse(MEDIUM_YAML, Schema::Core).expect("the YAML fixture parses");
    let untouched = to_yaml(&base);

    let mut set = base.clone();
    set_path(
        &mut set,
        &[
            Segment::Key(Cow::Borrowed("server")),
            Segment::Key(Cow::Borrowed("port")),
        ],
        CustomNode::plain_scalar("9090"),
        true,
        MEDIUM_YAML,
        None,
        false,
    )
    .expect("edit_set_nested: `server.port` has to resolve, or the scenario prices a refusal");
    assert_ne!(
        to_yaml(&set),
        untouched,
        "edit_set_nested no longer changes the document, so it measures a no-op"
    );

    let mut inserted = base.clone();
    insert_path(
        &mut inserted,
        &[
            Segment::Key(Cow::Borrowed("logging")),
            Segment::Key(Cow::Borrowed("outputs")),
        ],
        1,
        CustomNode::plain_scalar("stderr"),
        MEDIUM_YAML,
        None,
    )
    .expect(
        "edit_insert_sequence: `logging.outputs` has to be a sequence, or the scenario prices a \
             refusal",
    );
    assert_ne!(
        to_yaml(&inserted),
        untouched,
        "edit_insert_sequence no longer changes the document, so it measures a no-op"
    );

    let mut deleted = base.clone();
    delete_path(
        &mut deleted,
        &[
            Segment::Key(Cow::Borrowed("features")),
            Segment::Key(Cow::Borrowed("rate_limit")),
        ],
        MEDIUM_YAML,
        None,
    )
    .expect(
        "edit_delete_key: `features.rate_limit` has to resolve, or the scenario prices a \
             refusal",
    );
    assert_ne!(
        to_yaml(&deleted),
        untouched,
        "edit_delete_key no longer changes the document, so it measures a no-op"
    );
}

/// `edit_set_metadata` and `edit_delete_notes` exist to price work the three floors above cannot
/// reach, so the premise is a property of the fixture and each branch needs its own answer: the note,
/// the anchor and the tag all have to land on the replacement, and the delete has to widen over a
/// comment that is really there. The last assertion is the control - `MEDIUM_YAML` must stay free of
/// all three marks, because if someone decorates the shared fixture then `edit_set_nested` stops
/// being the bare case and the pair no longer says which half moved.
#[test]
fn the_metadata_edit_fixture_reaches_every_carriage_branch() {
    let base = parse(METADATA_EDIT_YAML, Schema::Core).expect("the fixture parses");
    let untouched = to_yaml(&base);
    assert!(
        untouched.contains("toggled by ops"),
        "the standalone comment `features.rate_limit` is deleted under is not in the document, so \
         the widening check below could pass on an empty region:\n{untouched}"
    );

    let mut set = base.clone();
    set_path(
        &mut set,
        &[
            Segment::Key(Cow::Borrowed("server")),
            Segment::Key(Cow::Borrowed("port")),
        ],
        CustomNode::plain_scalar("9090"),
        true,
        METADATA_EDIT_YAML,
        None,
        false,
    )
    .expect("edit_set_metadata: `server.port` has to resolve, or the scenario prices a refusal");
    let carried = to_yaml(&set);
    assert_ne!(
        carried, untouched,
        "edit_set_metadata no longer changes the document, so it measures a no-op"
    );
    for (what, needle) in [
        ("the replacement value", "9090"),
        ("the carried anchor", "&P"),
        ("the carried tag", "!int"),
        ("the carried note", "live value"),
        ("the alias that needs that anchor", "*P"),
    ] {
        assert!(
            carried.contains(needle),
            "edit_set_metadata lost {what}: `{needle}` is missing from the emitted document:\n{carried}"
        );
    }
    parse(&carried, Schema::Core)
        .expect("the edited document re-parses, so the carried anchor left no dangling alias");

    let mut deleted = base.clone();
    delete_path(
        &mut deleted,
        &[
            Segment::Key(Cow::Borrowed("features")),
            Segment::Key(Cow::Borrowed("rate_limit")),
        ],
        METADATA_EDIT_YAML,
        None,
    )
    .expect(
        "edit_delete_notes: `features.rate_limit` has to resolve, or the scenario prices a \
         refusal",
    );
    let widened = to_yaml(&deleted);
    assert_ne!(
        widened, untouched,
        "edit_delete_notes no longer changes the document, so it measures a no-op"
    );
    assert!(
        !widened.contains("toggled by ops"),
        "the key is gone but its standalone comment survived, so `extend_delete_over_comments` \
         widened over an empty region again:\n{widened}"
    );

    assert!(
        !MEDIUM_YAML.contains('#') && !MEDIUM_YAML.contains('&') && !MEDIUM_YAML.contains('!'),
        "MEDIUM_YAML now carries metadata, so `edit_set_nested` and `edit_delete_key` are no longer \
         the bare half of the pair and the two new floors need re-reading"
    );
}

/// The TOML note floors' premise, in both directions.
///
/// `from_toml_notes` reads this document and `to_toml_notes` writes the dialect-note document out as
/// TOML. Neither floor measures anything if the note machinery is handed nothing to do, and a reader
/// that stopped collecting comments would keep returning `Ok`, keep looping and keep printing a
/// number - so the check is the fixed point the dialect note tests already demand: read it, write it
/// back, and every note text must return.
#[test]
fn the_toml_note_fixture_carries_its_comments_and_the_values_of_the_plain_one() {
    let notes = [
        "boot order is fixed by the entrypoint",
        "the loopback name, not an address",
        "the pool size is checked against the container image",
        "five workers, two spare",
        "logging format is fixed by the collector",
        "both sinks stay unrotated",
        "toggled by ops, last so no key is left to claim it",
    ];
    assert_eq!(
        NOTES_TOML.matches('#').count(),
        notes.len(),
        "the fixture's comment count and the note list no longer agree"
    );
    let read = from_toml(NOTES_TOML).expect("the TOML reader fixture parses");
    let written = to_toml(&read).expect("TOML writes what its reader built");
    // Six of the seven come back. The seventh is the file's last line - an own-note no key or header
    // follows - and `pyrs_toml` names that at its own boundary: the pair it hands back is
    // `(table, every own-line note no key or header claimed)`, because the AST has no slot for those.
    // The fixture keeps the line anyway, since the reader still walks it and the leftover still has to
    // be collected, and the assert below pins the shape so that giving the tail a slot shows up as a
    // decision here rather than as a floor that moved on its own.
    for note in &notes[..notes.len() - 1] {
        assert!(
            written.contains(note),
            "the TOML round trip lost {note:?}:\n{written}"
        );
    }
    assert!(
        !written.contains(notes[notes.len() - 1]),
        "the trailing own-note found an AST slot, so this premise changed shape:\n{written}"
    );
    // The comments are the only difference from `MEDIUM_TOML`, and JSON has nowhere to put a
    // comment, so comparing the JSON renderings compares the values and nothing else.
    assert_eq!(
        to_json_text(&read).expect("json writes the commented tree"),
        to_json_text(&from_toml(MEDIUM_TOML).expect("plain TOML parses"))
            .expect("json writes the plain tree"),
        "NOTES_TOML no longer means what MEDIUM_TOML means"
    );
}

/// `to_toml_notes` writes the note-bearing YAML document out as TOML. The TOML writer has a slot for
/// a document note, a pair note, a section-header note and an item note, and `DIALECT_NOTES_YAML`
/// carries one of each - plus a standalone note above a key of a nested map, which TOML has to write as
/// an inline table. That one used to be dropped outright: the hub homes it on the mapping node and the
/// inline-table arm looked only at what a member carries, so the table stayed on one line and the
/// note had nowhere to be written. All six arrive now, and they arrive at a text that re-reads to
/// itself - which is the property a demotion owes, since a multi-line inline table is 1.1 syntax and
/// only a note is allowed to take a table there.
#[test]
fn the_dialect_note_fixture_reaches_the_toml_writer_where_toml_has_a_line_for_it() {
    let ast = parse(DIALECT_NOTES_YAML, Schema::Core).expect("the note fixture parses as YAML");
    let written = to_toml(&ast).expect("TOML writes the hub AST");
    for note in [
        "root leading note",
        "trailing on a pair",
        "note on the pair the value follows",
        "above the inner key",
        "item note",
        "note on a flow value",
    ] {
        assert!(
            written.contains(note),
            "toml dropped the note {note:?}:\n{written}"
        );
    }
    // The note that forces the demotion sits inside the braces, which is where this crate's reader
    // reports a table-interior leading note from - so the second emission finds it in the slot its
    // own first emission chose, and the text settles in one hop.
    assert!(
        written.contains("nested = {\n  # above the inner key\n"),
        "the inline-table note landed somewhere else, so the fixture's premise moved:\n{written}"
    );
    let again = to_toml(&from_toml(&written).expect("the emitted TOML reads back"))
        .expect("and writes back out");
    assert_eq!(
        written, again,
        "the note's placement is not a fixed point:\n{again}"
    );
}

#[test]
fn the_json_stream_stop_leaves_a_tail_to_skip_and_skips_it() {
    // `stream_json_stop` asks the sink to stop at a quarter of the events this document emits, so
    // the scenario prices a prefix and a refusal to read the rest. Both halves are premises: a
    // fixture that shrank to four events would make the quarter zero, and one whose tail the
    // producer ignores would report the full walk under a name that says otherwise.
    let total = pyrs_json::parse_stream(MEDIUM_JSON)
        .expect("stream_json_medium: MEDIUM_JSON has to stream, or the scenario prices an error")
        .len();
    let limit = total / 4;
    assert!(
        limit >= 1 && limit < total,
        "MEDIUM_JSON emits {total} events, so stopping at a quarter prices \
         {limit} of them - either nothing or everything"
    );

    let mut delivered = 0usize;
    let mut sink = |event: pyrs_json::StreamEvent<'_>| {
        let _ = event;
        delivered += 1;
        if delivered == limit {
            std::ops::ControlFlow::Break(())
        } else {
            std::ops::ControlFlow::Continue(())
        }
    };
    pyrs_json::parse_stream_with_options_to(
        MEDIUM_JSON,
        pyrs_json::JsonParseOptions::STRICT,
        &mut sink,
    )
    .expect("stream_json_stop: the stopping sink must not turn the document into an error");
    assert_eq!(
        delivered, limit,
        "the sink saw {delivered} events where the stop asks for {limit} of {total}, so \
         `ControlFlow::Break` no longer ends the walk"
    );
}

#[test]
fn the_note_flow_fixture_reaches_every_answer_the_writer_gives() {
    // `serialize_note_flow` prices the note-shape decisions the other five serialize scenarios never
    // take: `SMALL_YAML`, `MEDIUM_YAML`, `BLOCK_STYLE_YAML`, `BLOCK_SCALAR_YAML` and `ANCHOR_YAML` hold
    // demotion nor either lift runs. Each assertion below is one of the five answers that walk can
    // give. A fixture that quietly stopped producing one would keep the floor green while measuring a
    // document that no longer reaches the code the scenario is named for.
    let ast = parse(NOTE_FLOW_YAML, Schema::Core).expect("the note-flow fixture parses");
    let once = to_yaml(&ast);
    let again = to_yaml(&parse(&once, Schema::Core).unwrap_or_else(|error| {
        panic!("the fixture emitted text that will not re-parse: {error}")
    }));
    assert_eq!(
        once, again,
        "the fixture's shape needs two emissions to settle, so the scenario measures a document the \
         reader cannot hand back:\nfirst:\n{once}\nsecond:\n{again}"
    );

    // Braces kept with the note still on the line: the walk visited every member and said yes.
    assert!(
        once.contains("kept: [1, {b: 2}]  # fits"),
        "the accepted flow spelling is gone, so the walk no longer runs to a verdict:\n{once}"
    );
    // Braces abandoned for the member carrying two notes: the walk said no and the container demoted.
    assert!(
        once.contains("over:\n  - \n") && !once.contains("over: ["),
        "the two-note member no longer forces the demotion, so the block path is unmeasured:\n{once}"
    );
    // A `?` marker inside braces, with the note lifted onto the marker rather than into the body.
    assert!(
        once.contains("? [k1, k2]:"),
        "the marker form is gone, so `hoist_marker_note` and `spine_has_notes` price nothing:\n{once}"
    );
    // A complex key with no note of its own, whose note rides the first item of its body: the lift
    // walks down the spine to find it, and both the `Sequence` arms - the mutable one and its
    // read-only guard - only run on this shape. This is the pair PR #641 had to fix.
    assert!(
        once.contains("spine:\n  # hoisted above the marker\n  ?\n"),
        "the note no longer lands above the marker, so `spine_has_notes` and `hoist_marker_note`'s \
         sequence arm price nothing:\n{once}"
    );
    // The anchored header whose first entry carries a note: the lift that puts it above the dash.
    assert!(
        once.contains("lift: &A\n  # first entry note\n"),
        "the header lift no longer fires, so the two `lift_first_*_notes` helpers are unmeasured:\n{once}"
    );
}

#[test]
fn the_note_flow_fixture_reaches_both_readers_and_the_other_read_fixtures_do_not() {
    // `parse_notes` and `stream_notes` price the reader's comment machinery: `comment.rs`'s scan,
    // `AstReceiver::on_comment_event` and `StreamReceiver`'s own note carriage. Before them no
    // scenario read a document holding a note - `serialize_note_flow` and the two dialect writers
    // emit notes, the edit verbs move them, but nothing measured the cost of *finding* them.
    // The premise of a floor is the shape that reaches it, so both halves are asserted here: this
    // fixture still carries eleven notes through each reader, and the fixtures the older read rows
    // use still carry none. A drift in either direction turns the pair back into a number that
    // guards a comment-free document while its name says otherwise.
    let notes_in = NOTE_FLOW_YAML.matches('#').count();
    assert_eq!(
        notes_in, 11,
        "the fixture's note count moved, so the two floors' premise needs restating"
    );

    let ast = parse(NOTE_FLOW_YAML, Schema::Core).expect("the note-flow fixture parses");
    let emitted = to_yaml(&ast);
    assert_eq!(
        emitted.matches('#').count(),
        notes_in,
        "the AST route lost or invented a note, so `parse_notes` no longer reads what it names:\n{emitted}"
    );
    assert_eq!(
        to_yaml(&parse(&emitted, Schema::Core).expect("the emission re-parses")),
        emitted,
        "the fixture is not a fixed point, so the scenario measures a document mid-settlement"
    );

    let events = parse_stream(NOTE_FLOW_YAML).expect("the note-flow fixture streams");
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(&e.event_type, StreamEventType::Comment { .. }))
            .count(),
        notes_in,
        "the event route reported a different note count than the AST route, so `stream_notes` is \
         no longer its twin"
    );

    let long_yaml = long_string_yaml();
    for (name, fixture) in [
        ("parse_small", SMALL_YAML),
        ("parse_medium", MEDIUM_YAML),
        ("parse_block", BLOCK_STYLE_YAML),
        ("parse_anchors", ANCHOR_YAML),
        ("parse_inline_merge", MERGE_INLINE_YAML),
        ("stream_long_strings", long_yaml.as_str()),
    ] {
        assert_eq!(
            fixture.matches('#').count(),
            0,
            "{name} now reads a note, so the pair above is no longer the only floor pricing \
             the reader's comment scan"
        );
    }
}

#[test]
fn the_dialect_note_fixture_reaches_both_dialect_writers() {
    // `to_jsonc_notes` and `to_json5_notes` exist because the two floors beside them do not see a
    // single note: `to_jsonc_medium` and `to_json5_medium` run over `MEDIUM_YAML`, which carries no
    // `#`, so `emit_root_leading`, `emit_standalone_comment` and the trailing-note sites of
    // `pyrs-json/src/writer.rs` had no price on either dialect. The premise is that this fixture
    // still reaches those sites - a note that a later writer change stopped carrying would otherwise
    // leave the two floors guarding a comment-free emission while their names say otherwise.
    let ast = parse(DIALECT_NOTES_YAML, Schema::Core).expect("the dialect fixture parses");
    for (dialect, text) in [
        (
            "jsonc",
            to_jsonc_text(&ast).expect("jsonc writes the fixture"),
        ),
        (
            "json5",
            to_json5_text(&ast).expect("json5 writes the fixture"),
        ),
    ] {
        assert!(
            text.contains("// root leading note"),
            "{dialect} no longer spells the document note as a comment:\n{text}"
        );
        for note in [
            "trailing on a pair",
            "note on the pair the value follows",
            "above the inner key",
            "item note",
            "note on a flow value",
        ] {
            assert!(
                text.contains(note),
                "{dialect} dropped the note {note:?}:\n{text}"
            );
        }
    }
}

#[test]
fn the_dialect_note_reader_fixture_reaches_both_dialect_readers() {
    // The reader twins of the two floors above, added because `quality_matrix.py::ir_bridge_pairs`
    // reads a `to_<format>_<shape>` without its `from_<format>_<shape>` as a half-measured bridge.
    // Their premise is that this document still resolves seven comments - five line-form, one
    // block-form, one before the document - into notes the tree carries. A reader that started
    // skipping comments would keep returning `Ok`, keep looping and keep printing a number, so the
    // floor would price a walk that records nothing; the check is the fixed point the dialect
    // property tests already demand, re-emit what the reader built and every text must come back.
    for (dialect, written) in [
        (
            "jsonc",
            to_jsonc_text(&from_jsonc(NOTES_JSON).expect("jsonc reads the fixture"))
                .expect("jsonc writes what its reader built"),
        ),
        (
            "json5",
            to_json5_text(&from_json5(NOTES_JSON).expect("json5 reads the fixture"))
                .expect("json5 writes what its reader built"),
        ),
    ] {
        for note in [
            "root leading note",
            "trailing on a pair",
            "block above a key",
            "note on the inner value",
            "note on the pair the value follows",
            "item note",
            "note on a flow value",
        ] {
            assert!(
                written.contains(note),
                "{dialect} reading lost the note {note:?}:\n{written}"
            );
        }
    }
}

/// How a fixture file is stored is not part of what the gate measures.
///
/// `crates/pyrs-yaml-core/src/bench_inputs.rs` is declared `-text` and all ten commits that touched it
/// stored it with CRLF, while the other `-text` path, `benches/ir_gate.rs`, is stored with LF - so a
/// helper script that rewrites one of them changes every line of its diff without intending to, and
/// `.gitattributes` leaves open whether that moves a measurement. For these fixtures it cannot, and
/// this is the measurement rather than the inference: rustc normalises CRLF inside raw strings, so no
/// fixture compiles to a CR byte and both trees hand the engines the same input. Without it, the
/// floors would describe whichever checkout the last author happened to commit.
#[test]
fn no_benchmark_fixture_compiles_to_a_carriage_return() {
    for (name, text) in [
        ("SMALL_YAML", SMALL_YAML),
        ("MEDIUM_YAML", MEDIUM_YAML),
        ("ANCHOR_YAML", ANCHOR_YAML),
        ("BLOCK_STYLE_YAML", BLOCK_STYLE_YAML),
        ("MERGE_INLINE_YAML", MERGE_INLINE_YAML),
        ("METADATA_EDIT_YAML", METADATA_EDIT_YAML),
        ("NOTE_FLOW_YAML", NOTE_FLOW_YAML),
        ("DIALECT_NOTES_YAML", DIALECT_NOTES_YAML),
        ("NOTES_JSON", NOTES_JSON),
        ("NOTES_TOML", NOTES_TOML),
        ("MEDIUM_JSON", MEDIUM_JSON),
        ("MEDIUM_JSONC", MEDIUM_JSONC),
        ("MEDIUM_JSON5", MEDIUM_JSON5),
        ("MEDIUM_TOML", MEDIUM_TOML),
        ("long_string_yaml", long_string_yaml().as_str()),
        ("long_string_toml", long_string_toml().as_str()),
        ("long_string_json", long_string_json().as_str()),
        ("long_string_jsonc", long_string_jsonc().as_str()),
        ("long_string_json5", long_string_json5().as_str()),
        ("long_string_ndjson", long_string_ndjson().as_str()),
        ("short_string_ndjson", short_string_ndjson().as_str()),
    ] {
        assert!(
            !text.contains('\r'),
            "{name} compiled to a CR byte, so a fixture file's stored line endings would be part of \
             the measured input"
        );
    }
}
