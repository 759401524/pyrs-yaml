//! Every note the corpus tree keeps comes back on the node it left.
//!
//! Three tiers already touch this ground and none covers it. `events_replay.rs` charges the two
//! *walks* (tree and stream) against each other on the replay corpus, but compares them through
//! `to_json_text`, the strict writer, which forgets every comment slot on purpose: a note the tree
//! kept and the JSONC writer dropped is the same text under that comparator as a note neither walk
//! kept. `roundtrip_corpus.rs` re-reads the writers' output, which catches a note that moved or
//! multiplied but not one that never left the tree. And `fmt_pbt`'s generative properties ask the
//! question of `arb_custom_node`, which draws the shapes a generator knows, not the ones a source
//! text can spell — a note after a comma, inside a block comment, or on an object key reaches the
//! tree only through the reader.
//!
//! So this is the corpus half of `fmt_pbt::prop_jsonc_writer_carries_every_note` and its JSON5
//! twin, asked as the stronger question. The generative properties assert that each note's text
//! appears somewhere in the document (`json_missing_notes` looks it up with `contains`), which a
//! writer that re-homed the note onto another key passes: the text is there, under the wrong
//! member. That is the YAML family's re-homing defect class, and only this tier can ask the
//! placement question — it owns the source text *and* a reader for the emitted one. Every note is
//! keyed by the route the reader walks to it (a mapping key and its value share one segment, a
//! sequence item takes its index, a container key collapses to `?`), and the two multisets, tree
//! and re-read document, must match. Presence follows from placement, so it is not asserted twice.
//!
//! The text is flattened the way the writers spell it: `\n`/`\r` become spaces, the ends are
//! trimmed, and an inner `*/` is split to `* /` — the block spelling's one escape. A note whose
//! flattened text is empty is exempt, because every document contains the empty string.
//!
//! Both dialects are asked in both shapes (compact and pretty), because they emit notes through
//! different sites: compact writes block comments so a note cannot swallow the separator that
//! follows it, pretty writes `//` lines. A dialect that refuses a seed, or a writer that refuses a
//! tree, is skipped: the corpus deliberately holds bytes only some readers accept, and the
//! generative tier owns the refusal cases.
//!
//! A re-homed note is caught in either direction: deleting the root-note call from either compact
//! entry point reddens this file on `shape-root-line-note.seed` — the only document in the three
//! corpora that holds the root's line-end slot, the one position no member loop reaches — and a
//! note that leaves its key lands here as a `(path, text)` pair on one side and not the other.
//!
//! All three JSON corpora are walked, not only the two named above. `parse_json` was the one no
//! deterministic test read — its fuzz target asks only panic-freedom and refusal parity — and two
//! of its seeds (`jsonc-comments`, `former-crash-comment-feff`) carry notes, so walking it costs a
//! read and closes the gap rather than adding a claim. It is also what moves the floors: without it
//! the walk accepts 62 documents with 31 note-bearing, and the floors below sit above those.

use pyrs_ast::ast::CustomNode;
use pyrs_json::{
    JsonParseOptions, from_json_with_options, to_json5_text, to_json5_text_pretty, to_jsonc_text,
    to_jsonc_text_pretty,
};

/// Lower bounds on what the corpus must drive, measured when this file was written.
///
/// `MIN_PARSED_DOCUMENTS` counts seed-and-dialect trials a reader accepted — the documents the
/// walk claims to run on. `MIN_NOTE_BEARING_DOCUMENTS` counts the accepted trees that actually
/// carry a note, so it is the floor under the comparison itself; without it, a corpus of note-free
/// seeds would drive every trial and prove nothing. `MIN_PLACED_TRIALS` counts the document/shape
/// pairs the comparison actually ran on.
///
/// Measured 2026-11-18: 46 seeds over the three corpora x 2 comment-capable dialects is 92 trials,
/// 72 accepted documents, 34 of them note-bearing, 68 placed (34 x both shapes: 46 from the replay
/// corpus, 16 from the round-trip one, 6 from `parse_json`). Every floor sits between what the
/// whole corpus drives and what it drives with one corpus left out — 62 accepted, 31 noted, and
/// 22/52/62 placed for the replay, round-trip and parse corpus respectively — so dropping any one
/// of the three is a red run rather than a quietly smaller walk, while a single seed being retired
/// still passes.
const MIN_PARSED_DOCUMENTS: usize = 66;
const MIN_NOTE_BEARING_DOCUMENTS: usize = 32;
const MIN_PLACED_TRIALS: usize = 56;

