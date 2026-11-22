//! Every committed seed must reach the claim its target asserts.
//!
//! Each target in `SKIP_ON_REFUSAL` opens its body with a typed refusal as a pass: `yaml_roundtrip`
//! and `edit_yaml` `return` when `parse` refuses, `json_roundtrip` and `parse_json` `continue` when
//! all three JSON dialects refuse, `toml_roundtrip` and `parse_toml` when both TOML gates refuse.
//! Every assertion in those six files sits behind that guard, so a seed whose document is refused
//! there replays clean under `-runs=0` with the writer promise it was filed for never asked of it,
//! while the corpus total still counts it and the matrix row still credits the target.
//!
//! A refusal is not the same thing as no protection, and this gate does not claim it is. What the
//! registry denies is the stronger credit — that the input keeps the writer or placement defect
//! from coming back. The ten lines it carried until 2026-11-17 were closed by re-mining nine inputs
//! into documents one gate reads and moving the tenth, the duplicate-key refusal fixture, to
//! `parse_yaml`, where a refused input is the question the target asks. The list is empty now, and
//! that emptiness is a measurement: the counterexample test below keeps `reaches` refusing the two
//! shapes those inputs pinned, and a target whose seed directory went missing is refused rather
//! than read as clean.
//!
//! This asks each target's own question of its own seeds - the same engine calls, including both TOML
//! gates, which no Python surface offers - and refuses when the set of unreachable seeds differs
//! from the set registered in `.ci/seed-reach.txt`. An unreachable seed that gets fixed drops out
//! of the measurement and reddens the same assertion as one that slipped in unnoticed, so the
//! registry cannot quietly rot in either direction.
//!
//! Regenerate with `SEED_REACH_UPDATE=1 cargo test -p pyrs-yaml-core --test seed_reach`.
//!
//! The registry is a plain-text list (`.ci/seed-reach.txt`), one `target seedname` line per
//! unreachable seed: this crate links no JSON parser, and adding one to measure a corpus property
//! would cost more than the measurement is worth.
//!
//! The set of targets read is checked against the corpus, not trusted: `fuzz/seeds` is the
//! authority, and `every_seed_corpus_is_walked_or_explained` refuses a directory that is neither in
//! `SKIP_ON_REFUSAL` nor in `EXPLAINED_ABSENT`. Measured 2026-11-18: nine seed directories, six
//! walked and three explained. Without that pair a tenth target would replay clean and reach no
//! assertion - the exact hole this file exists to close - while every number here stayed green,
//! because a hand-written list cannot notice a corpus it never heard of.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use pyrs_yaml_core::parser::parse;
use pyrs_yaml_core::parser::yaml::Schema;

/// The targets that skip their whole body on a typed refusal.
///
/// Why the three in `EXPLAINED_ABSENT` need no reach check lives there, next to the list that keeps
/// them from being silently forgotten. `parse_json` and `parse_toml` are here for the reason the
/// round-trip targets are: both loop over the dialects (or the two TOML gates) and `continue` when a
/// parse refuses, so a seed no dialect accepts runs no writer and asserts nothing.
///
/// `edit_yaml` is measured one precondition short of its harness on purpose. The target also needs a
/// NUL separator, at least one complete two-byte record, a non-empty path list, and one verb that
/// answers ok (`fuzz/edit_driver.rs:384-593`, where the target's loop body lives since the fuzz
/// target and this repository's replay share it); this predicate asks only whether the document
/// parses, because deciding the rest means duplicating the harness's own path walk and verb
/// dispatch, and a second copy of that logic drifts silently. The rest is not left to trust: the
/// replay (`edit_scripts.rs`) requires every seed to apply at least one edit, measured 2026-11-18
/// as a per-seed count (all 69 clear it; the minimum is the one accepted edit of each 1-record
/// seed), so a script whose every verb refuses reddens there instead of sitting in the corpus while
/// the edit tier never ran - that per-seed floor is the weaker form of the claim this predicate
/// cannot make. Before the replay existed the gap was measured instead: with the harness
/// instrumented to panic at each precondition and every seed replayed in its own process, 62 of 62
/// seeds reached an assertion (ledger (jm) carries that method).
const SKIP_ON_REFUSAL: [&str; 6] = [
    "yaml_roundtrip",
    "edit_yaml",
    "json_roundtrip",
    "toml_roundtrip",
    "parse_json",
    "parse_toml",
];

