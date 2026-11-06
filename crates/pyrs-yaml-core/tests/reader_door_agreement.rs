//! The three ways the two YAML entry points are required to agree, as named cases.
//!
//! `fuzz/fuzz_targets/parse_yaml.rs` asks this contract of arbitrary bytes, which makes it a merge
//! gate over the corpus but not a named shape: when the contract moves, the fuzz target says only
//! "some input". These are the three branches with the five inputs that reach them, so the contract
//! `crates/pyrs-yaml-core/src/parser/mod.rs` states for `load_ast` has a test that says which half
//! broke. The fifth case is not a fourth branch - it pins the ordering that stops the
//! message-equality branch crying wolf when a stream's second document does not parse.

use pyrs_yaml_core::parser::yaml::Schema;
use pyrs_yaml_core::parser::{parse, parse_all};

/// The single-document door's refusal for a stream, by token rather than by prose.
fn single_refusal(text: &str) -> String {
    parse(text, Schema::Core)
        .err()
        .map(|error| error.to_string())
        .unwrap_or_else(|| panic!("the single door accepted {text:?}"))
}

#[test]
fn one_document_is_read_by_both_doors() {
    let stream = parse_all("a: 1\n", Schema::Core).expect("the stream door reads a pair");
    assert_eq!(stream.len(), 1, "one document, not {}", stream.len());
    assert!(
        parse("a: 1\n", Schema::Core).is_ok(),
        "the single door must accept what the stream door read: {}",
        single_refusal("a: 1\n")
    );
}

#[test]
fn no_document_at_all_is_not_a_second_document() {
    // The empty input is `null` to the single door and the empty stream to the other. Neither may
    // invent a boundary: an earlier reader of this pair would have called it a refusal.
    assert!(
        parse("", Schema::Core).is_ok(),
        "empty input must read as nothing, not fail"
    );
    assert_eq!(
        parse_all("", Schema::Core)
            .expect("the stream door reads empty input")
            .len(),
        0,
        "empty input must yield no documents rather than one null document"
    );
}

#[test]
fn a_second_document_answers_the_arity_refusal_not_a_syntax_defect() {
    use pyrs_yaml_core::parser::ParseError;

    for text in ["---\na: 1\n---\nb: 2\n", "a: 1\n...\nb: 2\n"] {
        let documents = parse_all(text, Schema::Core)
            .expect("the stream door reads the two documents")
            .len();
        assert!(documents > 1, "{text:?} read as {documents} document(s)");
        match parse(text, Schema::Core) {
            Err(ParseError::SingleDocumentExpected { .. }) => {}
            other => panic!(
                "the single door must answer SingleDocumentExpected for {text:?}, got {other:?}"
            ),
        }
    }
}

#[test]
fn both_doors_refuse_the_same_way_with_the_same_message() {
    for text in [
        "a: 1\n a: 2",
        "*x",
        "- 1\n- @two\n",
        "a: [1,\n",
        "x: [1, 2\n  y: 3",
    ] {
        let stream = parse_all(text, Schema::Core);
        let single = parse(text, Schema::Core);
        assert!(
            stream.is_err() && single.is_err(),
            "neither door may read {text:?}"
        );
        assert_eq!(
            single.as_ref().err().unwrap().to_string(),
            stream.as_ref().err().unwrap().to_string(),
            "the two doors refused {text:?} differently"
        );
    }
}

#[test]
fn a_second_document_that_does_not_parse_is_a_syntax_defect_at_both_doors() {
    // Measured, not documented: with a broken second document the scanner fails before the stream
    // reaches the second start marker, so both doors report the same syntax defect and the arity
    // refusal never appears. If that ordering changes, this case says so - and with it goes the
    // argument that the fuzz oracle cannot cry wolf.
    for text in [
        "a: 1\n---\n[1,\n",
        "a: 1\n...\nb: [1,\n",
        "---\nok\n---\n%\n",
    ] {
        let single = parse(text, Schema::Core);
        let stream = parse_all(text, Schema::Core);
        assert!(
            single.is_err() && stream.is_err(),
            "neither door may read {text:?}"
        );
        assert_eq!(
            single.as_ref().err().unwrap().to_string(),
            stream.as_ref().err().unwrap().to_string(),
            "a broken second document must be refused alike by {text:?}"
        );
    }
}
