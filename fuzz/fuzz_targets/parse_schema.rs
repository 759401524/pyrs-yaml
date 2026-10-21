//! The schema language: an arbitrary schema definition fed to its own parser.
//!
//! Every other target feeds the engine a *document*; this one feeds the `rules:` grammar a schema, which is the
//! shape that reaches `parse_schema_yaml` from user data. A crash here is a schema a user can write that the
//! engine cannot refuse cleanly.
//!
//! Same contract as the other targets: a rejected schema is a pass, a panic or a stack overflow is a bug.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pyrs_yaml_core::parser::yaml::schema_language::parse_schema_yaml;

fuzz_target!(|data: &[u8]| {
    let Ok(schema) = std::str::from_utf8(data) else {
        return;
    };
    let _ = parse_schema_yaml(schema);
});
