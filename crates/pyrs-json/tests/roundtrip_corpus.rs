//! The JSON-family round-trip invariant, replayed deterministically over the seed corpus.
//!
//! `fuzz/fuzz_targets/json_roundtrip.rs` asserts this over whatever libFuzzer explores, on
//! the schedule. A sampler cannot gate a merge — `fuzz.yml` says so explicitly, and it is
//! why the YAML tier keeps its idempotence and note-survival assertions in a corpus test —
//! so the same invariant needs somewhere that can fail a pull request. This is that place:
//! `cargo nextest`, i.e. every PR.
//!
//! The oracle is a **fixed point per dialect**, not text preservation:
//!
//! ```text
//! to_json(parse(to_json(parse(x)))) == to_json(parse(x))
//! ```
//!
//! Text preservation is the wrong demand here on purpose. `to_json_text` normalises JSON5
//! spellings (`0x1f` → `31`, `.5` → `0.5`) and drops comments, because strict JSON has
//! neither, so the first round legitimately moves text. What must not happen is a *second*
//! move: that means the writer emitted text whose tree is not the tree it came from.
//!
//! Inputs a dialect rejects are skipped, not failed — a typed rejection is a pass, and the
//! corpus deliberately holds bytes only one of the three readers accepts.
//!
//! All three seed corpora are walked, in every writer shape the family publishes: the three
//! compact writers and the three pretty ones at `indent = 2`. `parse_json`'s own target asserts
//! nothing about a writer, and `json_events_replay` is not charged a fixed point anywhere else
//! either - its target and `events_replay.rs` compare the tree walk against the event replay of
//! the *same* input, so they write one tree twice but never write, re-parse and write again.
//! (This doc used to say `events_replay.rs` asserts the same `once == twice`; it does not.)
//! `corpus_notes.rs` charges note conservation over the replay seeds rather than settlement. A
//! writer regression can settle inside a corpus no test ever asks a writer about, which is the
//! shape the TOML twin closed for `parse_toml`. The pretty pairs are separate code paths - both
//! container arms of `write_value_inner` branch on `pretty` - so the compact walk cannot see a
//! drift that only moves a multi-line layout. Measured 2026-11-18: 80 `json_roundtrip` rounds,
//! 24 `parse_json` and 72 `json_events_replay`, 176 together, every one settling.

use pyrs_json::{
    from_json, from_json5, from_jsonc, to_json_text, to_json_text_pretty, to_json5_text,
    to_json5_text_pretty, to_jsonc_text, to_jsonc_text_pretty,
};

/// Per-corpus lower bounds on how many writer rounds must reach the assertion, measured
/// 2026-11-18 with all six writer shapes.
///
/// Each seed is an input that made an assertion fail in the run that found it - a registered
/// defect, not an exemption. A corpus that silently drops out of the walk reddens on its own row
/// instead of being absorbed by the other two.
const MIN_CORPUS_ROUNDS: [(&str, usize); 3] = [
    ("json_roundtrip", 74),
    ("parse_json", 22),
    ("json_events_replay", 66),
];

/// Lower bound on the total. The smallest per-shape contribution is 16 rounds - the strict
/// writers over the three corpora - so losing any one of the six shapes reddens here even when
/// every corpus row still clears its own floor.
const MIN_SEEDED_ROUNDS: usize = 166;

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
        "no seeds in the {corpus} corpus at {}",
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

/// One dialect's round trip: evaluates to 1 when the pair ran, 0 when the input is not valid
/// in this dialect or the writer documents a rejection for it.
///
/// A macro rather than a function so this file never names the shared AST type: the
/// invariant does not depend on which node type travels between reader and writer, and a
/// function signature here would break when that type moves between crates.
macro_rules! round_trip {
    ($dialect:expr, $name:expr, $text:expr, $parse:path, $write:expr) => {{
        let (dialect, name, text) = ($dialect, $name, $text);
        match $parse(text) {
            Err(_) => 0,
            Ok(node) => match ($write)(&node) {
                Err(_) => 0,
                Ok(once) => {
                    let reparsed = $parse(&once).unwrap_or_else(|e| {
                        panic!("{name} [{dialect}]: writer output failed to re-parse: {e}\n---\n{once}\n---\n{text}")
                    });
                    let twice = ($write)(&reparsed).unwrap_or_else(|e| {
                        panic!("{name} [{dialect}]: writer rejected its own re-parsed tree: {e}\n---\n{once}")
                    });
                    assert_eq!(
                        once, twice,
                        "{name} [{dialect}] is not a fixed point:\nfirst:\n{once}\nsecond:\n{twice}"
                    );
                    1
                }
            },
        }
    }};
}

#[test]
fn json_family_writers_reach_a_fixed_point_over_the_seed_corpus() {
    let mut rounds = 0usize;
    for (corpus, floor) in MIN_CORPUS_ROUNDS {
        let mut corpus_rounds = 0usize;
        for (name, text) in seeds(corpus) {
            let label = format!("{corpus}/{name}");
            // Reader and writer are paired per dialect and never crossed: `to_jsonc_text` emits
            // comments and `to_json5_text` emits hex numbers and bare keys, none of which the
            // strict reader may accept, so a cross-dialect demand would red correct output.
            corpus_rounds += round_trip!("json", &label, &text, from_json, to_json_text);
            corpus_rounds += round_trip!("json-pretty", &label, &text, from_json, |n| {
                to_json_text_pretty(n, 2)
            });
            corpus_rounds += round_trip!("jsonc", &label, &text, from_jsonc, to_jsonc_text);
            corpus_rounds += round_trip!("json5", &label, &text, from_json5, to_json5_text);
            corpus_rounds += round_trip!("jsonc-pretty", &label, &text, from_jsonc, |n| {
                to_jsonc_text_pretty(n, 2)
            });
            corpus_rounds += round_trip!("json5-pretty", &label, &text, from_json5, |n| {
                to_json5_text_pretty(n, 2)
            });
        }
        assert!(
            corpus_rounds >= floor,
            "the {corpus} corpus only drove {corpus_rounds} writer rounds; expected at least {floor}"
        );
        rounds += corpus_rounds;
    }
    assert!(
        rounds >= MIN_SEEDED_ROUNDS,
        "the corpus only drove {rounds} writer rounds; expected at least {MIN_SEEDED_ROUNDS}"
    );
}
