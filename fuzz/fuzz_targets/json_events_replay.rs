//! The two walks of a JSON document, fuzzed against each other.
//!
//! `pyrs_json::parse_stream` is a second walk over the scanners - a different traversal that
//! re-derives the same dispatch - so unlike TOML's producer it can genuinely disagree with the tree
//! route: about what is valid, about where a refusal is reported, about how a token reads. The
//! `json_roundtrip` target proves the writer settles; nothing proved the two readers describe the
//! same document until this target. Both invariants are asserted here:
//!
//! - one verdict at one place: either both walks refuse with the same `ParseError`, or both accept;
//! - what they carry: `from_events` of the stream is the same document as the tree the scanner
//!   built - compared through the strict writer, which ignores comment slots because the stream
//!   cannot report which node owns a same-line note - and the stream holds exactly the notes the
//!   tree kept, in the same multiset.
#![no_main]

use libfuzzer_sys::fuzz_target;
use pyrs_ast::ast::CustomNode;
use pyrs_json::{
    JsonParseOptions, from_events, from_json_with_options, parse_stream_with_options, to_json_text,
};

/// Every note text a tree carries.
fn tree_notes(node: &CustomNode, into: &mut Vec<String>) {
    if let Some(note) = node.comment() {
        into.push(note.text.to_string());
    }
    for note in node.leading_comments() {
        into.push(note.text.to_string());
    }
    match node {
        CustomNode::Mapping { pairs, .. } => {
            for (name, value) in pairs.iter() {
                tree_notes(name, into);
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

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    for (dialect, opts) in [
        ("json", JsonParseOptions::STRICT),
        ("jsonc", JsonParseOptions::JSONC),
        ("json5", JsonParseOptions::JSON5),
    ] {
        let tree = from_json_with_options(text, opts);
        let stream = parse_stream_with_options(text, opts);
        let (node, events) = match (tree, stream) {
            (Ok(node), Ok(events)) => (node, events),
            (Err(first), Err(second)) => {
                assert_eq!(
                    first.to_string(),
                    second.to_string(),
                    "{dialect}: the two walks refused {text:?} differently"
                );
                continue;
            }
            (tree, stream) => panic!(
                "{dialect}: one walk accepted {text:?} and the other refused ({:?} / {:?})",
                tree.is_ok(),
                stream.is_ok()
            ),
        };
        let replay = from_events(&events)
            .unwrap_or_else(|e| panic!("{dialect}: its own stream refused to replay {text:?}: {e}"));
        match (to_json_text(&node), to_json_text(&replay)) {
            (Ok(once), Ok(twice)) => assert_eq!(
                once, twice,
                "{dialect}: the stream replays a different document\nfirst:\n{once}\nsecond:\n{twice}\ninput:\n{text}"
            ),
            // A JSON5 non-finite value is a document the strict writer refuses for both trees, and
            // it refuses it for the same reason: no JSON spelling of `.inf` exists.
            (Err(_), Err(_)) => {}
            (direct, replayed) => panic!(
                "{dialect}: the two trees differ in being writable ({:?} / {:?}) for {text:?}",
                direct.is_ok(),
                replayed.is_ok()
            ),
        }
        let mut kept: Vec<String> = Vec::new();
        tree_notes(&node, &mut kept);
        let mut reported: Vec<String> = events
            .iter()
            .filter_map(|event| match &event.event_type {
                pyrs_json::StreamEventType::Comment { text, .. } => Some(text.to_string()),
                _ => None,
            })
            .collect();
        kept.sort();
        reported.sort();
        assert_eq!(
            kept, reported,
            "{dialect}: the stream and the tree hold different notes for {text:?}"
        );
    }
});
