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

#![no_std]

// `format!`/`vec!` live in `alloc`'s macro exports, which a `no_std` crate
// only sees through `#[macro_use]` — the std prelude that carries them for
// a normal crate is deliberately absent here.
#[macro_use]
extern crate alloc;
// `#![no_std]` drops `std` from the extern prelude, so the host test harness
// needs it named explicitly when the `std` feature is on.
#[cfg(feature = "std")]
extern crate std;

mod events;
pub(crate) mod parser;
mod writer;

pub use events::{parse_stream, parse_stream_with_dialect};
pub use parser::{TomlDialect, from_toml, from_toml_v1_0, from_toml_with_options};
// The event vocabulary lives in its own crate so a reader can name it without pulling an engine; the
// producer reports it, and re-exports it here the way the YAML and JSON engines do.
pub use pyrs_event::{StreamEvent, StreamEventType};
pub use writer::to_toml;

use alloc::sync::Arc;
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use pyrs_ast::ast::{Chomping, CustomNode, NodeMap, NodeMeta, ScalarStyle, Tag};

/// A TOML 1.0 value at the leaf of the grammar. Intermediate representation
/// used by the parser before projection onto `CustomNode`.
///
/// Numeric variants carry an optional `source` spelling so `from_toml ->
/// to_toml` is byte-stable for radix-prefixed integers (`0xDEADBEEF`,
/// `0o755`, `0b1101`) and exponent floats (`1e10`, `-3.14e-2`). A spelling
/// YAML Core cannot read back as the same type gets a standard tag on the
/// projected node: Core has no `0b` lexeme, so the untagged text would
/// re-resolve to a string, while `!!int` states the type the document gave —
/// and a standard tag decides the type in every format since (bu).
/// Underscore-separated digits and explicit `+` signs are deliberately NOT
/// preserved: the writer emits a source spelling only where TOML's own
/// grammar accepts it verbatim, and neither form qualifies (matches #107).
#[derive(Clone)]
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
    /// An element carries its own notes, as a member of an inline table does: `leading` is the
    /// comment lines above it, `trailing` the one that ended its line. An array without comments
    /// still uses the same shape with empty annotations.
    Array(Vec<(TomlValue, KVAnnotations)>),
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

/// One comment as the scanner saw it: its text, and the byte offset of the `#` that opened it.
///
/// The projection onto `CustomNode` never reads `at` - the AST has no span for a note - but the
/// event stream cannot report a note it cannot place, and recording the offset here costs the read
/// path one `usize` in a record the parser already builds per comment.
#[derive(Clone)]
pub(crate) struct Note {
    pub(crate) text: String,
    pub(crate) at: usize,
}

impl Note {
    /// The AST shape: a note has a text and a slot, not an offset.
    pub(crate) fn into_comment(self, standalone: bool) -> pyrs_ast::ast::Comment {
        pyrs_ast::ast::Comment {
            text: Arc::from(self.text),
            standalone,
        }
    }
}

/// Comments captured for a `key = value` pair during parsing.
///
/// The pair projects onto the shared AST as follows:
/// - `leading` becomes the key node's `NodeMeta::leading_comments`, in source
///   order (rendered on their own lines before `key = value`).
/// - `trailing` becomes the value node's `NodeMeta::comment` with
///   `standalone = false` (rendered after the value on the same line).
/// - `blank_before` records a preceding blank line in the source so the
///   writer can reproduce the visual grouping.
///
/// Every own-line note above the pair survives. This used to keep only the last
/// one — an `Option<String>` that each comment line overwrote, which lost text
/// silently while the output stayed perfectly round-trippable, and the same
/// overwrite has now been fixed on the YAML and JSONC ingest paths too.
#[derive(Default, Clone)]
pub(crate) struct KVAnnotations {
    pub(crate) leading: Vec<Note>,
    pub(crate) trailing: Option<Note>,
    pub(crate) blank_before: bool,
    /// Byte offset of the name this pair wrote. For an array element, which has no name, it is the
    /// element's own start and the event projection reads `value_at` instead.
    pub(crate) key_at: usize,
    /// Byte offset of the value's first byte.
    pub(crate) value_at: usize,
}

/// Insertion-ordered TOML key/value store used during parsing.
///
/// `comment` holds the inline trailing text on the `[name] # ...` header
/// line; `leading` holds every standalone `# ...` line stacked immediately above
/// the header, in source order; `blank_before` records whether a blank
/// line separated this section from the previous pair.
pub(crate) struct CowTable {
    pub(crate) entries: NodeMap<String, TomlTable>,
    pub(crate) comment: Option<Note>,
    pub(crate) leading: Vec<Note>,
    pub(crate) blank_before: bool,
    /// The offset of the construct that opened this table: the `[` / `[[` of a header, or the first
    /// byte of the key path when a dotted key created it. The event stream reports its
    /// `MappingStart` here, and the line it lands on is what lets a reader tell a dotted key from a
    /// header without the vocabulary carrying a TOML-only field (`docs/dev/event-stream.md`,
    /// decision 4).
    pub(crate) at: usize,
    /// The offset of the one key segment that names this table. A header line carries a whole path,
    /// so without this every level of `[a.b.c]` would report the same position and the stream's
    /// events would no longer arrive in source order.
    pub(crate) key_at: usize,
}

