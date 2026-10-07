//! The TOML round-trip invariant, replayed deterministically over the seed corpus.
//!
//! Twin of `crates/pyrs-json/tests/roundtrip_corpus.rs`, and the same reasoning: the fuzz
//! target `fuzz/fuzz_targets/toml_roundtrip.rs` explores on the schedule, and a sampler
//! cannot gate a merge. This runs on every `cargo nextest`.
//!
//! The oracle is the fixed point
//!
//! ```text
//! to_toml(parse(to_toml(parse(x)))) == to_toml(parse(x))
//! ```
//!
//! For TOML this is the load-bearing invariant of the whole comment-fidelity effort: notes
//! attach to keys, to table headers and to inline-table members, and a writer that re-homes
//! one by a single line still produces text that reads back as the same *data*. Text
//! equality across rounds is what catches that, and it is exactly the assertion the YAML
//! side has been fixing against for a month.
//!
//! Both grammar revisions are exercised on the input side; the 1.1 reader re-reads the
//! output, because `to_toml` emits 1.1 spelling that a 1.0 reader may reject.

use pyrs_toml::{from_toml, from_toml_v1_0, to_toml};

/// Lower bound on how many rounds must reach the assertion — see the JSON twin for why a
/// floor exists at all. Measured when this file was written: 16 seeds driving 30 rounds.
const MIN_SEEDED_ROUNDS: usize = 23;

fn seeds() -> Vec<(String, String)> {
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds/toml_roundtrip");
    let mut entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("seed corpus missing at {}: {e}", dir.display()))
        .map(|entry| entry.expect("readable dir entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "seed"))
        .collect::<Vec<_>>();
    entries.sort();
    assert!(
        !entries.is_empty(),
        "no seeds in the TOML round-trip corpus at {}",
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

#[test]
fn toml_writer_reaches_a_fixed_point_over_the_seed_corpus() {
    let mut rounds = 0usize;
    for (name, text) in seeds() {
        for dialect in ["1.1", "1.0"] {
            let parsed = match dialect {
                "1.1" => from_toml(&text),
                _ => from_toml_v1_0(&text),
            };
            let Ok(node) = parsed else { continue };
            let Ok(once) = to_toml(&node) else { continue };
            rounds += 1;
            let reparsed = from_toml(&once).unwrap_or_else(|e| {
                panic!("{name} [{dialect} input]: TOML writer output failed to re-parse: {e}\n---\n{once}\n---\n{text}")
            });
            let twice = to_toml(&reparsed).unwrap_or_else(|e| {
                panic!("{name} [{dialect} input]: writer rejected its own re-parsed tree: {e}\n---\n{once}")
            });
            assert_eq!(
                once, twice,
                "{name} [{dialect} input] is not a fixed point:\nfirst:\n{once}\nsecond:\n{twice}"
            );
        }
    }
    assert!(
        rounds >= MIN_SEEDED_ROUNDS,
        "the corpus only drove {rounds} writer rounds; expected at least {MIN_SEEDED_ROUNDS}"
    );
}
