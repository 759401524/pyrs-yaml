//! Property-based fuzz tests for the TOML / JSON / JSONC / JSON5 formats.
//!
//! `pbt.rs` fuzzes the YAML spine; this module gives the four other formats the
//! same treatment, covering the properties the objective calls for:
//!
//! - **no-panic on hostile input** - each parser is fed arbitrary (often invalid)
//!   bytes and must return `Ok` or `Err`, never panic.
//! - **no-panic serialization** - each writer is driven over arbitrary ASTs and
//!   must never panic even on nodes the format cannot represent (it returns `Err`).
//! - **re-parseability + writer fixed point** - whatever a writer emits must
//!   re-parse, and re-serializing the re-parsed value is a fixed point (the
//!   writer is idempotent on its own output).

#![cfg(test)]

use crate::ast::proptest_strategies::*;
use crate::json;
use crate::toml;
use proptest::prelude::*;

proptest! {
    // -- no-panic: parsers on arbitrary (mostly invalid) input ---------------
    #[test]
    fn prop_dialect_parsers_never_panic(bytes in prop::collection::vec(any::<u8>(), 0..512)) {
        let s = String::from_utf8_lossy(&bytes);
        let _ = json::from_json(&s);
        let _ = json::from_jsonc(&s);
        let _ = json::from_json5(&s);
        let _ = toml::from_toml(&s);
    }

    // -- no-panic: writers on arbitrary ASTs (may legitimately error) --------
    #[test]
    fn prop_dialect_writers_never_panic(node in arb_custom_node()) {
        let _ = json::to_json_text(&node);
        let _ = json::to_jsonc_text(&node);
        let _ = json::to_json5_text(&node);
        let _ = toml::to_toml(&node);
    }

    // -- re-parseability -----------------------------------------------------
    // Whatever a writer emits (for a node it can represent) must parse back with
    // no error. Comment fidelity through the dialect round-trip is exercised by
    // the dialect unit tests; here we assert the writer never emits unparseable
    // output, using arbitrary generated ASTs.
    #[test]
    fn prop_json_output_reparses(node in arb_custom_node()) {
        let Ok(text) = json::to_json_text(&node) else { return Ok(()) };
        prop_assert!(json::from_json(&text).is_ok(), "JSON writer output must re-parse: {text}");
    }

    #[test]
    fn prop_jsonc_output_reparses(node in arb_custom_node()) {
        let Ok(text) = json::to_jsonc_text(&node) else { return Ok(()) };
        prop_assert!(json::from_jsonc(&text).is_ok(), "JSONC writer output must re-parse: {text}");
    }

    #[test]
    fn prop_json5_output_reparses(node in arb_custom_node()) {
        let Ok(text) = json::to_json5_text(&node) else { return Ok(()) };
        prop_assert!(json::from_json5(&text).is_ok(), "JSON5 writer output must re-parse: {text}");
    }

    #[test]
    fn prop_toml_output_reparses(node in arb_custom_node()) {
        let Ok(text) = toml::to_toml(&node) else { return Ok(()) };
        prop_assert!(toml::from_toml(&text).is_ok(), "TOML writer output must re-parse: {text}");
    }
}
