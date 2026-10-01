//! Engine-level cross-format tests for the TOML family. The `pyrs-toml`
//! crate owns the TOML grammar itself; scenarios that start from YAML source
//! (so they need this crate's parser) live here instead.

use crate::parser::{parse, yaml::Schema};
use crate::toml::{from_toml, to_toml};

#[test]
fn to_toml_roundtrip_table_and_array() {
    let yaml = "a: \"1\"\nb: 2\nc:\n  - x: 1\n  - x: 2\n";
    let ast = parse(yaml, Schema::Core).unwrap();
    let text = to_toml(&ast).unwrap();
    let back = from_toml(&text).unwrap();
    assert_eq!(to_toml(&back).unwrap(), text);
}
