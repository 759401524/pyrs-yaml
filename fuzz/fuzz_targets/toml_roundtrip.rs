//! parse -> write -> re-parse -> write over arbitrary TOML bytes.
//!
//! Same upgrade as `json_roundtrip` over its parse-only sibling: `parse_toml` feeds the
//! writer and re-reads the result, but discards it. The invariant worth asserting is that
//! the writer reaches a **fixed point** —
//!
//! ```text
//! to_toml(parse(to_toml(parse(x)))) == to_toml(parse(x))
//! ```
//!
//! which is the assertion this repository's comment-fidelity work lives or dies on. TOML
//! is where it bites hardest: notes are attached to keys, table headers and inline-table
//! members, and a writer that re-homes a note by one line produces text that re-reads
//! correctly while moving forever.
//!
//! Both grammar revisions are exercised on the input side and the 1.1 reader on the output
//! side, because `to_toml` emits 1.1 spelling (newline-`0x` byte strings and dotted-key
//! forms that the 1.0 reader rejects); requiring a 1.0 re-parse would red correct output.
//! A documented typed rejection is a pass, a panic is not.
#![no_main]

use libfuzzer_sys::fuzz_target;
use pyrs_toml::{from_toml, from_toml_v1_0, to_toml};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    for parsed in [from_toml(text).ok(), from_toml_v1_0(text).ok()] {
        let Some(node) = parsed else { continue };
        let Ok(once) = to_toml(&node) else {
            continue; // typed rejection is a pass
        };
        let reparsed = from_toml(&once)
            .unwrap_or_else(|e| panic!("TOML writer output failed to re-parse: {e}\n---\n{once}\n---\n{text}"));
        let twice = to_toml(&reparsed)
            .unwrap_or_else(|e| panic!("TOML writer rejected its own re-parsed tree: {e}\n---\n{once}"));
        assert_eq!(
            once, twice,
            "TOML serialization is not a fixed point:\nfirst:\n{once}\nsecond:\n{twice}"
        );
    }
});
