//! The third reader of the TOML IR, checked against the tree the same engine builds.
//!
//! `pyrs-toml` now describes one scan three ways: `from_toml` projects the intermediate
//! representation onto `CustomNode`, `parse_stream` reports it as events, and `read_into` hands its
//! values to a caller that builds its own shape - which is what `load_toml` does, and the reason it
//! no longer pays for the node model. The claim these tests hold is the architecture's: the three
//! descriptions cannot disagree, because one scanner decides and each route only reads the verdict.
//!
//! The token vocabulary is the one `tests/event_stream.rs` compares the stream with the tree on, so
//! a name, a scalar or a container reads the same in all three files.

use std::convert::Infallible;

use pyrs_ast::ast::{CustomNode, ScalarStyle, Tag};
use pyrs_toml::{
    ScalarProjection, TomlDialect, TomlReadError, TomlSink, from_toml_with_options, read_into,
};

/// Lower bound on corpus documents that reach the assertion, the same reason the round-trip and
/// JSON corpus twins carry one: a corpus that fails to load must not read as a pass.
const MIN_SEEDED_DOCUMENTS: usize = 16;

fn scalar_token(text: &str, style: ScalarStyle, tag: Option<&Tag>) -> String {
    let stated = match tag {
        Some(tag) => format!("<{}{}>", tag.handle, tag.suffix),
        None => String::new(),
    };
    format!("scalar[{style:?}]{stated}{text}")
}

/// The tree as an ordered token list, exactly as the stream comparison reads it.
fn tree_shape(node: &CustomNode, into: &mut Vec<String>) {
    match node {
        CustomNode::Mapping { pairs, .. } => {
            into.push("map-open".to_string());
            for (key, value) in pairs.iter() {
                tree_shape(key, into);
                tree_shape(value, into);
            }
            into.push("map-close".to_string());
        }
        CustomNode::Sequence { items, .. } => {
            into.push("seq-open".to_string());
            for item in items {
                tree_shape(item, into);
            }
            into.push("seq-close".to_string());
        }
        CustomNode::Scalar {
            value, style, meta, ..
        } => into.push(scalar_token(value.as_ref(), *style, meta.tag())),
        CustomNode::Null { .. } => into.push("scalar[Plain]null".to_string()),
        CustomNode::Alias { .. } => unreachable!("TOML writes no alias"),
    }
}

/// The same list written by the value reader.
struct Recorder {
    log: Vec<String>,
}

impl Recorder {
    fn read(src: &str, dialect: TomlDialect) -> Result<Vec<String>, TomlReadError<Infallible>> {
        let mut sink = Recorder { log: Vec::new() };
        read_into(src, dialect, &mut sink)?;
        Ok(sink.log)
    }
}

impl TomlSink for Recorder {
    type Error = Infallible;

    fn open_table(&mut self) -> Result<(), Infallible> {
        self.log.push("map-open".to_string());
        Ok(())
    }

    fn key(&mut self, name: std::sync::Arc<str>, style: ScalarStyle) -> Result<(), Infallible> {
        self.log.push(scalar_token(&name, style, None));
        Ok(())
    }

    fn scalar(&mut self, value: ScalarProjection) -> Result<(), Infallible> {
        let ScalarProjection {
            text, style, tag, ..
        } = value;
        self.log
            .push(scalar_token(text.as_str(), style, tag.as_ref()));
        Ok(())
    }

    fn open_array(&mut self) -> Result<(), Infallible> {
        self.log.push("seq-open".to_string());
        Ok(())
    }

    fn close_table(&mut self) -> Result<(), Infallible> {
        self.log.push("map-close".to_string());
        Ok(())
    }

    fn close_array(&mut self) -> Result<(), Infallible> {
        self.log.push("seq-close".to_string());
        Ok(())
    }
}