/// The seed corpora this gate does not walk, each for the reason its target needs no reach check.
///
/// Kept separate from the doc comment above so the derivation below can refuse a corpus that is in
/// neither list: `json_events_replay` asserts that both walks refuse with the same message on its
/// `(Err, Err)` arm, so a refusal is the claim it makes; `parse_yaml` compares the two doors'
/// messages on its refusal arm, so no seed replays it blind; `parse_schema` holds its result in
/// `let _ =`, which leaves "no panic on any input" as the assertion every seed does reach.
const EXPLAINED_ABSENT: [&str; 3] = ["json_events_replay", "parse_schema", "parse_yaml"];

fn reaches(target: &str, document: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(document) else {
        return false;
    };
    match target {
        "yaml_roundtrip" | "edit_yaml" => parse(text, Schema::Core).is_ok(),
        "json_roundtrip" | "parse_json" => {
            pyrs_json::from_json(text).is_ok()
                || pyrs_json::from_jsonc(text).is_ok()
                || pyrs_json::from_json5(text).is_ok()
        }
        "toml_roundtrip" | "parse_toml" => {
            pyrs_toml::from_toml(text).is_ok() || pyrs_toml::from_toml_v1_0(text).is_ok()
        }
        _ => true,
    }
}

/// The seeds of one target's directory, each paired with the bytes that target's own harness hands
/// to the engine call guarding its assertions - `None` when the harness returns before ever parsing.
fn seeds_of(target: &str, dir: &Path) -> Vec<(String, Option<Vec<u8>>)> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<(String, Option<Vec<u8>>)> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("seed") {
                return None;
            }
            let name = path.file_name()?.to_string_lossy().to_string();
            let raw = fs::read(&path).ok()?;
            // Only `edit_yaml` stores `document NUL script`: its harness parses up to that NUL and
            // returns at `fuzz/edit_driver.rs:386` when the separator is absent, so a seed without
            // one reaches no assertion however well its document parses. The other five targets are
            // handed the whole byte string, and splitting theirs would measure a shorter input than
            // the assertion ever sees.
            let document = if target == "edit_yaml" {
                raw.iter()
                    .position(|byte| *byte == 0)
                    .map(|split| raw[..split].to_vec())
            } else {
                Some(raw)
            };
            Some((name, document))
        })
        .collect();
    out.sort();
    out
}

fn registry_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.ci/seed-reach.txt")
}

fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds")
}

/// The unreachable seeds per target, and how many seeds the walk found in the six targets at all -
/// the second number is what keeps the first one honest, because a walk that found nothing would
/// otherwise report the same empty blind spot as a corpus that has none.
fn measure() -> (BTreeMap<String, Vec<String>>, usize) {
    let root = corpus_root();
    let mut blind: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut walked = 0usize;
    for target in SKIP_ON_REFUSAL {
        let seeds = seeds_of(target, &root.join(target));
        walked += seeds.len();
        for (name, document) in seeds {
            let reached = document.is_some_and(|bytes| reaches(target, &bytes));
            if !reached {
                blind.entry(target.to_string()).or_default().push(name);
            }
        }
    }
    (blind, walked)
}

/// The registry as `target seedname` lines, sorted, with the header comments the file starts with.
fn render(map: &BTreeMap<String, Vec<String>>) -> String {
    let mut out = String::new();
    for line in [
        "Seeds that reach NO assertion, measured by `crates/pyrs-yaml-core/tests/seed_reach.rs` against",
        "the engine call each of the six targets opens with, where a typed refusal is a pass. Replaying",
        "one of these inputs runs no writer and asks no round-trip promise. A line appears the moment an",
        "input stops reaching and drops when it is re-mined into a document that parses, or moved to a",
        "target that asks something of a refusal. An empty list is a measured state: the gate re-derives",
        "it from the corpus on every `cargo nextest`, keeps `reaches` refusing two known shapes, and",
        "refuses a target whose seed directory went missing. Regenerate:",
        "  SEED_REACH_UPDATE=1 cargo test -p pyrs-yaml-core --test seed_reach",
    ] {
        out.push_str(&format!("# {line}\n"));
    }
    let mut pairs: Vec<String> = Vec::new();
    for (target, names) in map {
        for name in names {
            pairs.push(format!("{target} {name}"));
        }
    }
    pairs.sort();
    for pair in pairs {
        out.push_str(&format!("{pair}\n"));
    }
    out
}

