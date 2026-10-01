//! Coverage-guided fuzzing of the YAML parser over arbitrary bytes.
//!
//! The invariant is the one `scripts/fuzz_panics.py` samples at the Python
//! layer but cannot guide: *no input may panic the parser*. Every path —
//! accept, typed `ParseError`, depth/size guard — is a pass; a panic, abort,
//! or OOM-scale leak is a bug. Runs under `cargo fuzz` (nightly + libFuzzer):
//!
//! ```text
//! cargo fuzz run parse_yaml -- -max_total_time=60
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;
use pyrs_schema::types::Schema;
use pyrs_yaml_core::parser::{parse, parse_all};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return; // non-UTF-8 is rejected at the boundary; not a parser input
    };
    // Both entry points: single document and the multi-document stream.
    let _ = parse(text, Schema::Core);
    let _ = parse_all(text, Schema::Core);
});
