//! The instruction gate's reader fixtures, pinned to what the writers emit.
//!
//! `from_json_medium` and `from_toml_medium` measure the reading half of each cross-format
//! bridge from committed bytes instead of bytes rendered at run time, so that a change in how
//! the writers *spell* their output cannot silently change what the reader is measured on: it
//! has to come through this file, fail, and be re-derived on purpose.
//!
//! What this file does **not** explain is the movement that was found while building it.
//! Rendering the input in setup (so the writer's cost cancels in the subtraction) put
//! `to_json_medium` +1.52% and `to_toml_medium` +0.26% above `main` with no engine code
//! changed - past the gate's 0.5% line on scenarios the addition never enters. The first
//! explanation offered here was the extra call site; committing the bytes removed the call and
//! moved `to_json_medium` +1.489% anyway, so that explanation is refuted by the pair of
//! measurements and the mechanism is recorded as unresolved. What is settled is the property:
//! a harness edit is not a neutral act for the numbers the same harness already produces, so a
//! re-baseline after one separates method from code by being written down, not by being
//! ignored. `serialize_*` and `parse_*` in that same binary stayed inside 0.042%.
use pyrs_json::{from_json, to_json_text};
use pyrs_toml::{from_toml, to_toml};
use pyrs_yaml_core::bench_inputs::{MEDIUM_JSON, MEDIUM_TOML, MEDIUM_YAML};
use pyrs_yaml_core::parser::parse;
use pyrs_yaml_core::parser::yaml::Schema;

#[test]
fn the_json_reader_fixture_is_what_the_writer_emits() {
    let ast = parse(MEDIUM_YAML, Schema::Core).expect("the YAML fixture parses");
    let emitted = to_json_text(&ast).expect("json writes the hub AST");
    assert_eq!(
        emitted, MEDIUM_JSON,
        "MEDIUM_JSON no longer matches `to_json_text`"
    );
    from_json(MEDIUM_JSON).expect("the JSON reader fixture parses");
}

#[test]
fn the_toml_reader_fixture_is_what_the_writer_emits() {
    let ast = parse(MEDIUM_YAML, Schema::Core).expect("the YAML fixture parses");
    let emitted = to_toml(&ast).expect("toml writes the hub AST");
    assert_eq!(
        emitted, MEDIUM_TOML,
        "MEDIUM_TOML no longer matches `to_toml`"
    );
    from_toml(MEDIUM_TOML).expect("the TOML reader fixture parses");
}
