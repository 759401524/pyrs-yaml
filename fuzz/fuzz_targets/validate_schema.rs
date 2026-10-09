//! The schema language: an arbitrary schema validating an arbitrary document.
//!
//! Every other target feeds a *document*; nothing here has ever fed the engine a *schema*. That gap is how a
//! crash in the rule-path walk survived review (ledger (bj): `path: $.café` aborted the process, and every
//! schema test in the tree used ASCII keys, so no test could reach the code that walks a key). The input is
//! split on its first NUL, so one corpus entry carries both grammars and their interaction - a rule path is
//! only navigated when the document parses, and `validate_node` is what does the navigating.
//!
//! Same contract as the other targets: a rejected schema or document is a pass, a panic or a stack overflow is
//! a bug. `validate_node` recurses over the document, so the parser's depth guard has to hold here too.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pyrs_yaml_core::parser::parse;
use pyrs_yaml_core::parser::yaml::Schema;
use pyrs_yaml_core::parser::yaml::schema_language::{parse_schema_yaml, validate_node};

fuzz_target!(|data: &[u8]| {
    let (schema_bytes, document_bytes) = match data.iter().position(|byte| *byte == 0) {
        Some(split) => (&data[..split], &data[split + 1..]),
        None => (data, &[][..]),
    };
    let Ok(schema) = std::str::from_utf8(schema_bytes) else {
        return;
    };
    let Ok(document) = std::str::from_utf8(document_bytes) else {
        return;
    };
    let Ok(resolver) = parse_schema_yaml(schema) else {
        return;
    };
    let Ok(ast) = parse(document, Schema::Core) else {
        return;
    };
    // Either outcome is a pass; both mean every rule path was walked without aborting.
    let _ = validate_node(&ast, &resolver, document);
});
