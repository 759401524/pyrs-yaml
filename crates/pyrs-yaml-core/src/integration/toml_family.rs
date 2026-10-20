//! Engine-level cross-format tests for the TOML family. The `pyrs-toml`
//! crate owns the TOML grammar itself; scenarios that start from YAML source
//! (so they need this crate's parser) live here instead.

use crate::parser::{parse, yaml::Schema};
use pyrs_toml::{from_toml, to_toml};

#[test]
fn to_toml_roundtrip_table_and_array() {
    let yaml = "a: \"1\"\nb: 2\nc:\n  - x: 1\n  - x: 2\n";
    let ast = parse(yaml, Schema::Core).unwrap();
    let text = to_toml(&ast).unwrap();
    let back = from_toml(&text).unwrap();
    assert_eq!(to_toml(&back).unwrap(), text);
}

#[test]
fn a_yaml_float_spelling_toml_refuses_is_written_in_toml_grammar() {
    // libFuzzer edit_yaml crash-71feb9d9: Core YAML floats allow leading
    // zeros in the integer part (`083e6`), while TOML's `float-int-part` is
    // a `dec-int` and does not. The writer's pass-through gate only looked
    // for an `e`, so the hub emitted `lis6 = 083e6` and the TOML reader
    // refused its own writer's text.
    for spelling in ["083e6", "+083e6", ".5e1", "1.e5", "00.5"] {
        let yaml = format!("v: {spelling}\n");
        let ast = parse(&yaml, Schema::Core).expect("YAML accepts the spelling");
        let text = to_toml(&ast).expect("the value is representable in TOML");
        assert!(
            !text.contains('"'),
            "{spelling:?} stopped being a float: {text}"
        );
        from_toml(&text).unwrap_or_else(|e| {
            panic!("TOML refused the writer's own text for {spelling:?}: {e}\n{text}")
        });
        assert_eq!(
            to_toml(&from_toml(&text).unwrap()).unwrap(),
            text,
            "not a fixed point for {spelling:?}"
        );
    }
}

#[test]
fn a_fraction_spelling_is_rendered_from_the_value_so_the_export_settles() {
    // The TOML reader keeps a float's source text only for exponent forms,
    // so a passed-through fraction would return from the hub as its
    // canonical rendering and the first export would not be the fixed point.
    let ast = parse("v: 1.50\n", Schema::Core).unwrap();
    let text = to_toml(&ast).unwrap();
    assert_eq!(text, "v = 1.5\n");
    assert_eq!(to_toml(&from_toml(&text).unwrap()).unwrap(), text);
}
