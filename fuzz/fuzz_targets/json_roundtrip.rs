//! parse -> write -> re-parse -> write over arbitrary JSON-family bytes.
//!
//! `parse_json` already feeds each writer and re-reads the output, but it discards the
//! re-parse result: it proves the reader accepts its own writer, not that the writer has
//! *settled*. The invariant the YAML tier asserts and the cross-library parity tests keep
//! finding broken is the stronger one —
//!
//! ```text
//! to_json(parse(to_json(parse(x)))) == to_json(parse(x))
//! ```
//!
//! — a document whose text still moves on the second round has not reached a fixed point,
//! and a stable-but-different second emission means the writer's output re-reads as a
//! different tree than the one it came from. Both are invisible to a target that throws
//! the re-parse away.
//!
//! The fixed point is checked **per dialect**, which is the only honest pairing:
//! `to_jsonc_text` emits comments and `to_json5_text` emits hex numbers and unquoted keys,
//! none of which the strict JSON reader may accept, so demanding cross-dialect re-parse
//! would red correct output. A documented typed rejection is a pass; a panic is not.
//!
//! Macro rather than a table of function pairs on purpose: an array of `(reader, writer)`
//! items does not typecheck, because each function is its own `fn` item type, and coercing
//! them to a shared pointer type would name the node and error types that `pyrs-json` keeps
//! behind its re-exports. A macro names nothing.
#![no_main]

use libfuzzer_sys::fuzz_target;
use pyrs_json::{from_json, from_json5, from_jsonc, to_json5_text, to_jsonc_text, to_json_text};

/// Assert that `$write`'s own output reads back, in the same dialect, to the same text.
macro_rules! settled {
    ($dialect:expr, $input:expr, $node:expr, $parse:path, $write:path) => {{
        if let Ok(once) = $write($node) {
            match $parse(&once) {
                Ok(reparsed) => match $write(&reparsed) {
                    Ok(twice) => assert_eq!(
                        once, twice,
                        "{} serialization is not a fixed point:\nfirst:\n{}\nsecond:\n{}\ninput:\n{}",
                        $dialect, once, twice, $input
                    ),
                    // Only reachable if the first emission was fine and the re-read tree
                    // is not writable - which is the same class of inconsistency.
                    Err(e) => panic!(
                        "{} writer rejected its own re-parsed tree: {}\n---\n{}",
                        $dialect, e, once
                    ),
                },
                Err(e) => panic!(
                    "{} writer output failed to re-parse: {}\n---\n{}\n---\n{}",
                    $dialect, e, once, $input
                ),
            }
        }
    }};
}

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
        settled!("json", text, &node, from_json, to_json_text);
        settled!("jsonc", text, &node, from_jsonc, to_jsonc_text);
        settled!("json5", text, &node, from_json5, to_json5_text);
    }
});
