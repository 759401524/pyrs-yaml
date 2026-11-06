//! Coverage-guided fuzzing of the YAML parser over arbitrary bytes.
//!
//! Two invariants, in the order the engine states them.
//!
//! 1. *No input may panic the parser.* Every path — accept, typed `ParseError`, depth/size guard —
//!    is a pass; a panic, abort, or OOM-scale leak is a bug. This is the half
//!    `scripts/fuzz_panics.py` samples at the Python layer but cannot guide.
//! 2. *The two entry points refuse the same way.* `load_ast` is shared precisely so the
//!    single-document door and the stream door cannot drift, and its comment says so. Before this
//!    change the target asserted nothing about that: both calls were `let _ =`, so a divergence in
//!    either the refusal message or which door refuses was invisible here. What is asked now: when
//!    the stream door refuses, the single door refuses with the identical message; when it accepts
//!    at most one document, the single door accepts; and when it accepts two or more, the single
//!    door answers `ParseError::SingleDocumentExpected` - the arity refusal the engine defines for
//!    that case, deliberately not a `Syntax` defect because every document in the input is
//!    well-formed.
//!
//! Deliberately not asked: tree equality between the two doors. Their nodes carry source spans, and
//! whether those compare equal across doors is a different claim with a different owner —
//! `tests/test_yaml_readers_parity.py` holds the Python-side version of it.
//!
//! Runs under `cargo fuzz` (nightly + libFuzzer):
//!
//! ```text
//! cargo fuzz run parse_yaml -- -max_total_time=60
//! ```

#![no_main]

use libfuzzer_sys::fuzz_target;
use pyrs_schema::types::Schema;
use pyrs_yaml_core::parser::{ParseError, parse, parse_all};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return; // non-UTF-8 is rejected at the boundary; not a parser input
    };
    // Both entry points: single document and the multi-document stream.
    let single = parse(text, Schema::Core);
    let stream = parse_all(text, Schema::Core);

    let documents = match &stream {
        Ok(documents) => documents.len(),
        Err(refused) => {
            let single_message = single
                .as_ref()
                .err()
                .map(ToString::to_string)
                .unwrap_or_else(|| format!("the single door accepted {text:?}"));
            assert_eq!(
                single_message,
                refused.to_string(),
                "the two doors refused {text:?} differently: single said {single_message}, stream \
                 said {refused}"
            );
            return;
        }
    };

    if documents > 1 {
        match single {
            Err(ParseError::SingleDocumentExpected { .. }) => {}
            other => panic!(
                "a {documents}-document stream must answer SingleDocumentExpected at the single \
                 door, got {other:?}: {text:?}"
            ),
        }
    } else {
        assert!(
            single.is_ok(),
            "the stream door read {documents} document(s) from {text:?} and the single door \
             refused it: {}",
            single
                .as_ref()
                .err()
                .map(ToString::to_string)
                .unwrap_or_default()
        );
    }
});