fn seeds() -> Vec<String> {
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/toml_roundtrip");
    let mut entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("seed corpus missing at {}: {e}", dir.display()))
        .map(|entry| entry.expect("readable dir entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "seed"))
        .collect::<Vec<_>>();
    entries.sort();
    entries
        .into_iter()
        .map(|path| {
            std::fs::read_to_string(path).expect("a seed the corpus names is readable as text")
        })
        .collect()
}

/// Every document the corpus holds that the AST route accepts, the value route accepts identically.
#[test]
fn the_value_reader_describes_what_the_tree_stores() {
    let mut compared = 0;
    for seed in seeds() {
        let Ok(tree) = from_toml_with_options(&seed, TomlDialect::V1_1) else {
            continue;
        };
        let mut expected = Vec::new();
        tree_shape(&tree, &mut expected);
        let actual = Recorder::read(&seed, TomlDialect::V1_1)
            .expect("a document the tree route projects is a document the value route reads");
        assert_eq!(
            actual, expected,
            "the two readers disagree about\n{seed}\nvalue route: {actual:?}\ntree route: {expected:?}"
        );
        compared += 1;
    }
    assert!(
        compared >= MIN_SEEDED_DOCUMENTS,
        "only {compared} corpus documents reached the comparison; the seed corpus has to be \
         measured, not assumed"
    );
}

/// A document the two routes agree about, in either direction, and a refusal that reaches no value.
#[test]
fn the_two_routes_share_one_verdict() {
    let mut refused = 0;
    for seed in seeds() {
        for dialect in [TomlDialect::V1_0, TomlDialect::V1_1] {
            let tree = from_toml_with_options(&seed, dialect).is_ok();
            match (tree, Recorder::read(&seed, dialect)) {
                (true, Ok(log)) => assert!(
                    log.len() >= 2,
                    "a valid document reached the sink as {log:?} under {dialect:?}"
                ),
                (false, Err(TomlReadError::Parse(_))) => refused += 1,
                (false, Ok(log)) => panic!(
                    "the tree route refuses this {dialect:?} document and the value route walked {} \
                     events: {seed:?}\n{log:?}",
                    log.len()
                ),
                (true, Err(TomlReadError::Parse(_))) => panic!(
                    "the tree route projects this {dialect:?} document and the value route refused it: \
                     {seed:?}"
                ),
                (_, Err(TomlReadError::Sink(never))) => match never {},
            }
        }
    }
    assert!(
        refused > 0,
        "no seed in the corpus is refused by either dialect"
    );
}

/// The scan decides before any value exists, so a refused document never reaches a caller.
#[test]
fn a_refused_document_reaches_no_value() {
    let src = "a = \n";
    assert!(
        from_toml_with_options(src, TomlDialect::V1_1).is_err(),
        "the tree route must refuse the sample this test reads"
    );
    let mut sink = Recorder { log: Vec::new() };
    match read_into(src, TomlDialect::V1_1, &mut sink) {
        Err(TomlReadError::Parse(_)) => assert!(
            sink.log.is_empty(),
            "the sink saw {:?} before the refusal",
            sink.log
        ),
        other => panic!("the value reader accepted {src:?}: {other:?}"),
    }
}

/// The shapes a corpus built for round-trip text does not guarantee: every tag the hub states, the
/// style a multi-line string keeps, a name quoted or dotted, and the sequence an array-of-tables is.
#[test]
fn pinned_documents_state_what_the_reader_hands_over() {
    // A key named `n` leaves the scanner `DoubleQuoted`, because the hub quotes any name a YAML 1.1
    // reader would type. Both readers ask the same `toml_key_style`, so this records which answer
    // they agree on rather than what a caller might expect a TOML name to look like.
    let cases: [(&str, &[&str]); 11] = [
        (
            "bin = 0b1101\n",
            &[
                "map-open",
                "scalar[Plain]bin",
                "scalar[Plain]<!!int>0b1101",
                "map-close",
            ],
        ),
        (
            "hex = 0xFF\n",
            &[
                "map-open",
                "scalar[Plain]hex",
                "scalar[Plain]0xFF",
                "map-close",
            ],
        ),
        (
            "plain = 7\n",
            &[
                "map-open",
                "scalar[Plain]plain",
                "scalar[Plain]7",
                "map-close",
            ],
        ),
        (
            "when = 1979-05-27T07:32:00Z\n",
            &[
                "map-open",
                "scalar[Plain]when",
                "scalar[Plain]<!!timestamp>1979-05-27T07:32:00Z",
                "map-close",
            ],
        ),
        (
            "only_time = 07:32:00\n",
            &[
                "map-open",
                "scalar[Plain]only_time",
                "scalar[Plain]<!time>07:32:00",
                "map-close",
            ],
        ),
        (
            "multi = \"\"\"line1\nline2\"\"\"\n",
            &[
                "map-open",
                "scalar[Plain]multi",
                "scalar[Literal]line1\nline2",
                "map-close",
            ],
        ),
        (
            "\"quoted key\" = 1\n",
            &[
                "map-open",
                "scalar[Plain]quoted key",
                "scalar[Plain]1",
                "map-close",
            ],
        ),
        (
            "\"true\" = 1\n",
            &[
                "map-open",
                "scalar[DoubleQuoted]true",
                "scalar[Plain]1",
                "map-close",
            ],
        ),
        (
            "[a]\n[a.b]\nc = 1\n",
            &[
                "map-open",
                "scalar[Plain]a",
                "map-open",
                "scalar[Plain]b",
                "map-open",
                "scalar[Plain]c",
                "scalar[Plain]1",
                "map-close",
                "map-close",
                "map-close",
            ],
        ),
        (
            "[[t]]\nn = 1\n[[t]]\nn = 2\n",
            &[
                "map-open",
                "scalar[Plain]t",
                "seq-open",
                "map-open",
                "scalar[DoubleQuoted]n",
                "scalar[Plain]1",
                "map-close",
                "map-open",
                "scalar[DoubleQuoted]n",
                "scalar[Plain]2",
                "map-close",
                "seq-close",
                "map-close",
            ],
        ),
        (
            "inline = { x = 1, y = [2, 3] }\n",
            &[
                "map-open",
                "scalar[Plain]inline",
                "map-open",
                "scalar[Plain]x",
                "scalar[Plain]1",
                "scalar[DoubleQuoted]y",
                "seq-open",
                "scalar[Plain]2",
                "scalar[Plain]3",
                "seq-close",
                "map-close",
                "map-close",
            ],
        ),
    ];
    for (src, expected) in cases {
        let tree = from_toml_with_options(src, TomlDialect::V1_1)
            .expect("each pinned document is valid TOML");
        let mut shaped = Vec::new();
        tree_shape(&tree, &mut shaped);
        let actual = Recorder::read(src, TomlDialect::V1_1)
            .expect("a document the tree route projects is a document the value route reads");
        assert_eq!(actual, shaped, "the two readers disagree about {src:?}");
        assert_eq!(
            shaped,
            expected.to_vec(),
            "the hub's answer for {src:?} moved"
        );
    }
}