fn committed_pairs(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect();
    lines.sort();
    lines
}

/// The two directions the registry and the measurement can disagree in, as report lines.
fn drift_lines(expected: &[String], registered: &[String]) -> Vec<String> {
    let mut drift = Vec::new();
    for pair in expected {
        if !registered.contains(pair) {
            drift.push(format!("UNREGISTERED  {pair}"));
        }
    }
    for pair in registered {
        if !expected.contains(pair) {
            drift.push(format!(
                "STALE ENTRY   {pair} (the seed now reaches the assertion)"
            ));
        }
    }
    drift
}

#[test]
fn registered_unreachable_seeds_equal_the_measured_set() {
    let (measured, corpus) = measure();
    let path = registry_path();
    if std::env::var("SEED_REACH_UPDATE").is_ok_and(|value| value == "1") {
        let total: usize = measured.values().map(Vec::len).sum();
        fs::write(&path, render(&measured)).expect("the registry writes");
        println!("wrote {total} unreachable seeds to {}", path.display());
        return;
    }

    let text = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "{} cannot be read ({error}); run SEED_REACH_UPDATE=1 cargo test -p pyrs-yaml-core --test seed_reach",
            path.display()
        )
    });
    let registered = committed_pairs(&text);
    let expected: Vec<String> = render(&measured)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect();

    let drift = drift_lines(&expected, &registered);
    let total = expected.len();
    println!("unreachable seeds measured: {total} of {corpus} walked");
    for (target, names) in &measured {
        println!("  {target}: {}", names.len());
    }
    assert!(
        drift.is_empty(),
        "the unreachable-seed registry disagrees with the measurement:\n{}",
        drift.join("\n")
    );
    // The upper bound is the guard against a broken `reaches`: a target call that refuses more than
    // a quarter of its own archived inputs is not describing that target, and the registry would
    // then be a list of measurement artefacts rather than of seeds. The fraction is stated against
    // the walked total because the corpus grows every time an input is archived, and a hard-coded
    // corpus size would be a number nothing checks.
    assert!(
        total * 4 < corpus,
        "{total} unreachable of {corpus} walked seeds is not a believable measurement - the seed \
         walk found almost everything blind; check `reaches` against the target sources"
    );
    // An empty registry is only a measurement when the walk had a corpus; `total * 4 < corpus` is
    // satisfied by 0 of 0. A per-target floor is what keeps a deleted seed directory from reading
    // as a clean target - the assertion this replaced guarded the file ("the list must not be
    // empty"), which is a claim about the registry rather than about the walk behind it.
    for target in SKIP_ON_REFUSAL {
        assert!(
            !seeds_of(target, &corpus_root().join(target)).is_empty(),
            "{target} carries no committed seed, so its unreachable set was never measurable"
        );
    }
}

