//! Every committed seed must reach the claim its target asserts.
//!
//! Each target in `SKIP_ON_REFUSAL` opens its body with a typed refusal as a pass: `yaml_roundtrip`
//! and `edit_yaml` `return` when `parse` refuses, `json_roundtrip` and `parse_json` `continue` when
//! all three JSON dialects refuse, `toml_roundtrip` and `parse_toml` when both TOML gates refuse.
//! Every assertion in those six files sits behind that guard, so a seed whose document is refused
//! there replays clean under `-runs=0` with the writer promise it was filed for never asked of it,
//! while the corpus total still counts it and the matrix row still credits the target.
//!
//! A refusal is not the same thing as no protection, and this gate does not claim it is: five of the
//! registered lines answer `max depth exceeded (max=1000)`, which is the TOML depth guard's own
//! postcondition. What the registry denies is the stronger credit — that the input keeps the writer
//! or placement defect from coming back — and the inputs stay filed because the refusal they pin is
//! a behaviour worth keeping.
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

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use pyrs_yaml_core::parser::parse;
use pyrs_yaml_core::parser::yaml::Schema;

/// The targets that skip their whole body on a typed refusal.
///
/// `json_events_replay` is absent on purpose: its `(Err, Err)` arm asserts that the two walks refuse
/// with the same message, so a refusal is the claim it makes. `parse_yaml` and `parse_schema` hold
/// their result in `let _ =`, which leaves "no panic on any input" as the assertion every seed does
/// reach. `parse_json` and `parse_toml` are not like those: both loop over the dialects (or the two
/// TOML gates) and `continue` when a parse refuses, so a seed no dialect accepts runs no writer and
/// asserts nothing - the same hole the round-trip targets have, which is why they are listed here.
const SKIP_ON_REFUSAL: [&str; 6] = [
    "yaml_roundtrip",
    "edit_yaml",
    "json_roundtrip",
    "toml_roundtrip",
    "parse_json",
    "parse_toml",
];

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
            // returns at `edit_yaml.rs:418` when the separator is absent, so a seed without one
            // reaches no assertion however well its document parses. The other five targets are
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

/// The unreachable seeds per target, and how many seeds the walk found in the six targets at all -
/// the second number is what keeps the first one honest, because a walk that found nothing would
/// otherwise report the same empty blind spot as a corpus that has none.
fn measure() -> (BTreeMap<String, Vec<String>>, usize) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds");
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
        "one of these inputs runs no writer and asks no round-trip promise. That is not no protection:",
        "five lines answer `max depth exceeded (max=1000)`, the TOML depth guard's own postcondition,",
        "and stay filed because the refusal they pin is worth keeping. What the list denies is the",
        "stronger credit - that the archived defect cannot come back. A line drops when an input is",
        "re-mined into a document that parses, and appears the moment one stops reaching. Regenerate:",
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
    assert!(
        !registered.is_empty(),
        "the registry lists no seed while the targets refuse some - read the measurement above"
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
