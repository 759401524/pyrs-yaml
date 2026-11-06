//! Every committed seed must reach the claim its target asserts.
//!
//! Each round-trip target opens with a typed refusal as a pass: `yaml_roundtrip` and `edit_yaml`
//! `return` when `parse` refuses, `json_roundtrip` `continue`s when all three JSON dialects refuse,
//! `toml_roundtrip` when both TOML gates refuse. A seed whose document is refused there replays clean
//! under `-runs=0` while proving only that the refusal still happens - it cannot bring back the
//! placement or ownership defect it was archived for, yet the corpus total still counts it and the
//! matrix row still credits the target.
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

/// The targets that skip their whole body on a typed refusal. `json_events_replay` asserts on the
/// refusal pair itself and the `parse_*` targets exist to parse or refuse, so every seed there does
/// reach a claim.
const SKIP_ON_REFUSAL: [&str; 4] = [
    "yaml_roundtrip",
    "edit_yaml",
    "json_roundtrip",
    "toml_roundtrip",
];

fn reaches(target: &str, document: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(document) else {
        return false;
    };
    match target {
        "yaml_roundtrip" | "edit_yaml" => parse(text, Schema::Core).is_ok(),
        "json_roundtrip" => {
            pyrs_json::from_json(text).is_ok()
                || pyrs_json::from_jsonc(text).is_ok()
                || pyrs_json::from_json5(text).is_ok()
        }
        "toml_roundtrip" => {
            pyrs_toml::from_toml(text).is_ok() || pyrs_toml::from_toml_v1_0(text).is_ok()
        }
        _ => true,
    }
}

fn seeds_of(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<(String, Vec<u8>)> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("seed") {
                return None;
            }
            let name = path.file_name()?.to_string_lossy().to_string();
            let raw = fs::read(&path).ok()?;
            // An `edit_yaml` seed is `document NUL script`; the document is what the target parses.
            let document = raw.split(|byte| *byte == 0).next().unwrap_or(&[]).to_vec();
            Some((name, document))
        })
        .collect();
    out.sort();
    out
}

fn registry_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.ci/seed-reach.txt")
}

/// The unreachable seeds per target, and how many seeds the walk found in the four targets at all -
/// the second number is what keeps the first one honest, because a walk that found nothing would
/// otherwise report the same empty blind spot as a corpus that has none.
fn measure() -> (BTreeMap<String, Vec<String>>, usize) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds");
    let mut blind: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut walked = 0usize;
    for target in SKIP_ON_REFUSAL {
        let seeds = seeds_of(&root.join(target));
        walked += seeds.len();
        for (name, document) in seeds {
            if !reaches(target, &document) {
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
        "Seeds that reach NO assertion, measured by `crates/pyrs-yaml-core/tests/seed_reach.rs`",
        "against the engine call each round-trip target opens with. A typed refusal is a pass in",
        "those targets, so replaying one of these inputs proves the refusal still happens and",
        "nothing about the defect the seed was archived for. The list is a measured state: it",
        "shrinks when an input is re-mined into a document that parses, and a new line appears the",
        "moment one stops reaching. Regenerate with",
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
        7,
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
