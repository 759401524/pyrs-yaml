//! The two walks of a JSON document, replayed deterministically over the seed corpus.
//!
//! `parse_stream` is a second traversal over the same scanners, so unlike TOML's producer —
//! which projects events out of the tree it already built and cannot disagree with itself about
//! validity — it can genuinely differ from the tree route about a verdict, a refusal position, or
//! how a token reads. `fuzz/fuzz_targets/json_events_replay.rs` asserts that they do not, on
//! whatever libFuzzer explores. A sampler cannot gate a merge, so the same invariant needs a
//! place that can fail a pull request: this is that place, the pairing
//! `crates/pyrs-json/tests/roundtrip_corpus.rs` already establishes.
//!
//! Two claims, per seed and per dialect:
//!
//! - **one verdict, one place**: either both walks accept, or both refuse with the same message
//!   (which carries the offset, so position is covered by the same assertion);
//! - **what they carry**: [`pyrs_json::from_events`] of the stream builds the document the scanner
//!   built, compared through the strict writer, and the stream reports exactly the notes the tree
//!   kept.
//!
//! The writer is the comparator on purpose: it forgets comment slots and re-derives everything
//! else, so the equality says the two walks resolved the *data* alike. Note text is then compared
//! as a multiset, which is the most the event vocabulary can promise — it has no way to say which
//! node owns a same-line note (`docs/dev/event-stream.md`, "The one thing the vocabulary still
//! cannot say"), and `standalone` is excluded because the two walks use that one boolean for two
//! questions: the tree's says which slot a note rides, the stream's says whether the source gave
//! it a line of its own. `the_two_walks_answer_different_questions_with_standalone` in
//! `src/parser.rs` pins that divergence, found by this gate.
//!
//! A dialect that refuses a seed skips it: the corpus deliberately holds bytes only some of the
//! readers accept.

use pyrs_ast::ast::CustomNode;
use pyrs_json::{
    JsonParseOptions, StreamEventType, from_events, from_json_with_options,
    parse_stream_with_options, to_json_text,
};

/// Lower bounds on what the corpus must drive, measured when this file was written.
///
/// `MIN_SEEDED_PAIRS` counts every seed-and-dialect pair that reached a verdict assertion —
/// a shared refusal is evidence too, it is the same walk agreeing on where the document stops
/// making sense. `MIN_REPLAYED_PAIRS` counts the pairs both walks *accepted*, so it is the floor
/// under the replay and note comparisons. One floor cannot cover both: a corpus of nothing but
/// refusals would drive pairs and replay none, and the second claim would go untested in silence.
///
/// Measured: 13 seeds over 3 dialects drove 39 pairs and replayed 22 accepted documents. Both
/// floors sit below those numbers on purpose — slack for a seed being retired, not targets to hit.
const MIN_SEEDED_PAIRS: usize = 30;
const MIN_REPLAYED_PAIRS: usize = 15;

fn seeds() -> Vec<(String, String)> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fuzz/seeds/json_events_replay");
    let mut entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("seed corpus missing at {}: {e}", dir.display()))
        .map(|entry| entry.expect("readable dir entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "seed"))
        .collect::<Vec<_>>();
    entries.sort();
    assert!(
        !entries.is_empty(),
        "no seeds in the JSON replay corpus at {}",
        dir.display()
    );
    entries
        .into_iter()
        .map(|path| {
            let name = path
                .file_name()
                .expect("file name")
                .to_string_lossy()
                .into();
            let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            (name, String::from_utf8_lossy(&bytes).into_owned())
        })
        .collect()
}

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

#[test]
fn the_two_walks_describe_the_same_document() {
    let mut driven = 0usize;
    let mut replayed = 0usize;
    for (name, text) in seeds() {
        for (dialect, opts) in [
            ("json", JsonParseOptions::STRICT),
            ("jsonc", JsonParseOptions::JSONC),
            ("json5", JsonParseOptions::JSON5),
        ] {
            let (tree, stream) = (
                from_json_with_options(&text, opts),
                parse_stream_with_options(&text, opts),
            );
            let (node, events) = match (tree, stream) {
                (Ok(node), Ok(events)) => (node, events),
                (Err(first), Err(second)) => {
                    assert_eq!(
                        first.to_string(),
                        second.to_string(),
                        "{name} [{dialect}]: the two walks refused differently"
                    );
                    driven += 1;
                    continue;
                }
                (tree, stream) => panic!(
                    "{name} [{dialect}]: one walk accepted and the other refused ({:?} / {:?})",
                    tree.is_ok(),
                    stream.is_ok()
                ),
            };
            driven += 1;
            replayed += 1;
            let replay = from_events(&events).unwrap_or_else(|e| {
                panic!("{name} [{dialect}]: its own stream refused to replay: {e}")
            });
            match (to_json_text(&node), to_json_text(&replay)) {
                (Ok(once), Ok(twice)) => assert_eq!(
                    once, twice,
                    "{name} [{dialect}]: the stream replays a different document\n\
                     first:\n{once}\nsecond:\n{twice}\ninput:\n{text}"
                ),
                // A JSON5 non-finite value is a document the strict writer refuses, and it refuses
                // the replayed tree for the same reason it refuses the scanned one.
                (Err(_), Err(_)) => {}
                (direct, replayed) => panic!(
                    "{name} [{dialect}]: the two trees differ in being writable ({:?} / {:?})",
                    direct.is_ok(),
                    replayed.is_ok()
                ),
            }
            let mut kept: Vec<String> = Vec::new();
            tree_notes(&node, &mut kept);
            let mut reported: Vec<String> = events
                .iter()
                .filter_map(|event| match &event.event_type {
                    StreamEventType::Comment { text, .. } => Some(text.to_string()),
                    _ => None,
                })
                .collect();
            kept.sort();
            reported.sort();
            assert_eq!(
                kept, reported,
                "{name} [{dialect}]: the stream and the tree hold different notes\n\
                 tree: {kept:?}\nstream: {reported:?}\ninput:\n{text}"
            );
        }
    }
    assert!(
        driven >= MIN_SEEDED_PAIRS,
        "the corpus only drove {driven} verdict pairs; expected at least {MIN_SEEDED_PAIRS}"
    );
    assert!(
        replayed >= MIN_REPLAYED_PAIRS,
        "the corpus only replayed {replayed} accepted documents; expected at least {MIN_REPLAYED_PAIRS}"
    );
}
