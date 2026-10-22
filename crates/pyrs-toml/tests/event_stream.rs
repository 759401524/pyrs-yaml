//! The tree-free event stream, checked against the tree the same engine builds.
//!
//! `pyrs-toml` reads a document once and describes it twice: `from_toml` projects the scanner's
//! intermediate representation onto `CustomNode`, `parse_stream` reports the same representation as
//! `pyrs-event` events. These tests ask whether the two descriptions agree - which is the claim the
//! architecture makes - and pin the two places where a stream says something a tree has no field
//! for.

use pyrs_ast::ast::{CustomNode, ScalarStyle};
use pyrs_toml::{NoteOwner, TomlDialect, from_toml_with_options, parse_stream_with_dialect};

/// One scalar reduced to what both descriptions can state.
fn scalar_token(text: &str, style: ScalarStyle, tag: Option<&pyrs_ast::ast::Tag>) -> String {
    let stated = match tag {
        Some(tag) => format!("<{}{}>", tag.handle, tag.suffix),
        None => String::new(),
    };
    format!("scalar[{style:?}]{stated}{text}")
}

/// The tree as an ordered token list: every name, every scalar, every container in the order a
/// reader meets them. Decorations are left out here and counted by their own test, because the AST
/// discards a note that has no slot while the stream reports what the document wrote.
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

/// The same list read off the events, with the frame bookkeeping that tells a key's scalar from a
/// value's scalar removed - both descriptions list them in document order.
fn event_shape(events: &[pyrs_toml::StreamEvent<'_>]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match &event.event_type {
            pyrs_toml::StreamEventType::MappingStart { .. } => Some("map-open".to_string()),
            pyrs_toml::StreamEventType::MappingEnd => Some("map-close".to_string()),
            pyrs_toml::StreamEventType::SequenceStart { .. } => Some("seq-open".to_string()),
            pyrs_toml::StreamEventType::SequenceEnd => Some("seq-close".to_string()),
            pyrs_toml::StreamEventType::Scalar {
                value, style, tag, ..
            } => Some(scalar_token(value.as_ref(), *style, tag.as_ref())),
            _ => None,
        })
        .collect()
}

/// Every note in the tree, with the slot it sits in.
///
/// The tree's flag is the placement: `true` means the note rides the node's leading list, and the
/// node's inline slot is the one note that does not.
fn tree_notes(node: &CustomNode, into: &mut Vec<(String, bool)>) {
    if let Some(note) = node.comment() {
        into.push((note.text.to_string(), note.standalone));
    }
    for note in node.leading_comments() {
        into.push((note.text.to_string(), note.standalone));
    }
    match node {
        CustomNode::Mapping { pairs, .. } => {
            for (key, value) in pairs.iter() {
                tree_notes(key, into);
                tree_notes(value, into);
            }
        }
        CustomNode::Sequence { items, .. } => {
            for item in items {
                tree_notes(item, into);
            }
        }
        _ => {}
    }
}

