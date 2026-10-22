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
//! else, so the equality says the two walks resolved the *data* alike. The notes are then compared
//! in full - text, the source fact and the owner - because the event vocabulary can now say which
//! node owns a same-line note (`docs/dev/event-stream.md`, "The note flag split"), and the replay
//! asserts the stronger claim as well: the folded tree equals the scanned tree, decorations
//! included, so every note lands in the slot the tree had it in.
//!
//! A dialect that refuses a seed skips it: the corpus deliberately holds bytes only some of the
//! readers accept.

use pyrs_ast::ast::CustomNode;
use pyrs_event::NoteOwner;
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

/// Every note a tree carries, as its text and the slot it rides.
///
/// The tree's `standalone` flag is that slot: `true` is the leading list, `false` the one inline
/// slot. The stream's owner names the same two slots, so the two are comparable.
fn tree_notes(node: &CustomNode, into: &mut Vec<(String, bool)>) {
    if let Some(note) = node.comment() {
        into.push((note.text.to_string(), note.standalone));
    }
    for note in node.leading_comments() {
        into.push((note.text.to_string(), note.standalone));
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

/// Every note a stream reports, as its text, the source fact and the owner.
fn stream_notes(events: &[pyrs_event::StreamEvent<'_>]) -> Vec<(String, bool, NoteOwner)> {
    events
        .iter()
        .filter_map(|event| match &event.event_type {
            StreamEventType::Comment {
                text,
                owns_line,
                owner,
            } => Some((text.to_string(), *owns_line, *owner)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_jsonc_document_rebuilds_itself_from_its_own_events() {
    // The claim the corpus loop makes in the general case, named on one document: a JSONC source
    // with a note on the key, one on the value, one after a pair and one inside a sequence folds
    // back into the tree the parser built - every note in the slot it came from - and the strict
    // writer writes the folded tree as the text the parser produced.
    let text = r#"{"a" /* on the key */: /* on the value */ 1, // after a pair
 "b": [/* inside a sequence */ 2]}"#;
    let node = from_json_with_options(text, JsonParseOptions::JSONC).expect("valid JSONC");
    let events = parse_stream_with_options(text, JsonParseOptions::JSONC).expect("streams");
    let replay = from_events(&events).expect("its own stream replays");
    assert_eq!(
        to_json_text(&node).expect("writable"),
        to_json_text(&replay).expect("writable"),
        "the folded tree writes different text"
    );
    // The stronger claim, decorations included: the note that rode the key is still on the key,
    // the one inside the sequence is still inside it.
    assert_eq!(replay, node, "the folded tree filed a note in another slot");
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
            let mut kept: Vec<(String, bool)> = Vec::new();
            tree_notes(&node, &mut kept);
            let reported = stream_notes(&events);
            // The full comment, not the text alone: the owner has to name the same slot the tree
            // filed the note in, and the source fact has to agree with that slot.
            let mut kept_slots = kept.clone();
            let mut reported_slots: Vec<(String, bool)> = reported
                .iter()
                .map(|(text, _, owner)| (text.clone(), owner.is_leading()))
                .collect();
            kept_slots.sort();
            reported_slots.sort();
            assert_eq!(
                kept_slots, reported_slots,
                "{name} [{dialect}]: the stream and the tree hold different notes\n\
                 tree: {kept_slots:?}\nstream: {reported_slots:?}\ninput:\n{text}"
            );
            for (note, owns_line, owner) in &reported {
                assert!(
                    !*owns_line || owner.is_leading(),
                    "{name} [{dialect}]: {note} had a line of its own but does not lead"
                );
                assert!(
                    !owner.is_inline() || !*owns_line,
                    "{name} [{dialect}]: {note} takes the inline slot but had a line of its own"
                );
            }
            // The stronger claim: the folded tree is the scanned tree, decorations included, so
            // every note lands in the slot the tree had it in.
            assert_eq!(
                replay, node,
                "{name} [{dialect}]: the folded tree is not the scanned tree\ninput:\n{text}"
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
