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

use pyrs_json::{from_json, from_json5, from_jsonc, to_json_text, to_json5_text, to_jsonc_text};

/// Lower bound on how many dialect rounds must reach the assertion.
///
/// Measured when this file was written: 14 seeds driving 33 rounds; the floor
/// is slack for a seed being retired, not a target to hit.
/// assertion fail in the run that found them - a registered defect, not an exemption. The floor sits
/// below that on purpose — it is slack for a seed being retired, not a target to hit — and
/// a count under it would mean the corpus stopped carrying input the writers can act on at
/// all. The silently-vacuous gate is what a floor exists to catch; the same reasoning is why
/// `crates/pyrs-yaml-core/tests/note_survival.rs` declares one.
const MIN_SEEDED_ROUNDS: usize = 25;

fn seeds() -> Vec<(String, String)> {
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/json_roundtrip");
    let mut entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("seed corpus missing at {}: {e}", dir.display()))
        .map(|entry| entry.expect("readable dir entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "seed"))
        .collect::<Vec<_>>();
    entries.sort();
    assert!(
        !entries.is_empty(),
        "no seeds in the JSON round-trip corpus at {}",
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
    ($dialect:expr, $name:expr, $text:expr, $parse:path, $write:path) => {{
        let (dialect, name, text) = ($dialect, $name, $text);
        match $parse(text) {
            Err(_) => 0,
            Ok(node) => match $write(&node) {
                Err(_) => 0,
                Ok(once) => {
                    let reparsed = $parse(&once).unwrap_or_else(|e| {
                        panic!("{name} [{dialect}]: writer output failed to re-parse: {e}\n---\n{once}\n---\n{text}")
                    });
                    let twice = $write(&reparsed).unwrap_or_else(|e| {
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
    for (name, text) in seeds() {
        // Reader and writer are paired per dialect and never crossed: `to_jsonc_text` emits
        // comments and `to_json5_text` emits hex numbers and bare keys, none of which the
        // strict reader may accept, so a cross-dialect demand would red correct output.
        rounds += round_trip!("json", &name, &text, from_json, to_json_text);
        rounds += round_trip!("jsonc", &name, &text, from_jsonc, to_jsonc_text);
        rounds += round_trip!("json5", &name, &text, from_json5, to_json5_text);
    }
    assert!(
        rounds >= MIN_SEEDED_ROUNDS,
        "the corpus only drove {rounds} writer rounds; expected at least {MIN_SEEDED_ROUNDS}"
    );
}
