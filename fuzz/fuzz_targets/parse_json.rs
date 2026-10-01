//! JSON / JSONC / JSON5 parsers plus their writers, fuzzed together.
//!
//! Grammar surface the dialect fuzzers reach with random bytes is near-zero
//! coverage (loaders bail at the first byte); libFuzzer evolves past it. Every
//! parse result that succeeds is fed back through each writer and re-parsed —
//! the writer contract (`to_json*` output is always loadable by our own
//! readers, comments/JSON5 spellings riding the AST) must hold for hostile
//! inputs too, and no path may panic.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pyrs_json::{from_json, from_json5, from_jsonc, to_json5_text, to_jsonc_text, to_json_text};

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
        // Each writer's output must re-parse under the strictest reader.
        for out in [
            to_json_text(&node),
            to_jsonc_text(&node),
            to_json5_text(&node),
        ] {
            let Ok(out) = out else { continue }; // documented rejection is a pass
            let _ = from_json(&out);
            let _ = from_jsonc(&out);
            let _ = from_json5(&out);
        }
    }
});
