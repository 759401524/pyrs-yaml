pub mod comment;
pub mod merge;
pub mod registry;
pub mod scalar;
pub mod schema_language;

// The scalar-type vocabulary + resolution rules live in the `pyrs-schema`
// crate; re-exported here so `crate::parser::yaml::{types, schema}` paths and
// the flat re-exports below resolve unchanged across the engine.
pub use pyrs_schema::{schema, types};

pub use comment::{RawAnchor, YamlScan, compute_line_offsets, extract_anchors, scan_yaml};
pub use merge::resolve_merge_keys;
pub use registry::SchemaRegistry;
pub use scalar::{BlockHeader, detect_block_header, unescape_double_quoted};
pub use schema::resolve_yaml_type;
pub use schema_language::{RuleResolver, YamlTypeKind, parse_schema_yaml};
pub use types::{Schema, SchemaResolver, YamlSchema, YamlType};
