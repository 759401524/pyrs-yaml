//! TOML ⇄ `CustomNode` conversion — the hub's TOML spoke.
//!
//! Native TOML 1.0 grammar ([toml.io/en/v1.0.0]) implemented in the
//! `granit-parser` house style: byte-level scanner with exact error
//! positions, direct AST construction, no intermediate value tree. The
//! writer renders the shared AST back to TOML text with stable rejection
//! reasons. Round-trip EDITING (comments, styles, splices) stays YAML-only
//! by design: TOML is an exchange format here, not an editing substrate.
//!
//! Type mapping is loss-aware, mirroring the YAML schema semantics:
//!
//! | TOML            | CustomNode                                        |
//! |-----------------|---------------------------------------------------|
//! | string          | Scalar style=DoubleQuoted (never re-resolved)     |
//! | integer/float   | plain scalar, YAML float specials `.inf`/`.nan`   |
//! | boolean         | plain `true`/`false`                              |
//! | datetime        | plain RFC 3339 + `!timestamp` tag                 |
//! | table           | Mapping (plain keys)                              |
//! | array           | Sequence; inline tables become nested Mappings    |
//!
//! `to_toml` rejects the shapes TOML cannot hold: a non-table root,
//! null values, aliases, non-scalar keys.
//!
//! [toml.io/en/v1.0.0]: https://toml.io/en/v1.0.0

pub(crate) mod parser;
mod writer;

pub use parser::from_toml;
pub use writer::to_toml;

use crate::ast::{Chomping, CustomNode, NodeMeta, ScalarStyle, Tag};
use std::sync::Arc;

/// A TOML 1.0 value at the leaf of the grammar. Intermediate representation
/// used by the parser before projection onto `CustomNode`.
///
/// Numeric variants carry an optional `source` spelling so `from_toml ->
/// to_toml` is byte-stable for radix-prefixed integers (`0xDEADBEEF`,
/// `0o755`, `0b1101`) and exponent floats (`1e10`, `-3.14e-2`). The
/// spellings we DO preserve are exactly those YAML Core schema also
/// parses to the same numeric type, so the projected `CustomNode` needs
/// no `!!int` / `!!float` tag to survive downstream `to_yaml`/`load_*`
/// round-trips. Underscore-separated digits and explicit `+` signs are
/// deliberately NOT preserved because YAML Core reads them as strings;
/// the parser canonicalizes them to plain decimal (matches #107).
pub(crate) enum TomlValue {
    String(String),
    Integer(i64, Option<Arc<str>>),
    Float(f64, Option<Arc<str>>),
    Boolean(bool),
    /// Full RFC 3339 text (offset/local date-time, local date, local time).
    /// `Some(kind)` for offset date-time (`Z` suffix or `±HH:MM` offset);
    /// `None` for local variants.
    Datetime(String, Option<&'static str>),
    Array(Vec<TomlValue>),
    InlineTable(Vec<(String, TomlValue)>),
}

/// A TOML table entry: a concrete value, an implicit sub-table (created by
/// a dotted key or a parent header path), an explicit table, or an
/// array-of-tables. Only `Explicit` and `ArrayOfTables` correspond to
/// headers in the source text; `Implicit` cannot be re-opened by `[a]` or
/// extended by dotted keys.
pub(crate) enum TomlTable {
    Value(TomlValue),
    Implicit(CowTable),
    Explicit(CowTable),
    ArrayOfTables(Vec<CowTable>),
}

/// Insertion-ordered TOML key/value store used during parsing.
pub(crate) struct CowTable {
    pub(crate) entries: indexmap::IndexMap<String, TomlTable>,
    pub(crate) comment: Option<String>,
}

impl CowTable {
    pub(crate) fn new() -> Self {
        Self {
            entries: indexmap::IndexMap::new(),
            comment: None,
        }
    }
}

impl Default for CowTable {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn cow_table_to_node(t: CowTable) -> CustomNode {
    let mut pairs = indexmap::IndexMap::new();
    for (k, v) in t.entries {
        let key = CustomNode::Scalar {
            value: k.into(),
            style: ScalarStyle::Plain,
            chomping: Chomping::Clip,
            meta: NodeMeta::default(),
        };
        pairs.insert(key, toml_table_to_node(v));
    }
    CustomNode::Mapping {
        pairs,
        flow_style: false,
        meta: NodeMeta {
            comment: t.comment.map(into_comment),
            ..Default::default()
        },
    }
}

pub(crate) fn toml_table_to_node(t: TomlTable) -> CustomNode {
    match t {
        TomlTable::Value(v) => toml_value_to_node(v),
        TomlTable::Implicit(ct) | TomlTable::Explicit(ct) => cow_table_to_node(ct),
        TomlTable::ArrayOfTables(v) => CustomNode::Sequence {
            items: v.into_iter().map(cow_table_to_node).collect(),
            flow_style: false,
            meta: NodeMeta::default(),
        },
    }
}

pub(crate) fn toml_value_to_node(v: TomlValue) -> CustomNode {
    match v {
        TomlValue::String(s) => CustomNode::Scalar {
            value: s.into(),
            style: ScalarStyle::DoubleQuoted,
            chomping: Chomping::Clip,
            meta: NodeMeta::default(),
        },
        TomlValue::Integer(i, source) => match source {
            Some(src) => CustomNode::plain_scalar(src),
            None => CustomNode::plain_scalar(i.to_string()),
        },
        TomlValue::Float(f, source) => match source {
            Some(src) => CustomNode::plain_scalar(src),
            None => CustomNode::plain_scalar(fmt_yaml_float(f)),
        },
        TomlValue::Boolean(b) => CustomNode::plain_scalar(b.to_string()),
        TomlValue::Datetime(s, kind) => CustomNode::Scalar {
            value: s.into(),
            style: ScalarStyle::Plain,
            chomping: Chomping::Clip,
            meta: NodeMeta {
                tag: Some(Tag {
                    handle: "!".to_string(),
                    suffix: kind.unwrap_or("timestamp").to_string(),
                }),
                ..Default::default()
            },
        },
        TomlValue::Array(v) => CustomNode::Sequence {
            items: v.into_iter().map(toml_value_to_node).collect(),
            flow_style: false,
            meta: NodeMeta::default(),
        },
        TomlValue::InlineTable(v) => {
            let mut pairs = indexmap::IndexMap::new();
            for (k, val) in v {
                let key = CustomNode::Scalar {
                    value: k.into(),
                    style: ScalarStyle::Plain,
                    chomping: Chomping::Clip,
                    meta: NodeMeta::default(),
                };
                pairs.insert(key, toml_value_to_node(val));
            }
            CustomNode::Mapping {
                pairs,
                flow_style: true,
                meta: NodeMeta::default(),
            }
        }
    }
}

pub(crate) fn fmt_yaml_float(f: f64) -> String {
    if f == f64::INFINITY {
        ".inf".to_string()
    } else if f == f64::NEG_INFINITY {
        "-.inf".to_string()
    } else if f.is_nan() {
        ".nan".to_string()
    } else {
        let s = format!("{f}");
        if s.contains('.') || s.contains('e') || s.contains('E') {
            s
        } else {
            format!("{s}.0")
        }
    }
}

fn into_comment(text: String) -> crate::ast::Comment {
    crate::ast::Comment {
        text: std::sync::Arc::from(text),
        standalone: false,
    }
}
