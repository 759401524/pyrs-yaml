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
    LONG_RECORDS, LONG_VALUE, MEDIUM_JSON, MEDIUM_JSON5, MEDIUM_JSONC, MEDIUM_TOML, MEDIUM_YAML,
    long_string_json, long_string_json5, long_string_jsonc, long_string_ndjson, long_string_toml,
    long_string_yaml, short_string_ndjson,
};
use pyrs_yaml_core::editing::navigate::Segment;
use pyrs_yaml_core::editing::plan::{delete_path, insert_path, set_path};
use pyrs_yaml_core::parser::parse;
use pyrs_yaml_core::parser::yaml::Schema;
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