/// Every seed corpus is either walked here or explained, and every name in those lists still names
/// a target with seeds.
///
/// `SKIP_ON_REFUSAL` and `EXPLAINED_ABSENT` are hand-written while `fuzz/seeds` is the authority,
/// so a new target that arrives with a corpus would be walked by nothing and reported by nothing -
/// the same blindness as a stale `.ci/seed-reach.txt`, one level up. The opposite direction costs
/// the same to check and fails the same way in practice: a renamed target leaves a name behind that
/// matches no directory, and the list keeps reading as coverage of something that moved.
#[test]
fn every_seed_corpus_is_walked_or_explained() {
    let root = corpus_root();
    let mut corpora: Vec<String> = fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("{} must be readable: {error}", root.display()))
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    corpora.sort();
    assert!(
        !corpora.is_empty(),
        "no seed corpus found under {}, so this check measured nothing",
        root.display()
    );

    let unclassified: Vec<&str> = corpora
        .iter()
        .map(String::as_str)
        .filter(|name| !SKIP_ON_REFUSAL.contains(name) && !EXPLAINED_ABSENT.contains(name))
        .collect();
    assert!(
        unclassified.is_empty(),
        "seed corpora neither walked nor explained: {unclassified:?} - add each to SKIP_ON_REFUSAL \
         if its target skips on a refusal, or to EXPLAINED_ABSENT with the reason it needs no \
         reach check"
    );

    let named: Vec<&str> = SKIP_ON_REFUSAL
        .iter()
        .chain(EXPLAINED_ABSENT.iter())
        .copied()
        .collect();
    let missing: Vec<&str> = named
        .iter()
        .copied()
        .filter(|name| !corpora.iter().any(|corpus| corpus == name))
        .collect();
    assert!(
        missing.is_empty(),
        "targets named here with no seed corpus: {missing:?} - a name kept after its corpus moved \
         reads as coverage of nothing"
    );
    let targetless: Vec<&str> = corpora
        .iter()
        .map(String::as_str)
        .filter(|name| {
            !Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../fuzz/fuzz_targets")
                .join(format!("{name}.rs"))
                .is_file()
        })
        .collect();
    assert!(
        targetless.is_empty(),
        "seed corpora with no target source: {targetless:?} - their inputs replay nothing"
    );
    for name in SKIP_ON_REFUSAL {
        assert!(
            !EXPLAINED_ABSENT.contains(&name),
            "{name} is both walked and explained, and whichever list a reader believes decides the \
             answer"
        );
    }
}

/// `reaches` must still say no to a document its reader refuses.
///
/// The registry is empty, so an always-true `reaches` would leave the measurement agreeing with the
/// file while nothing reached anything. These are the two refusals the closed lines pinned - the
/// TOML depth guard and the duplicate-key refusal - asked directly instead of through a corpus
/// whose inputs were re-mined.
#[test]
fn reach_counterexamples_survive_the_remining() {
    let deep = format!("{} = 1\n", vec!["a"; 1100].join("."));
    assert!(
        !reaches("toml_roundtrip", deep.as_bytes()),
        "the TOML depth guard is gone"
    );
    assert!(
        !reaches("parse_toml", deep.as_bytes()),
        "the TOML depth guard is gone"
    );
    assert!(
        !reaches("yaml_roundtrip", b"? !? \"\"\n: 1\n? !? \"\"\n: 2\n"),
        "the duplicate-key refusal is gone"
    );
}

/// The renderer and the reader are one format: whatever `render` writes, `committed_pairs` must
/// give back. This is the pair that decides whether the registry can rot silently, so it is tested
/// against synthetic maps rather than against the committed file - the committed file is what the
/// assertion above compares, and an agreement between two wrong readers would show nothing.
#[test]
fn the_renderer_and_the_registry_reader_agree() {
    let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
    map.insert(
        "yaml_roundtrip".to_string(),
        vec![
            "former-crash-1.seed".to_string(),
            "shape-a.seed".to_string(),
        ],
    );
    map.insert("json_roundtrip".to_string(), vec!["only.seed".to_string()]);

    let written = render(&map);
    let read_back = committed_pairs(&written);
    let mut expected: Vec<String> = vec![
        "json_roundtrip only.seed".to_string(),
        "yaml_roundtrip former-crash-1.seed".to_string(),
        "yaml_roundtrip shape-a.seed".to_string(),
    ];
    expected.sort();
    assert_eq!(
        read_back, expected,
        "the registry text must parse back to its entries"
    );
    // The header comments are explanation, not entries: one line counted as a seed would move the
    // count the matrix quotes from the file.
    assert_eq!(
        written.lines().filter(|line| line.starts_with('#')).count(),
        8,
        "the header changed shape; the gate's own line filter reads it"
    );
}

#[test]
fn drift_reports_both_directions() {
    let measured = vec!["yaml_roundtrip new.seed".to_string()];
    let registered = vec!["yaml_roundtrip stale.seed".to_string()];
    let drift = drift_lines(&measured, &registered);
    assert_eq!(drift.len(), 2, "both halves must be visible: {drift:?}");
    assert!(
        drift[0].starts_with("UNREGISTERED") && drift[0].contains("new.seed"),
        "{drift:?}"
    );
    assert!(
        drift[1].starts_with("STALE ENTRY") && drift[1].contains("stale.seed"),
        "{drift:?}"
    );
    assert!(
        drift_lines(&measured, &measured).is_empty(),
        "agreement must be silent"
    );
}