fn seeds(corpus: &str) -> Vec<(String, String)> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fuzz/seeds")
        .join(corpus);
    let mut entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("seed corpus missing at {}: {e}", dir.display()))
        .map(|entry| entry.expect("readable dir entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "seed"))
        .collect::<Vec<_>>();
    entries.sort();
    assert!(
        !entries.is_empty(),
        "no seeds in the JSON note corpus at {}",
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

/// The route to a member, in the writers' own terms. A mapping key and its value share the
/// segment: the slot a note sat in is not part of the claim, only the member it belongs to.
fn key_segment(key: &CustomNode) -> String {
    match key {
        CustomNode::Scalar { value, .. } => value.to_string(),
        CustomNode::Null { .. } => "null".to_string(),
        _ => "?".to_string(),
    }
}

/// Every note the tree carries, keyed by the route to the node that owns it: the one inline slot
/// (which the lazy accessor normalises across the two storage conventions) and the leading list.
/// Keys are nodes like any other, and the JSON writers emit notes on them, so a note on a key is
/// part of the claim.
fn note_sites(node: &CustomNode, path: &str, into: &mut Vec<(String, String)>) {
    let mut own = Vec::new();
    if let Some(note) = node.comment().filter(|c| !c.standalone) {
        own.push(note.text.to_string());
    }
    for note in node.leading_comments() {
        own.push(note.text.to_string());
    }
    for text in own {
        let flat = text.trim().replace(['\n', '\r'], " ").replace("*/", "* /");
        if !flat.is_empty() {
            into.push((path.to_string(), flat));
        }
    }
    match node {
        CustomNode::Mapping { pairs, .. } => {
            for (key, value) in pairs.iter() {
                let child = format!("{path}/{}", key_segment(key));
                note_sites(key, &child, into);
                note_sites(value, &child, into);
            }
        }
        CustomNode::Sequence { items, .. } => {
            for (index, item) in items.iter().enumerate() {
                note_sites(item, &format!("{path}[{index}]"), into);
            }
        }
        _ => {}
    }
}

#[test]
fn every_corpus_note_comes_back_on_the_node_it_left() {
    type Writer = fn(&CustomNode) -> Option<String>;
    let dialects: [(&str, JsonParseOptions, Writer, Writer); 2] = [
        (
            "jsonc",
            JsonParseOptions::JSONC,
            |node| to_jsonc_text(node).ok(),
            |node| to_jsonc_text_pretty(node, 2).ok(),
        ),
        (
            "json5",
            JsonParseOptions::JSON5,
            |node| to_json5_text(node).ok(),
            |node| to_json5_text_pretty(node, 2).ok(),
        ),
    ];
    let mut parsed = 0usize;
    let mut noted = 0usize;
    let mut placed = 0usize;
    // All three committed JSON corpora: the replay corpus holds the note-slot shapes (a note after
    // a comma, around a separator, in every slot), the round-trip corpus the comment-only, block
    // and JSON5-spelling documents, and the parse corpus the two small accepted documents its
    // target replayed without ever asking a writer anything.
    for (corpus, name, text) in ["json_events_replay", "json_roundtrip", "parse_json"]
        .into_iter()
        .flat_map(|corpus| {
            seeds(corpus)
                .into_iter()
                .map(move |seed| (corpus, seed.0, seed.1))
        })
    {
        for (dialect, options, compact, pretty) in dialects {
            let Ok(node) = from_json_with_options(&text, options) else {
                continue;
            };
            parsed += 1;
            let mut original = Vec::new();
            note_sites(&node, "", &mut original);
            if !original.is_empty() {
                noted += 1;
            }
            for (shape, write) in [("compact", compact), ("pretty", pretty)] {
                let Some(emitted) = write(&node) else {
                    continue;
                };
                if original.is_empty() {
                    continue;
                }
                let reparsed = from_json_with_options(&emitted, options).unwrap_or_else(|e| {
                    panic!(
                        "{corpus}/{name} [{dialect}/{shape}]: writer output failed to re-parse: \
                         {e}\n---\n{emitted}\n---\n{text}"
                    )
                });
                let mut round = Vec::new();
                note_sites(&reparsed, "", &mut round);
                let mut expected = original.clone();
                expected.sort();
                round.sort();
                assert_eq!(
                    expected, round,
                    "{corpus}/{name} [{dialect}/{shape}]: a note left its node; the left side is \
                     the tree's (path, text), the right the re-read document's\nemitted:\n{emitted}"
                );
                placed += 1;
            }
        }
    }
    assert!(
        parsed >= MIN_PARSED_DOCUMENTS,
        "the corpus only drove {parsed} accepted documents; expected at least \
         {MIN_PARSED_DOCUMENTS}"
    );
    assert!(
        noted >= MIN_NOTE_BEARING_DOCUMENTS,
        "only {noted} accepted documents carried a note; expected at least \
         {MIN_NOTE_BEARING_DOCUMENTS}"
    );
    assert!(
        placed >= MIN_PLACED_TRIALS,
        "only {placed} note-bearing documents were compared after a re-read; expected at least \
         {MIN_PLACED_TRIALS}"
    );
}
