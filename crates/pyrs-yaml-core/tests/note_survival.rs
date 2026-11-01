//! Note survival over the committed fuzz corpus.
//!
//! The round-trip tier's oracle is idempotence — `to_yaml(parse(to_yaml(x))) ==
//! to_yaml(x)` — and a document that is stable and merely short a note passes it.
//! Losing a note is therefore invisible to the assertion that has driven every
//! comment fix in this engine, which is how five silent losses (stacked notes,
//! comment-only documents, a folded null key, a consumed merge key and the mapping
//! it consumed) survived behind green tests. This file asserts the other invariant:
//!
//! **every note the reader recorded for an input appears in the emitted document.**
//!
//! It walks the corpora by spoke, named because the set is closed and grows with each fixed crash
//! rather than with someone's imagination: the YAML spoke reads `fuzz/seeds/yaml_roundtrip`,
//! `fuzz/seeds/parse_yaml` and the document half of `fuzz/seeds/edit_yaml` (whose entries are
//! `document NUL script`), and the TOML spoke reads `fuzz/seeds/toml_roundtrip` and
//! `fuzz/seeds/parse_toml` through `from_toml` / `to_toml`. Measured on 2026-11-02 the two spokes
//! charge 60 + 2 + 1 and 11 + 3 note-bearing seeds. Inputs that are not valid UTF-8 or that the
//! parser (correctly) rejects are skipped: they have no AST, so there is no note to keep.
//!
//! **This file is the conservation half; the settlement half is elsewhere and wider.**
//! `crates/pyrs-toml/tests/roundtrip_corpus.rs` and its JSON twin replay the same corpora for the
//! fixed point on every `cargo nextest`. That is not the same claim: an emission that *drops* a note
//! settles immediately, so only charging the tree's notes against the text sees the loss — the
//! failure mode `(hw)` and `(hy)` name for TOML, where a lost line passed re-parseability, the fixed
//! point and tree equality all at once.
//!
//! **Not charged here, and why.** The three JSON corpora (`json_roundtrip` 16 seeds,
//! `json_events_replay` 15, `parse_json` 6) share one byte string across `from_json`, `from_jsonc`
//! and `from_json5`, and only the latter two record a note — but strict `to_json_text` has nowhere to
//! put it, so a conservation assertion written against that emission would fail by design. A JSONC
//! conservation half has to compare against `to_jsonc_text`, which is a different claim from the one
//! this file makes and is left unbuilt rather than quietly folded in. `parse_schema` holds one seed
//! and has no comment vocabulary at all.
//!
//! **What this gate cannot see, stated because a green check invites trust in it.**
//! The measure is the note the *reader recorded*, so a note lost during ingest is
//! invisible: the AST never had it, and nothing is then demanded of the output.
//! Proven by mutation, not by argument — making `attach_inline_comment` report
//! success without attaching anything (the defect that dropped `!x # note`, crash-
//! ce106ccc) reddens `a_note_beside_a_property_only_root_survives_and_settles` and
//! leaves this file **green**. An ingest-side gate would have to count comments in
//! the source text, which cannot be done reliably here: a `#` is a comment only
//! outside quotes, outside a block-scalar body and outside a tag token, and the
//! corpus deliberately contains `!###0 …` — a tag whose suffix is `#` characters.
//! So this file is the writer-side half of the invariant and the fuzz tier plus the
//! pinned per-shape tests are the reader-side half; neither is claimed to cover the
//! other.

use std::path::{Path, PathBuf};

use pyrs_toml::{from_toml, to_toml};
use pyrs_yaml_core::ast::CustomNode;
use pyrs_yaml_core::parser::parse;
use pyrs_yaml_core::parser::yaml::Schema;
use pyrs_yaml_core::serializer::to_yaml;

/// Every note text the reader attached anywhere in `node`, in source order.
///
/// Read through `comment()` / `leading_comments()` rather than the raw fields: a
/// note lives in one of two storage conventions (`decor.leading_comments` or a
/// legacy `comment { standalone: true }`) and the accessors normalise across both,
/// which is the whole reason the writers cannot miss a stack.
fn note_texts(node: &CustomNode, out: &mut Vec<String>) {
    if let Some(c) = node.comment() {
        out.push(c.text.to_string());
    }
    for c in node.leading_comments() {
        out.push(c.text.to_string());
    }
    match node {
        CustomNode::Mapping { pairs, .. } => {
            for (key, value) in pairs.iter() {
                note_texts(key, out);
                note_texts(value, out);
            }
        }
        CustomNode::Sequence { items, .. } => {
            for item in items {
                note_texts(item, out);
            }
        }
        CustomNode::Scalar { .. } | CustomNode::Null { .. } | CustomNode::Alias { .. } => {}
    }
}

fn seed_dirs() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds");
    ["yaml_roundtrip", "parse_yaml", "edit_yaml"]
        .iter()
        .map(|name| root.join(name))
        .filter(|dir| dir.is_dir())
        .collect()
}

