//! Every note the corpus tree keeps reaches the comment-capable writers.
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
//! twin, asserted with the same rule: every note on every node (and on every mapping key), its
//! `\n`/`\r` flattened to spaces and its ends trimmed, either appears in the emitted document or
//! appears with an inner `*/` split — the block spelling's one escape. A note whose flattened text
//! is empty is exempt, because every document contains the empty string: whether the writer kept
//! that note is not a question this oracle can answer either way.
//!
//! Both dialects are asked in both shapes (compact and pretty), because they emit notes through
//! different sites: compact writes block comments so a note cannot swallow the separator that
//! follows it, pretty writes `//` lines. A dialect that refuses a seed, or a writer that refuses a
//! tree, is skipped: the corpus deliberately holds bytes only some readers accept, and the
//! generative tier owns the refusal cases.
//!
//! Deleting the root-note call from either compact entry point reddens this file on
//! `shape-root-line-note.seed`, naming that dialect and shape. The seed was added with this file:
//! no other document in either corpus held the root's line-end slot, the one position no member
//! loop reaches.

use pyrs_ast::ast::CustomNode;
use pyrs_json::{
    JsonParseOptions, from_json_with_options, to_json5_text, to_json5_text_pretty, to_jsonc_text,
    to_jsonc_text_pretty,
};

/// Lower bounds on what the corpus must drive, measured when this file was written.
///
/// `MIN_PARSED_DOCUMENTS` counts seed-and-dialect trials a reader accepted — the documents the
/// writer claims run on. `MIN_NOTE_BEARING_DOCUMENTS` counts the accepted trees that actually
/// carry a note, so it is the floor under the comparison itself; without it, a corpus of note-free
/// seeds would drive every trial and prove nothing. Measured: 36 seeds over the two corpora x 2
/// comment-capable dialects is 72 trials, 56 accepted documents, 26 of them note-bearing. Both
/// floors sit below those numbers on purpose — slack for a seed being retired, not targets to hit.
const MIN_PARSED_DOCUMENTS: usize = 38;
const MIN_NOTE_BEARING_DOCUMENTS: usize = 15;

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

/// Every note the tree carries, in both slots: the one inline slot (which the lazy accessor
/// normalises across the two storage conventions) and the leading list. Keys are nodes like any
/// other, and the JSON writers emit notes on them, so a note on a key is part of the claim.
fn notes(node: &CustomNode, into: &mut Vec<String>) {
    if let Some(note) = node.comment().filter(|c| !c.standalone) {
        into.push(note.text.to_string());
    }
    for note in node.leading_comments() {
        into.push(note.text.to_string());
    }
    match node {
        CustomNode::Mapping { pairs, .. } => {
            for (key, value) in pairs.iter() {
                notes(key, into);
                notes(value, into);
            }
        }
        CustomNode::Sequence { items, .. } => {
            for item in items {
                notes(item, into);
            }
        }
        _ => {}
    }
}

/// The notes that never reached `emitted`, in the spelling the writers use.
fn lost_notes(node: &CustomNode, emitted: &str) -> Vec<String> {
    let mut carried = Vec::new();
    notes(node, &mut carried);
    carried
        .into_iter()
        .filter(|text| {
            let expected = text.trim().replace(['\n', '\r'], " ");
            !expected.is_empty()
                && !emitted.contains(&expected)
                && !emitted.contains(&expected.replace("*/", "* /"))
        })
        .collect()
}

#[test]
fn every_note_the_corpus_tree_keeps_reaches_a_comment_writer() {
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
    // Both committed JSON corpora: the replay corpus holds the note-slot shapes (a note after a
    // comma, around a separator, in every slot), the round-trip corpus the comment-only, block and
    // JSON5-spelling documents. Neither alone spells the shapes the other does.
    for (corpus, name, text) in ["json_events_replay", "json_roundtrip"]
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
            let mut carried = Vec::new();
            notes(&node, &mut carried);
            if !carried.is_empty() {
                noted += 1;
            }
            for (shape, write) in [("compact", compact), ("pretty", pretty)] {
                let Some(emitted) = write(&node) else {
                    continue;
                };
                let lost = lost_notes(&node, &emitted);
                assert!(
                    lost.is_empty(),
                    "{corpus}/{name} [{dialect}/{shape}]: {} note(s) never reached the document\n\
                     emitted:\n{emitted}\nlost:\n{}",
                    lost.len(),
                    lost.join("\n")
                );
            }
        }
    }
    assert!(
        parsed >= MIN_PARSED_DOCUMENTS,
        "the corpus only drove {parsed} accepted documents; expected at least {MIN_PARSED_DOCUMENTS}"
    );
    assert!(
        noted >= MIN_NOTE_BEARING_DOCUMENTS,
        "only {noted} accepted documents carried a note; expected at least \
         {MIN_NOTE_BEARING_DOCUMENTS}"
    );
}
