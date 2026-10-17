//! JSON / JSONC / JSON5 parsers plus their writers, fuzzed together.
//!
//! Grammar surface the dialect fuzzers reach with random bytes is near-zero coverage (loaders bail
//! at the first byte); libFuzzer evolves past it. Every parse result that succeeds is fed back
//! through each writer, and the writer contract is asserted where it is a contract: **each writer's
//! output must load under a reader that can express what it wrote**. Panics are bugs everywhere.
//!
//! The pairing follows the grammar, not the wish. `to_json_text` is strict JSON, so all three
//! readers must take it. `to_jsonc_text` carries `//` notes that RFC 8259 has no place for, so it
//! is checked against the two readers that do (JSONC and JSON5 both carry comments) and only
//! probed against the strict one. `to_json5_text` may write `'x'`, a trailing comma, `0xAB`,
//! `Infinity` — spellings neither strict JSON nor JSONC accepts — so it is checked under JSON5
//! alone. A rejection by a reader that cannot express the spelling is a pass, and asserting the
//! opposite would test the dialect boundary rather than the writer.
//!
//! Before this, every re-read was `let _ =`, so the sentence "our own readers must load our own
//! writers' output" was documentation with no oracle behind it: a writer that emitted a byte no
//! reader of its own dialect could take would have passed. The value-preservation claim stays with
//! `json_roundtrip`, which re-writes the re-parsed node and compares the text; this target's job is
//! the loadability boundary across the three dialects.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pyrs_json::{from_json, from_json5, from_jsonc, to_json_text, to_json5_text, to_jsonc_text};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    for parsed in [
        from_json(text).ok(),
        from_jsonc(text).ok(),
        from_json5(text).ok(),
    ] {
        let Some(node) = parsed else { continue };
        if let Ok(out) = to_json_text(&node) {
            for result in [from_json(&out), from_jsonc(&out), from_json5(&out)] {
                result.unwrap_or_else(|e| {
                    panic!("to_json_text wrote strict JSON its own readers reject: {e}\n---\n{out}")
                });
            }
        }
        if let Ok(out) = to_jsonc_text(&node) {
            for result in [from_jsonc(&out), from_json5(&out)] {
                result.unwrap_or_else(|e| {
                    panic!(
                        "to_jsonc_text wrote a comment form its own readers reject: {e}\n---\n{out}"
                    )
                });
            }
            let _ = from_json(&out);
        }
        if let Ok(out) = to_json5_text(&node) {
            from_json5(&out).unwrap_or_else(|e| {
                panic!("to_json5_text wrote a JSON5 form the JSON5 reader rejects: {e}\n---\n{out}")
            });
            let _ = from_json(&out);
            let _ = from_jsonc(&out);
        }
    }
});
