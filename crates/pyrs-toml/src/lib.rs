//! TOML ⇄ `CustomNode` conversion — the hub's TOML spoke.
//!
//! Native TOML **1.1** grammar ([toml.io/en/v1.1.0]) implemented in
//! the `granit-parser` house style: byte-level scanner with exact
//! error positions, direct AST construction, no intermediate value
//! tree. The writer renders the shared AST back to TOML text with
//! stable rejection reasons. Round-trip EDITING (comments, styles,
//! splices) stays YAML-only by design: TOML is an exchange format
//! here, not an editing substrate.
//!
//! PR #116 upgraded the parser from TOML 1.0 to TOML 1.1. Four
//! additions land here:
//!
//! - **A1** Multi-line inline tables + trailing commas inside an
//!   inline table. Interior comments are supported only via a
//!   follow-up (see §Non-goals in the design doc).
//! - **A2** `\xHH` byte escape in basic strings (codepoints ≤ 0xFF).
//! - **A3** `\e` escape for U+001B (ESC).
//! - **A4** Optional seconds in local-time and date-time values
//!   (`14:15` and `2010-03-05 14:15` become valid).
//!
//! Strict TOML 1.0 consumers stay supported through
//! [`from_toml_v1_0`] and the [`TomlDialect::V1_0`] option; those
//! entry points reject every 1.1-only input with a positional parse
//! error. `from_toml` defaults to V1_1 (a superset — every 1.0 doc
//! parses identically).
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
//! [toml.io/en/v1.1.0]: https://toml.io/en/v1.1.0

pub(crate) mod parser;
mod writer;

pub use parser::{TomlDialect, from_toml, from_toml_v1_0, from_toml_with_options};
pub use writer::to_toml;

use pyrs_ast::ast::{Chomping, CustomNode, NodeMeta, ScalarStyle, Tag};
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
    /// `bool` records whether the source used a multi-line form (`"""…"""`
    /// or `'''…'''`). PR #132: it lets the projected `CustomNode` carry
    /// `ScalarStyle::Literal` so the multi-line shape survives the YAML hub
    /// and `to_toml` re-emits a `"""` block instead of an escaped single line.
    String(String, bool),
    Integer(i64, Option<Arc<str>>),
    Float(f64, Option<Arc<str>>),
    Boolean(bool),
    /// Full RFC 3339 text (offset/local date-time, local date, local time).
    /// `Some(kind)` for offset date-time (`Z` suffix or `±HH:MM` offset);
    /// `None` for local variants.
    Datetime(String, Option<&'static str>),
    Array(Vec<TomlValue>),
    InlineTable(Vec<(String, TomlValue, KVAnnotations)>),
}

/// A TOML table entry: a concrete value, an implicit sub-table (created by
/// a dotted key or a parent header path), an explicit table, or an
/// array-of-tables. Only `Explicit` and `ArrayOfTables` correspond to
/// headers in the source text; `Implicit` cannot be re-opened by `[a]` or
/// extended by dotted keys.
pub(crate) enum TomlTable {
    Value(TomlValue, KVAnnotations),
    Implicit(CowTable),
    Explicit(CowTable),
    ArrayOfTables(Vec<CowTable>),
}

/// Comments captured for a `key = value` pair during parsing.
///
/// The pair projects onto the shared AST as follows:
/// - `leading` becomes the key node's `NodeMeta::leading_comment` with
///   `standalone = true` (rendered on its own line before `key = value`).
/// - `trailing` becomes the value node's `NodeMeta::comment` with
///   `standalone = false` (rendered after the value on the same line).
/// - `blank_before` records a preceding blank line in the source so the
///   writer can reproduce the visual grouping.
///
/// Only the LAST contiguous standalone block immediately above a pair
/// survives; earlier blocks separated by blank lines are dropped, which
/// matches the YAML receiver's model and `toml_edit`'s decor handling.
#[derive(Default)]
pub(crate) struct KVAnnotations {
    pub(crate) leading: Option<String>,
    pub(crate) trailing: Option<String>,
    pub(crate) blank_before: bool,
}

/// Insertion-ordered TOML key/value store used during parsing.
///
/// `comment` holds the inline trailing text on the `[name] # ...` header
/// line; `leading` holds the last standalone `# ...` block on the line
/// immediately above the header; `blank_before` records whether a blank
/// line separated this section from the previous pair.
pub(crate) struct CowTable {
    pub(crate) entries: indexmap::IndexMap<String, TomlTable>,
    pub(crate) comment: Option<String>,
    pub(crate) leading: Option<String>,
    pub(crate) blank_before: bool,
}