/// The bytes of one seed that are a *document*.
///
/// `edit_yaml` entries are `document NUL script`: the fuzzer splits them the same way, so charging
/// the whole buffer would ask the YAML parser to read a script as content and reject the input,
/// which the loop above would then skip. Measured 2026-11-02: all 31 seeds hold a NUL, **none** of
/// the 31 parses whole, and cutting at the first NUL yields 31 documents — an unsplit walk would not
/// have failed, it would have found nothing, which is the worse outcome for a coverage gate.
fn document_bytes<'a>(dir: &Path, raw: &'a [u8]) -> &'a [u8] {
    if dir.file_name().is_some_and(|name| name == "edit_yaml") {
        let end = raw.iter().position(|b| *b == 0).unwrap_or(raw.len());
        &raw[..end]
    } else {
        raw
    }
}

#[test]
fn every_note_in_the_seed_corpus_survives_one_emission() {
    let mut checked = 0usize;
    let mut failures = Vec::new();

    for dir in seed_dirs() {
        let mut entries = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("{} must be readable: {e}", dir.display()))
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "seed"))
            .collect::<Vec<_>>();
        entries.sort();
        for path in entries {
            let raw = std::fs::read(&path).expect("seed must be readable");
            let Ok(src) = std::str::from_utf8(document_bytes(&dir, &raw)) else {
                continue; // rejected before any note exists
            };
            let Ok(node) = parse(src, Schema::Core) else {
                continue; // a typed rejection has no AST to lose notes from
            };
            let mut texts = Vec::new();
            note_texts(&node, &mut texts);
            if texts.is_empty() {
                continue;
            }
            checked += 1;
            let emitted = to_yaml(&node);
            let dropped: Vec<&String> = texts.iter().filter(|t| !emitted.contains(&***t)).collect();
            if !dropped.is_empty() {
                failures.push(format!(
                    "{}: dropped {:?} from {src:?}, emitted {emitted:?}",
                    path.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                    dropped,
                ));
                continue;
            }
            // The same input also has to settle in one emission, which is the
            // assertion the fuzz tier makes by replaying these files: checking it
            // here too turns the corpus into a deterministic gate that runs on every
            // `cargo nextest`, not only where libFuzzer happens to explore.
            let again =
                to_yaml(&parse(&emitted, Schema::Core).unwrap_or_else(|e| {
                    panic!("{}: emitted text must re-parse: {e}", path.display())
                }));
            if again != emitted {
                failures.push(format!(
                    "{}: not a one-round fixed point: {emitted:?} -> {again:?}",
                    path.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "note survival broken over the committed corpus:\n{}",
        failures.join("\n")
    );
    // The corpus must actually cover the invariant: a vacuous pass would be worse
    // than no test, because it looks like coverage. Measured on 2026-11-02, the three
    // YAML-spoke corpora carry notes on 60 + 2 + 1 seeds; 55 is that count with slack
    // for inputs being retired, and a reading below it means the corpus lost its
    // comments rather than that the assertion is merely unused.
    assert!(
        checked >= 55,
        "expected the seed corpus to carry notes for many inputs, got {checked} note-bearing seeds"
    );
}

/// The conservation half of the same invariant, on the TOML spoke.
///
/// `crates/pyrs-toml/tests/roundtrip_corpus.rs` already replays these files for the fixed point on
/// every `cargo nextest`, and that is the assertion the TOML fuzz target makes — but a writer that
/// *deletes* a note reaches its fixed point on the first round, so settlement cannot see the loss.
/// `(hy)` measured the shape: `# K` above a section that carries `# T` emitted `# T\n[a]\nx = 1\n`
/// with the key's line gone, and re-parseability, the fixed point and tree equality all passed on a
/// document that had lost something. The property tier catches that class over *generated* trees
/// (`prop_toml_writer_carries_every_note`), which is a draw, not a replay — this charges the corpus,
/// so every crash the fuzzer already surrendered is asked again on every run.
#[test]
fn every_note_in_the_toml_corpus_survives_one_emission() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fuzz/seeds");
    let mut checked = 0usize;
    let mut failures = Vec::new();
    for name in ["toml_roundtrip", "parse_toml"] {
        let dir = root.join(name);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut paths = entries
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|p| p.extension().is_some_and(|ext| ext == "seed"))
            .collect::<Vec<_>>();
        paths.sort();
        for path in paths {
            let Ok(raw) = std::fs::read(&path) else {
                continue;
            };
            let Ok(src) = std::str::from_utf8(&raw) else {
                continue;
            };
            let Ok(node) = from_toml(src) else {
                continue;
            };
            let mut texts = Vec::new();
            note_texts(&node, &mut texts);
            if texts.is_empty() {
                continue;
            }
            checked += 1;
            let Ok(emitted) = to_toml(&node) else {
                failures.push(format!(
                    "{}: the writer refused a tree its own reader produced",
                    path.display()
                ));
                continue;
            };
            let dropped: Vec<&String> = texts.iter().filter(|t| !emitted.contains(&***t)).collect();
            if !dropped.is_empty() {
                failures.push(format!(
                    "{}: dropped {dropped:?} from {src:?}, emitted {emitted:?}",
                    path.display()
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "TOML note survival broken over the committed corpus:\n{}",
        failures.join("\n")
    );
    // Measured on 2026-11-02: 11 + 3 note-bearing seeds across the two TOML corpora. The floor sits
    // below that so retiring a seed does not redden the tier, and well above zero so a corpus that
    // stopped carrying notes cannot pass.
    assert!(
        checked >= 12,
        "expected the TOML seed corpus to carry notes, got {checked} note-bearing seeds"
    );
}