impl CowTable {
    /// A table opened by the construct at `at`, named by the segment at `key_at`.
    pub(crate) fn new(at: usize, key_at: usize) -> Self {
        Self {
            entries: NodeMap::default(),
            comment: None,
            leading: Vec::new(),
            blank_before: false,
            at,
            key_at,
        }
    }
}

impl Default for CowTable {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

pub(crate) fn cow_table_to_node(t: CowTable) -> CustomNode {
    let mut pairs = NodeMap::default();
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
            TomlTable::Implicit(mut ct) | TomlTable::Explicit(mut ct) => {
                let blank = ct.blank_before;
                // Detach `leading` before moving `ct` into the recursive
                // conversion, then reattach it onto the child node's own
                // `leading_comment` slot. That is the whole point of the
                // #114 AST change: a section header's leading note and
                // its inline trailing note coexist on the same node,
                // without ever displacing each other.
                let leading_text = core::mem::take(&mut ct.leading);
                let mut node = cow_table_to_node(ct);
                for note in leading_text {
                    node.push_leading_comment(note.into_comment(true));
                }
                (Vec::new(), None, blank, node)
            }
            TomlTable::ArrayOfTables(list) => {
                let items: Vec<CustomNode> = list
                    .into_iter()
                    .map(|mut sub| {
                        let blank = sub.blank_before;
                        let leading = core::mem::take(&mut sub.leading);
                        let mut n = cow_table_to_node(sub);
                        for note in leading {
                            n.push_leading_comment(note.into_comment(true));
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
                (Vec::new(), None, false, node)
            }
        };
        let key_style = toml_key_style(&k);
        let mut key = CustomNode::Scalar {
            value: k.into(),
            style: key_style,
            chomping: Chomping::Clip,
            block_indent: None,
            meta: NodeMeta::default(),
        };
        for note in standalone {
            key.push_leading_comment(note.into_comment(true));
        }
        let mut val = value_node;
        if let Some(note) = inline {
            val.set_comment(note.into_comment(false));
        }
        // A blank-line hint rides on the value node so the writer sees
        // both hints (comment + blank) coming from the same slot chain.
        if blank {
            val.set_blank_before(true);
        }
        pairs.insert(key, val);
    }
    let mut meta = match t.comment.map(|note| note.into_comment(false)) {
        Some(comment) => NodeMeta::with_comment(comment),
        None => NodeMeta::default(),
    };
    if t.blank_before {
        meta.decor_mut().blank_before = true;
    }
    CustomNode::Mapping {
        pairs,
        flow_style: false,
        meta,
    }
}

/// What a TOML scalar means to the hub: the text it stores, the style and chomping it stores them
/// in, and the tag the document's own type needs in order to be read back as the same type.
///
/// One function answers this for both readers of the IR - `toml_value_to_node` above builds the
/// node from it, `events` reports the same three fields as a `Scalar` event - so the projection a
/// stream describes and the tree a caller edits can never disagree about what `0b1010` or
/// `2024-01-01` means. It is the TOML shape of the same rule `pyrs-json` writes once as
/// `hub_scalar`.
#[derive(Debug, Clone)]
pub(crate) struct ScalarProjection {
    pub(crate) text: String,
    pub(crate) style: ScalarStyle,
    pub(crate) chomping: Chomping,
    pub(crate) tag: Option<Tag>,
}

/// Project one non-container TOML value. A container has no single answer: it is reported by its
/// own open and close events.
///
/// It takes the value by value, not by reference: the tree route already owns the value and moves
/// its text into the node, and copying that text per scalar is work the read path pays for every
/// document. The event producer only borrows the IR, so it clones there and pays for what it must
/// hand out as owned text anyway.
pub(crate) fn scalar_projection(value: TomlValue) -> ScalarProjection {
    match value {
        TomlValue::String(text, multiline) => {
            // A multi-line TOML string projects onto a YAML literal block so the shape survives the
            // hub. The block value keeps its trailing newlines and chomping records how many
            // (verified convention: Strip = none, Clip = exactly one, Keep = two or more), so the
            // YAML writer/parser round-trips the value byte-for-byte.
            let chomping = if multiline {
                let trailing = text.len() - text.trim_end_matches('\n').len();
                match trailing {
                    0 => Chomping::Strip,
                    1 => Chomping::Clip,
                    _ => Chomping::Keep,
                }
            } else {
                Chomping::Clip
            };
            ScalarProjection {
                style: if multiline {
                    ScalarStyle::Literal
                } else {
                    ScalarStyle::DoubleQuoted
                },
                text,
                chomping,
                tag: None,
            }
        }
        TomlValue::Integer(number, source) => match source {
            Some(src) => {
                let text: String = src.as_ref().into();
                // A preserved spelling only survives the hub if YAML Core reads it back as the
                // same type. `0b1010` does not - Core has no binary lexeme, so an untagged plain
                // scalar re-resolves to a string. The tag states the type instead, which is the
                // route the spoke declined while the bridges still read text alone; since (bu) the
                // loader, `to_yaml`, `to_toml` and the JSON writers all ask the tag first, so one
                // answer cannot drift from another.
                let tag = if matches!(
                    pyrs_schema::types::Schema::Core.resolve(&text),
                    pyrs_schema::types::YamlType::Int(_)
                ) {
                    None
                } else {
                    Some(Tag {
                        handle: "!!".to_string(),
                        suffix: "int".to_string(),
                    })
                };
                ScalarProjection {
                    text,
                    style: ScalarStyle::Plain,
                    chomping: Chomping::Clip,
                    tag,
                }
            }
            None => ScalarProjection {
                text: number.to_string(),
                style: ScalarStyle::Plain,
                chomping: Chomping::Clip,
                tag: None,
            },
        },
        TomlValue::Float(number, source) => ScalarProjection {
            // An exponent spelling (`1e10`) is kept because YAML Core reads it back as a float;
            // otherwise the hub's own float text is used, which is where `.inf` and `.nan` live.
            text: match source {
                Some(src) => src.as_ref().into(),
                None => fmt_yaml_float(number),
            },
            style: ScalarStyle::Plain,
            chomping: Chomping::Clip,
            tag: None,
        },
        TomlValue::Boolean(flag) => ScalarProjection {
            text: flag.to_string(),
            style: ScalarStyle::Plain,
            chomping: Chomping::Clip,
            tag: None,
        },
        TomlValue::Datetime(text, kind) => ScalarProjection {
            text,
            style: ScalarStyle::Plain,
            chomping: Chomping::Clip,
            // `date` and `time` keep their own tag because `datetime.fromisoformat` rejects a bare
            // `HH:MM:SS`; the tag is the load path's instruction, not a decoration.
            tag: Some(Tag {
                handle: "!".to_string(),
                suffix: kind.unwrap_or("timestamp").to_string(),
            }),
        },
        TomlValue::Array(_) | TomlValue::InlineTable(_) => {
            unreachable!("a container is described by its own events and its own nodes")
        }
    }
}

pub(crate) fn toml_value_to_node(v: TomlValue) -> CustomNode {
    match v {
        TomlValue::Array(v) => CustomNode::Sequence {
            items: v
                .into_iter()
                .map(|(val, anns)| {
                    // An element's notes ride the same two slots a pair's do: own-line notes on the
                    // element's leading list, the one that ended its line on its inline comment.
                    let mut node = toml_value_to_node(val);
                    for note in anns.leading {
                        node.push_leading_comment(note.into_comment(true));
                    }
                    if let Some(note) = anns.trailing {
                        node.set_comment(note.into_comment(false));
                    }
                    node
                })
                .collect(),
            flow_style: false,
            meta: NodeMeta::default(),
        },
        TomlValue::InlineTable(v) => {
            let mut pairs = NodeMap::default();
            for (k, val, anns) in v {
                let key_style = toml_key_style(&k);
                let mut key = CustomNode::Scalar {
                    value: k.into(),
                    style: key_style,
                    chomping: Chomping::Clip,
                    block_indent: None,
                    meta: NodeMeta::default(),
                };
                // PR #119: interior comments ride the same two-slot
                // convention as the surrounding tables - the own-line
                // notes above a member onto its key's leading list,
                // the same-line trailing note onto its value's
                // `comment`.
                for note in anns.leading {
                    key.push_leading_comment(note.into_comment(true));
                }
                let mut value_node = toml_value_to_node(val);
                if let Some(note) = anns.trailing {
                    value_node.set_comment(note.into_comment(false));
                }
                pairs.insert(key, value_node);
            }
            CustomNode::Mapping {
                pairs,
                flow_style: true,
                meta: NodeMeta::default(),
            }
        }
        other => {
            let ScalarProjection {
                text,
                style,
                chomping,
                tag,
            } = scalar_projection(other);
            CustomNode::Scalar {
                value: text.into(),
                style,
                chomping,
                // TOML multi-line strings use `\` escapes, not a YAML block indentation indicator,
                // so there is nothing to round-trip.
                block_indent: None,
                meta: match tag {
                    Some(tag) => NodeMeta::with_tag(tag),
                    None => NodeMeta::default(),
                },
            }
        }
    }
}

/// The style a TOML key must carry so that reading the equivalent YAML gives back the
/// same string.
///
/// A TOML key is a string by TOML's grammar, and the shared AST expresses "this scalar
/// is a string" the way YAML does - by quoting, which is what the string *values* above
/// already do. So `"1" = 2` must arrive as a quoted key, or `to_yaml` writes `1: 2` and
/// the next reader sees an integer; `"" = 3` would otherwise become a null key. Keys
/// that no YAML schema types (`host`, `port`) keep the plain style, so converting a
/// real config to YAML stays clean.
pub(crate) fn toml_key_style(key: &str) -> ScalarStyle {
    if pyrs_schema::schema::plain_text_is_typed(key) {
        ScalarStyle::DoubleQuoted
    } else {
        ScalarStyle::Plain
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