/// Every note the stream reports, with the source's own fact and the owner it names.
fn stream_notes(events: &[pyrs_toml::StreamEvent<'_>]) -> Vec<(String, bool, NoteOwner)> {
    events
        .iter()
        .filter_map(|event| match &event.event_type {
            pyrs_toml::StreamEventType::Comment {
                text,
                owns_line,
                owner,
            } => Some((text.to_string(), *owns_line, *owner)),
            _ => None,
        })
        .collect()
}

/// The same notes read as the slot each one takes, which is what the tree and the stream agree on.
fn stream_slots(notes: &[(String, bool, NoteOwner)]) -> Vec<(String, bool)> {
    notes
        .iter()
        .map(|(text, _, owner)| (text.clone(), owner.is_leading()))
        .collect()
}

fn shapes_of(src: &str, dialect: TomlDialect) -> (Vec<String>, Vec<String>) {
    let tree =
        from_toml_with_options(src, dialect).unwrap_or_else(|e| panic!("{src} must parse: {e}"));
    let events = parse_stream_with_dialect(src, dialect)
        .unwrap_or_else(|e| panic!("{src} must stream: {e}"));
    let mut from_tree = Vec::new();
    tree_shape(&tree, &mut from_tree);
    (from_tree, event_shape(&events))
}

#[test]
fn the_stream_reports_a_document_in_structural_order() {
    //            0123456 7 8 9 10 11 12 13 14 15 16 17 18 19
    let src = "# lead\n[a]\nb.c = 1\n";
    let events = parse_stream_with_dialect(src, TomlDialect::V1_1).expect("valid TOML");
    let read: Vec<String> = events
        .iter()
        .map(|event| {
            let kind = match &event.event_type {
                pyrs_toml::StreamEventType::StreamStart => "stream-start".to_string(),
                pyrs_toml::StreamEventType::StreamEnd => "stream-end".to_string(),
                pyrs_toml::StreamEventType::DocumentStart => "document-start".to_string(),
                pyrs_toml::StreamEventType::DocumentEnd => "document-end".to_string(),
                pyrs_toml::StreamEventType::MappingStart { tag, .. } => {
                    format!("mapping-start[{}]", tag.is_some())
                }
                pyrs_toml::StreamEventType::MappingEnd => "mapping-end".to_string(),
                pyrs_toml::StreamEventType::SequenceStart { tag, .. } => {
                    format!("sequence-start[{}]", tag.is_some())
                }
                pyrs_toml::StreamEventType::SequenceEnd => "sequence-end".to_string(),
                pyrs_toml::StreamEventType::Scalar {
                    value,
                    style,
                    anchor,
                    tag,
                } => {
                    assert!(anchor.is_none(), "TOML has no anchor");
                    format!("scalar[{style:?}][tag={tag:?}]{value}")
                }
                pyrs_toml::StreamEventType::Alias { name } => {
                    panic!("TOML has no alias: {name}")
                }
                pyrs_toml::StreamEventType::Comment {
                    text,
                    owns_line,
                    owner,
                } => {
                    format!("comment[owns_line={owns_line},owner={owner:?}]{text}")
                }
            };
            format!("{kind}@{}:{}", event.line, event.column)
        })
        .collect();
    assert_eq!(
        read,
        [
            // The document opens where its bytes open.
            "stream-start@0:0",
            "document-start@0:0",
            "mapping-start[false]@0:0",
            // A header names its table inside the brackets that open it: the key sits at 8,
            // the `MappingStart` it belongs to at 7. Structural order, not byte order, is what
            // a consumer walks - and TOML is the only one of the five formats that inverts them.
            "scalar[Plain][tag=None]a@1:1",
            // The note above `[a]` leads the table, not its name: the tree files it on the
            // mapping the header opens, so the stream reports it after the name and before the
            // mapping starts - the one order in which the note's owner is the node a consumer
            // builds next.
            "comment[owns_line=true,owner=FollowingLeading]lead@0:0",
            "mapping-start[false]@1:0",
            // `b.c = 1`: the dotted key's own level, then the pair's name at 14 and its value
            // at 18, all on the line the document wrote them on.
            "scalar[Plain][tag=None]b@2:0",
            "mapping-start[false]@2:0",
            "scalar[Plain][tag=None]c@2:2",
            "scalar[Plain][tag=None]1@2:6",
            // A close carries the offset of the construct it closes: TOML wrote no end token.
            "mapping-end@2:0",
            "mapping-end@1:0",
            "mapping-end@0:0",
            "document-end@3:0",
            "stream-end@3:0",
        ]
    );
}

#[test]
fn the_two_readers_describe_the_same_document() {
    let cases: &[(TomlDialect, &str)] = &[
        (TomlDialect::V1_1, ""),
        (TomlDialect::V1_1, "a = 1"),
        (TomlDialect::V1_1, "[tbl]\nkey = \"value\"\n"),
        (TomlDialect::V1_1, "a.b.c = 1\na.d = 2\n"),
        (
            TomlDialect::V1_1,
            "[[fruit]]\nname = \"apple\"\n[[fruit]]\nname = \"pear\"\n",
        ),
        (
            TomlDialect::V1_1,
            "nums = [1, 2, [3, 4], {x = 1, y = [2]}]\n",
        ),
        (TomlDialect::V1_1, "inline = { a = 1, b = { c = 2 } }\n"),
        (TomlDialect::V1_1, "hex = 0x1F\nbin = 0b1010\noct = 0o755\n"),
        (
            TomlDialect::V1_1,
            "when = 2024-01-01\nstamp = 2024-01-01T10:00:00Z\ntime = 10:00:00\n",
        ),
        (TomlDialect::V1_1, "f = 1e10\ng = -0.5\nh = nan\ni = inf\n"),
        (
            TomlDialect::V1_1,
            "s = \"a\\nb\"\nl = 'a\\b'\nm = \"\"\"x\ny\"\"\"\n",
        ),
        (TomlDialect::V1_1, "\"1\" = 2\n\"\" = 3\nhost = \"h\"\n"),
        (TomlDialect::V1_1, "[a]\n[a.b]\n[a.b.c]\nx = 1\n"),
        (TomlDialect::V1_0, "a = 1\n[s]\nb = [1, 2]\n"),
    ];
    for (dialect, src) in cases {
        let (tree, stream) = shapes_of(src, *dialect);
        assert_eq!(tree, stream, "the two readers disagree about {src:?}");
    }
}

#[test]
fn every_note_the_tree_keeps_the_stream_reports_too() {
    let noted: &[&str] = &[
        "# above\na = 1 # after\n",
        "# one\n# two\nb = 2\n",
        "[s] # on the header\n# above the key\nk = 1 # after the value\n",
        "arr = [\n  # above the element\n  1, # after it\n]\n",
        "tbl = {\n  # above the member\n  k = 1 # after it\n}\n",
    ];
    for src in noted {
        let tree = from_toml_with_options(src, TomlDialect::V1_1).expect("valid TOML");
        let mut from_tree = Vec::new();
        tree_notes(&tree, &mut from_tree);
        let events = parse_stream_with_dialect(src, TomlDialect::V1_1).expect("valid TOML");
        let mut from_stream = stream_slots(&stream_notes(&events));
        // The stream's order is the one a consumer walks, the tree's is the one a node holds, so
        // the two are compared as the multisets they are.
        from_tree.sort();
        from_stream.sort();
        assert_eq!(from_tree, from_stream, "note sets differ for {src:?}");
        assert!(
            !from_tree.is_empty(),
            "the fixture stopped carrying a note at all: {src:?}"
        );
    }
}

/// The two facts a note carries: what the source did, and where the tree filed it.
#[test]
fn every_note_the_tree_files_is_reported_with_the_same_owner() {
    let cases: &[&str] = &[
        "# above\na = 1 # after\n",
        "# one\n# two\nb = 2\n",
        "[s] # on the header\n# above the key\nk = 1 # after the value\n",
        "arr = [\n  # above the element\n  1, # after it\n]\n",
        "tbl = {\n  # above the member\n  k = 1 # after it\n}\n",
        "[[p]]\n# above the element\nx = 1\n[[p]]\n# above the second\nx = 2\n",
        "a.b = 1 # after\n",
        "# lead\n[a]\nk = 1\n",
    ];
    for src in cases {
        let tree = from_toml_with_options(src, TomlDialect::V1_1).expect("valid TOML");
        let mut kept = Vec::new();
        tree_notes(&tree, &mut kept);
        let events = parse_stream_with_dialect(src, TomlDialect::V1_1).expect("valid TOML");
        let reported = stream_notes(&events);
        let mut slots = stream_slots(&reported);
        kept.sort();
        slots.sort();
        assert_eq!(
            kept, slots,
            "the two readers file the notes differently for {src:?}"
        );
        for (_, owns_line, owner) in &reported {
            assert!(
                !owns_line || owner.is_leading(),
                "a note that had a line to itself leads: {src:?}"
            );
            assert!(
                !owner.is_inline() || !owns_line,
                "the inline slot never holds a note that had a line to itself: {src:?}"
            );
        }
    }
}

/// A note the source wrote on a line it did not own can still lead the node the tree files it on.
#[test]
fn a_note_that_shared_a_line_can_still_lead_the_next_element() {
    // TOML files a comment with the element that follows it, because a comment runs to the end of
    // its line. This one shared the line with the value before it - so the source gave it no line
    // of its own, and the tree still puts it above the next element. The two facts are separate,
    // which is the whole reason the flag was split.
    let src = "arr = [1, # shared\n  2]\n";
    let events = parse_stream_with_dialect(src, TomlDialect::V1_1).expect("valid TOML");
    assert_eq!(
        stream_notes(&events),
        [("shared".to_string(), false, NoteOwner::FollowingLeading)]
    );
    let tree = from_toml_with_options(src, TomlDialect::V1_1).expect("valid TOML");
    let mut kept = Vec::new();
    tree_notes(&tree, &mut kept);
    assert_eq!(kept, [("shared".to_string(), true)]);
}

#[test]
fn a_note_the_tree_has_no_slot_for_is_still_in_the_stream() {
    // The AST has no home for an own-line note that meets no key or header in a document that has
    // keys of its own - the writer emits no such text, so the hub discards it. The stream reports
    // what the document wrote, and the difference is the boundary, not a leak.
    let src = "a = 1\n# nobody's note\n";
    let tree = from_toml_with_options(src, TomlDialect::V1_1).expect("valid TOML");
    let mut kept = Vec::new();
    tree_notes(&tree, &mut kept);
    let events = parse_stream_with_dialect(src, TomlDialect::V1_1).expect("valid TOML");
    assert_eq!(
        kept,
        Vec::<(String, bool)>::new(),
        "the tree gained a slot for a loose note"
    );
    assert_eq!(
        stream_notes(&events),
        [(
            "nobody's note".to_string(),
            true,
            NoteOwner::PrecedingLeading
        )]
    );
    // A document that is *only* a note has no key to miss it, so both routes keep it.
    let only = "# just this\n";
    let tree = from_toml_with_options(only, TomlDialect::V1_1).expect("valid TOML");
    let mut kept = Vec::new();
    tree_notes(&tree, &mut kept);
    let events = parse_stream_with_dialect(only, TomlDialect::V1_1).expect("valid TOML");
    assert_eq!(kept, [("just this".to_string(), true)]);
    assert_eq!(
        stream_notes(&events),
        [("just this".to_string(), true, NoteOwner::PrecedingLeading)]
    );
}

#[test]
fn the_tag_field_carries_what_the_document_states() {
    // `tag` was YAML's field. TOML fills it for exactly the values whose type the hub would
    // otherwise lose, which is the answer step 2 of the event stream was meant to give.
    let src = "when = 2024-01-01\nstamp = 2024-01-01T10:00:00Z\ntime = 10:00:00\nbin = 0b1010\nhex = 0xFF\n";
    let events = parse_stream_with_dialect(src, TomlDialect::V1_1).expect("valid TOML");
    let shape = event_shape(&events);
    assert!(
        shape.contains(&"scalar[Plain]<!!timestamp>2024-01-01T10:00:00Z".to_string()),
        "{shape:?}"
    );
    assert!(
        shape.contains(&"scalar[Plain]<!!timestamp>2024-01-01".to_string()),
        "{shape:?}"
    );
    assert!(
        shape.contains(&"scalar[Plain]<!time>10:00:00".to_string()),
        "{shape:?}"
    );
    // YAML Core has no binary lexeme, so the spelling needs the tag to stay an integer; hex it can
    // read needs none.
    assert!(
        shape.contains(&"scalar[Plain]<!!int>0b1010".to_string()),
        "{shape:?}"
    );
    assert!(
        shape.contains(&"scalar[Plain]0xFF".to_string()),
        "{shape:?}"
    );
}

#[test]
fn one_scanner_decides_for_both_readers() {
    // There is no second grammar to fall out of agreement, and the claim is testable: whatever the
    // tree route refuses, the stream route refuses with the same diagnostic at the same place.
    let rejected: &[&str] = &[
        "",
        "a =",
        "a",
        "= 1",
        "[a",
        "[[a]",
        "[]",
        "a = 1\na = 2\n",
        "[a]\n[a]\n",
        "a.b = 1\n[a]\n",
        "a = 1\n[a.b]\n",
        "x = 1x",
        "a = 01",
        "a = 1__2",
        "a = _1",
        "a = 1_",
        "s = \"unterminated",
        "s = 'unterminated",
        "m = \"\"\"unterminated",
        "a = 2024-13-01",
        "a = 2024-01-01T10:00",
        "a = \x00",
        "a = 1\r\nb = 2\r",
        "arr = [1,",
        "tbl = { a = 1, }",
        "tbl = { a = 1, a.b = 2 }",
    ];
    for src in rejected {
        for dialect in [TomlDialect::V1_0, TomlDialect::V1_1] {
            let tree = from_toml_with_options(src, dialect);
            let stream = parse_stream_with_dialect(src, dialect);
            match (tree, stream) {
                (Err(first), Err(second)) => {
                    assert_eq!(first, second, "{src:?} [{dialect:?}] refused differently")
                }
                (Ok(_), Ok(_)) => {}
                (tree, stream) => panic!(
                    "{src:?} [{dialect:?}]: one reader accepted and the other refused ({:?} / {:?})",
                    tree.is_ok(),
                    stream.is_ok()
                ),
            }
        }
    }
}

#[test]
fn an_array_of_tables_opens_one_sequence_and_one_mapping_per_header() {
    let src = "[[p]]\nx = 1\n[[p]]\nx = 2\n";
    let events = parse_stream_with_dialect(src, TomlDialect::V1_1).expect("valid TOML");
    assert_eq!(
        event_shape(&events),
        [
            "map-open",
            "scalar[Plain]p",
            "seq-open",
            "map-open",
            "scalar[Plain]x",
            "scalar[Plain]1",
            "map-close",
            "map-open",
            "scalar[Plain]x",
            "scalar[Plain]2",
            "map-close",
            "seq-close",
            "map-close",
        ]
    );
    // Each element's mapping is the one its own header opened.
    // Each element's mapping is the one its own header opened: the root and the first element share
    // offset 0 because the document begins with that header, and the second element begins at the
    // `[[p]]` line that adds it.
    let opens: Vec<(usize, usize)> = events
        .iter()
        .filter_map(|event| match event.event_type {
            pyrs_toml::StreamEventType::MappingStart { .. } => Some((event.line, event.column)),
            _ => None,
        })
        .collect();
    assert_eq!(opens, [(0, 0), (0, 0), (2, 0)], "{opens:?}");
}

#[test]
fn the_seed_corpus_describes_the_same_document_twice() {
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/toml_roundtrip");
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("seed corpus missing at {}: {e}", dir.display()))
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "seed"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "the TOML seed corpus is empty");
    let mut compared = 0usize;
    for path in paths {
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let src = String::from_utf8_lossy(&bytes).into_owned();
        for dialect in [TomlDialect::V1_0, TomlDialect::V1_1] {
            let tree = from_toml_with_options(&src, dialect);
            let events = parse_stream_with_dialect(&src, dialect);
            match (tree, events) {
                (Ok(node), Ok(events)) => {
                    let mut from_tree = Vec::new();
                    tree_shape(&node, &mut from_tree);
                    assert_eq!(
                        from_tree,
                        event_shape(&events),
                        "{} [{dialect:?}]: the two readers describe different documents",
                        path.display()
                    );
                    let mut kept = Vec::new();
                    tree_notes(&node, &mut kept);
                    let reported = stream_notes(&events);
                    // The stream never loses a note and never invents one; it may report a note the
                    // AST has no slot for, which is the boundary named above.
                    let mut slots = stream_slots(&reported);
                    kept.sort();
                    slots.sort();
                    assert!(
                        kept.iter().all(|note| slots.contains(note)),
                        "{} [{dialect:?}]: the tree kept {kept:?}, the stream reported {reported:?}",
                        path.display()
                    );
                    compared += 1;
                }
                (Err(first), Err(second)) => assert_eq!(
                    first,
                    second,
                    "{} [{dialect:?}]: the two readers refused differently",
                    path.display()
                ),
                (tree, events) => panic!(
                    "{} [{dialect:?}]: one reader accepted and the other refused ({:?} / {:?})",
                    path.display(),
                    tree.is_ok(),
                    events.is_ok()
                ),
            }
        }
    }
    assert!(
        compared >= 10,
        "the corpus only compared {compared} documents"
    );
    // The unowned-note boundary is a real shape, and `crates/pyrs-toml/tests/event_stream.rs`
    // names it; this corpus is not required to carry one, so no floor is claimed for it.
}
