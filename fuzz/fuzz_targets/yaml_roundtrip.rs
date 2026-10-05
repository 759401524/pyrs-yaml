//! parse -> serialize -> re-parse -> serialize over arbitrary YAML bytes.
//!
//! Round-trip *equality* is pinned by the proptest suite on generated ASTs;
//! what fuzz adds is the parser-side surface: serialized output of hostile
//! input must re-parse (the engine's own contract: `to_yaml` never emits
//! unparseable text) and never panic at any depth. A re-parse failure is the
//! SELF-REPARSE-FAIL class the dogfooding loop hunts — here reachable in
//! milliseconds per edge case instead of one corpus pass per run.
//!
//! A second invariant — note *survival* — is deliberately NOT asserted here: this
//! tier's oracle is text equality across rounds, and a document that is stable and
//! merely short a note passes it. That half now lives in
//! `crates/pyrs-yaml-core/tests/note_survival.rs`, which replays the committed seed
//! corpus and requires every note the reader recorded to appear in the emission. It
//! was kept out of this file on purpose: libFuzzer's corpus is nondeterministic, an
//! assertion that reddes on whatever the fuzzer explores next cannot gate a PR, and
//! the corpus test runs on every `cargo nextest` instead.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pyrs_schema::types::Schema;
use pyrs_yaml_core::parser::parse;
use pyrs_yaml_core::serializer::to_yaml;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let Ok(node) = parse(text, Schema::Core) else {
        return; // typed rejection is a pass
    };
    let once = to_yaml(&node);
    let reparsed = parse(&once, Schema::Core)
        .unwrap_or_else(|e| panic!("fmt output failed to re-parse: {e}\n---\n{once}\n---\n{text}"));
    let twice = to_yaml(&reparsed);
    assert_eq!(
        once, twice,
        "serialization not idempotent:\nfirst:\n{once}\nsecond:\n{twice}"
    );
});