impl CowTable {
    pub(crate) fn new() -> Self {
        Self {
            entries: indexmap::IndexMap::new(),
            comment: None,
            leading: None,
            blank_before: false,
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
        // Comments split across two `NodeMeta` slots (PR #114): the
        // standalone block goes to `leading_comment` (its own line
        // above), the inline trailing note stays on `comment` (same
        // line after the value). Sub-table and AOT branches push their
        // own leading onto the child node's `leading_comment` so both
        // slots can coexist on one node — that is the whole reason for
        // the AST change.
        let (standalone, inline, blank, value_node) = match v {
            TomlTable::Value(val, anns) => {
                let n = toml_value_to_node(val);
                (anns.leading, anns.trailing, anns.blank_before, n)
            }
            TomlTable::Implicit(ct) | TomlTable::Explicit(ct) => {
                let blank = ct.blank_before;
                // Detach `leading` before moving `ct` into the recursive
                // conversion, then reattach it onto the child node's own
                // `leading_comment` slot. That is the whole point of the
                // #114 AST change: a section header's leading note and
                // its inline trailing note coexist on the same node,
                // without ever displacing each other.
                let leading_text = ct.leading.clone();
                let mut node = cow_table_to_node(ct);
                if let Some(text) = leading_text {
                    node.set_leading_comment(pyrs_ast::ast::Comment {
                        text: std::sync::Arc::from(text),
                        standalone: true,
                    });
                }
                (None, None, blank, node)
            }
            TomlTable::ArrayOfTables(list) => {
                let items: Vec<CustomNode> = list
                    .into_iter()
                    .map(|sub| {
                        let blank = sub.blank_before;
                        let leading = sub.leading.clone();
                        let mut n = cow_table_to_node(sub);
                        if let Some(text) = leading {
                            n.set_leading_comment(pyrs_ast::ast::Comment {
                                text: std::sync::Arc::from(text),
                                standalone: true,
                            });
                        }
                        if blank {
                            n.set_blank_before(true);
                        }
                        n
                    })
                    .collect();
                let node = CustomNode::Sequence {
                    items,
                    flow_style: false,
                    meta: NodeMeta::default(),
                };
                (None, None, false, node)
            }
        };
        let mut key = CustomNode::Scalar {
            value: k.into(),
            style: ScalarStyle::Plain,
            chomping: Chomping::Clip,
            meta: NodeMeta::default(),
        };
        if let Some(text) = standalone {
            key.set_leading_comment(pyrs_ast::ast::Comment {
                text: std::sync::Arc::from(text),
                standalone: true,
            });
        }
        let mut val = value_node;
        if let Some(text) = inline {
            val.set_comment(pyrs_ast::ast::Comment {
                text: std::sync::Arc::from(text),
                standalone: false,
            });
        }
        // A blank-line hint rides on the value node so the writer sees
        // both hints (comment + blank) coming from the same slot chain.
        if blank {
            val.set_blank_before(true);
        }
        pairs.insert(key, val);
    }
    let mut meta = NodeMeta {
        comment: t.comment.map(into_comment),
        ..Default::default()
    };
    if t.blank_before {
        let decor = meta.decor.get_or_insert_with(Default::default);
        decor.blank_before = true;
    }
    CustomNode::Mapping {
        pairs,
        flow_style: false,
        meta,
    }
}

pub(crate) fn toml_value_to_node(v: TomlValue) -> CustomNode {
    match v {
        TomlValue::String(s, multiline) => {
            // A multi-line TOML string projects onto a YAML literal block so
            // the shape survives the hub. The block value keeps its trailing
            // newlines and chomping records how many (verified convention:
            // Strip = none, Clip = exactly one, Keep = two or more), so the
            // YAML writer/parser round-trips the value byte-for-byte.
            let (style, chomping) = if multiline {
                let trailing = s.len() - s.trim_end_matches('\n').len();
                let ch = match trailing {
                    0 => Chomping::Strip,
                    1 => Chomping::Clip,
                    _ => Chomping::Keep,
                };
                (ScalarStyle::Literal, ch)
            } else {
                (ScalarStyle::DoubleQuoted, Chomping::Clip)
            };
            CustomNode::Scalar {
                value: s.into(),
                style,
                chomping,
                meta: NodeMeta::default(),
            }
        }
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
            for (k, val, anns) in v {
                let mut key = CustomNode::Scalar {
                    value: k.into(),
                    style: ScalarStyle::Plain,
                    chomping: Chomping::Clip,
                    meta: NodeMeta::default(),
                };
                // PR #119: interior comments ride the same two-slot
                // convention as the surrounding tables — the own-line
                // note above a member onto its key's `leading_comment`,
                // the same-line trailing note onto its value's
                // `comment`.
                if let Some(text) = anns.leading {
                    key.set_leading_comment(pyrs_ast::ast::Comment {
                        text: std::sync::Arc::from(text),
                        standalone: true,
                    });
                }
                let mut value_node = toml_value_to_node(val);
                if let Some(text) = anns.trailing {
                    value_node.set_comment(pyrs_ast::ast::Comment {
                        text: std::sync::Arc::from(text),
                        standalone: false,
                    });
                }
                pairs.insert(key, value_node);
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

fn into_comment(text: String) -> pyrs_ast::ast::Comment {
    pyrs_ast::ast::Comment {
        text: std::sync::Arc::from(text),
        standalone: false,
    }
}
