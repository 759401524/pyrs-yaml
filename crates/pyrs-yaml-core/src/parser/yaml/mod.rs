pub mod comment;
pub mod direct;
pub mod merge;
// Runtime schema registration is a std concern: the global registry is an
// `RwLock`, and the user-schema language needs `regex`. A `no_std` caller
// builds a `Schema` value directly instead of looking one up by name.
#[cfg(feature = "std")]
pub mod registry;
pub mod scalar;
#[cfg(feature = "std")]
pub mod schema_language;

// The scalar-type vocabulary + resolution rules live in the `pyrs-schema`
// crate; re-exported here so `crate::parser::yaml::{types, schema}` paths and
// the flat re-exports below resolve unchanged across the engine.
pub use pyrs_schema::{schema, types};

pub use comment::{
    YamlScan, anchor_name_before, anchor_name_within, compute_line_offsets, scan_yaml,
};
pub use direct::{DirectEvent, replicable_events};
pub use merge::resolve_merge_keys;
#[cfg(feature = "std")]
pub use registry::SchemaRegistry;
pub use scalar::{BlockHeader, detect_block_header, unescape_double_quoted};
pub use schema::resolve_yaml_type;
#[cfg(feature = "std")]
pub use schema_language::{RuleResolver, YamlTypeKind, parse_schema_yaml};
pub use types::{Schema, SchemaResolver, YamlSchema, YamlType};
