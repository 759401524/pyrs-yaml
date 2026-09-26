//! TOML ⇄ `CustomNode` conversion — the hub's TOML spoke.
//!
//! TOML documents convert to the same AST the YAML parser produces
//! (`from_toml`), and any AST made of TOML-representable values renders
//! back as TOML text (`to_toml`). Round-trip EDITING (comments, styles,
//! splices) stays YAML-only by design: TOML/INI are exchange formats
//! for pyrs-yaml, not editing substrates.
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

use crate::ast::{Chomping, CustomNode, NodeMeta, ScalarStyle, Tag};
use crate::error::{ParseError, SerializeError};
use crate::parser::yaml::{Schema, YamlType};
use std::str::FromStr;

/// Parse TOML text into the shared AST.
pub fn from_toml(src: &str) -> Result<CustomNode, ParseError> {
    let doc = src.parse::<toml_edit::DocumentMut>().map_err(|e| {
        let (line, col) = e.span().map_or((0, 0), |r| {
            let byte = r.start;
            let line = src[..byte.min(src.len())].matches('\n').count();
            let col = byte - src[..byte.min(src.len())].rfind('\n').map_or(0, |i| i + 1);
            (line, col)
        });
        ParseError::Syntax {
            message: format!("TOML parse error: {e}"),
            line,
            col,
        }
    })?;
    Ok(table_to_node(doc.as_table()))
}

fn table_to_node(table: &toml_edit::Table) -> CustomNode {
    let mut pairs = indexmap::IndexMap::new();
    for (key, item) in table.iter() {
        pairs.insert(
            CustomNode::plain_scalar(key.to_string()),
            item_to_node(item),
        );
    }
    CustomNode::Mapping {
        pairs,
        flow_style: false,
        meta: NodeMeta::default(),
    }
}

fn item_to_node(item: &toml_edit::Item) -> CustomNode {
    match item {
        toml_edit::Item::Table(t) => table_to_node(t),
        toml_edit::Item::Value(v) => value_to_node(v),
        // Implicit/None items are formatting artifacts of valid documents.
        _ => CustomNode::plain_null(),
    }
}

fn value_to_node(value: &toml_edit::Value) -> CustomNode {
    match value {
        // Strings must never be schema-resolved downstream ("true" stays a
        // string), which the DoubleQuoted style guarantees in to_dict.
        toml_edit::Value::String(s) => CustomNode::Scalar {
            value: s.value().to_string().into(),
            style: ScalarStyle::DoubleQuoted,
            chomping: Chomping::Clip,
            meta: NodeMeta::default(),
        },
        toml_edit::Value::Integer(i) => CustomNode::plain_scalar(i.value().to_string()),
        toml_edit::Value::Float(f) => CustomNode::plain_scalar(format_toml_float(*f.value())),
        toml_edit::Value::Boolean(b) => CustomNode::plain_scalar(b.value().to_string()),
        toml_edit::Value::Datetime(d) => {
            let text = d.value().to_string();
            match Schema::Core.resolve(&text) {
                YamlType::Null => CustomNode::plain_scalar(text),
                resolved => {
                    let _ = resolved;
                    CustomNode::Scalar {
                        value: text.into(),
                        style: ScalarStyle::Plain,
                        chomping: Chomping::Clip,
                        meta: NodeMeta {
                            tag: Some(Tag::primary("timestamp")),
                            ..Default::default()
                        },
                    }
                }
            }
        }
        toml_edit::Value::Array(a) => CustomNode::Sequence {
            items: a.iter().map(value_to_node).collect(),
            flow_style: false,
            meta: NodeMeta::default(),
        },
        toml_edit::Value::InlineTable(it) => {
            let mut pairs = indexmap::IndexMap::new();
            for (key, v) in it.iter() {
                pairs.insert(CustomNode::plain_scalar(key.to_string()), value_to_node(v));
            }
            CustomNode::Mapping {
                pairs,
                flow_style: true,
                meta: NodeMeta::default(),
            }
        }
    }
}

fn format_toml_float(f: f64) -> String {
    if f == f64::INFINITY {
        ".inf".to_string()
    } else if f == f64::NEG_INFINITY {
        "-.inf".to_string()
    } else if f.is_nan() {
        ".nan".to_string()
    } else {
        let s = format!("{f}");
        // YAML/TOML floats must carry a decimal point.
        if s.contains('.') || s.contains('e') || s.contains('E') {
            s
        } else {
            format!("{s}.0")
        }
    }
}

/// Render a value-only AST as TOML text. `SerializeError::Internal` carries
/// the rejection reason for shapes TOML cannot represent.
pub fn to_toml(node: &CustomNode) -> Result<String, SerializeError> {
    let CustomNode::Mapping { pairs, .. } = node else {
        return Err(SerializeError::Internal("toml-requires-table-root"));
    };
    let mut doc = toml_edit::DocumentMut::new();
    for (key, value) in pairs {
        let CustomNode::Scalar {
            value: key_text, ..
        } = key
        else {
            return Err(SerializeError::Internal("toml-keys-must-be-scalars"));
        };
        // toml_edit's display layer quotes keys that require it; the AST
        // key text is the logical key either way.
        let item = node_to_value(value)?;
        doc[key_text.as_ref()] = toml_edit::Item::Value(item);
    }
    Ok(doc.to_string())
}

fn node_to_value(node: &CustomNode) -> Result<toml_edit::Value, SerializeError> {
    Ok(match node {
        CustomNode::Scalar {
            value, style, meta, ..
        } => {
            if meta
                .tag
                .as_ref()
                .is_some_and(|t| t.to_string().contains("timestamp"))
                && let Ok(dt) = toml_edit::Datetime::from_str(value)
            {
                return Ok(toml_edit::Value::Datetime(toml_edit::Formatted::new(dt)));
            }
            match (style, Schema::Core.resolve(value)) {
                (ScalarStyle::Plain, YamlType::Null) => {
                    return Err(SerializeError::Internal("toml-cannot-represent-null"));
                }
                (ScalarStyle::Plain, YamlType::Bool(b)) => toml_edit::Value::from(b),
                (ScalarStyle::Plain, YamlType::Int(i)) => toml_edit::Value::from(i),
                (ScalarStyle::Plain, YamlType::Float(f)) => toml_edit::Value::from(f),
                // Everything else is text: quoted scalars verbatim, plain
                // strings resolved-to-Str.
                _ => toml_edit::Value::from(value.as_ref()),
            }
        }
        CustomNode::Sequence { items, .. } => {
            let mut arr = toml_edit::Array::new();
            for item in items {
                // Array::push takes a Value; Mapping items project to inline
                // tables, null has no TOML spelling and is rejected.
                match item {
                    CustomNode::Null { .. } => {
                        return Err(SerializeError::Internal("toml-cannot-represent-null"));
                    }
                    CustomNode::Mapping { .. } => arr.push(node_to_inline_table(item)?),
                    _ => arr.push(node_to_value(item)?),
                }
            }
            toml_edit::Value::Array(arr)
        }
        CustomNode::Mapping { .. } => toml_edit::Value::InlineTable(node_to_inline_table(node)?),
        CustomNode::Alias { .. } | CustomNode::Null { .. } => {
            return Err(SerializeError::Internal("toml-unsupported-node"));
        }
    })
}

/// Build an inline table; `InlineTable::insert` only accepts a `Value`, so
/// nested tables recurse through the inline projection already.
fn node_to_inline_table(node: &CustomNode) -> Result<toml_edit::InlineTable, SerializeError> {
    let CustomNode::Mapping { pairs, .. } = node else {
        return Err(SerializeError::Internal("toml-expected-table"));
    };
    let mut it = toml_edit::InlineTable::new();
    for (k, v) in pairs {
        let CustomNode::Scalar { value: kt, .. } = k else {
            return Err(SerializeError::Internal("toml-keys-must-be-scalars"));
        };
        it.insert(kt.as_ref(), node_to_value(v)?);
    }
    Ok(it)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    #[test]
    fn from_toml_basic_types() {
        let ast = from_toml(
            "title = \"config\"\ncount = 3\nratio = 0.5\non = true\nat = 1979-05-27T07:32:00Z\n[tbl]\nk = \"v\"\n",
        )
        .unwrap();
        let yaml = crate::serializer::to_yaml(&ast);
        // Strings keep quoting (no re-resolution); typed scalars are plain.
        assert_yaml_roundtrip(&yaml);
        let back = from_toml(&toml_of(&ast)).unwrap();
        assert_eq!(toml_of(&back), toml_of(&ast));
    }

    /// Value-projection: render an AST through the Core schema as a
    /// canonical TOML string (used only inside these tests for equality).
    fn toml_of(node: &CustomNode) -> String {
        to_toml(node).unwrap()
    }

    fn assert_yaml_roundtrip(yaml: &str) {
        let ast = parser::parse(yaml, Schema::Core).unwrap();
        assert_eq!(crate::serializer::to_yaml(&ast), yaml);
    }

    #[test]
    fn string_true_stays_string() {
        let ast = from_toml("s = \"true\"\nn = \"42\"\n").unwrap();
        let CustomNode::Mapping { pairs, .. } = &ast else {
            panic!()
        };
        for (_, v) in pairs {
            assert!(
                matches!(
                    v,
                    CustomNode::Scalar {
                        style: ScalarStyle::DoubleQuoted,
                        ..
                    }
                ),
                "toml strings must not re-resolve"
            );
        }
    }

    #[test]
    fn to_toml_roundtrip_table_and_array() {
        let yaml = "a: \"1\"\nb: 2\nc:\n  - x: 1\n  - x: 2\n";
        let ast = parser::parse(yaml, Schema::Core).unwrap();
        let text = to_toml(&ast).unwrap();
        let back = from_toml(&text).unwrap();
        assert_eq!(to_toml(&back).unwrap(), text);
    }

    #[test]
    fn to_toml_rejects_root_scalar_and_null() {
        assert!(to_toml(&CustomNode::plain_scalar("x")).is_err());
        let mut pairs = indexmap::IndexMap::new();
        pairs.insert(CustomNode::plain_scalar("k"), CustomNode::plain_null());
        let ast = CustomNode::plain_mapping(pairs);
        assert!(to_toml(&ast).is_err());
    }

    #[test]
    fn float_specials_map_both_ways() {
        let ast = from_toml("i = inf\nm = nan\nbig = 1.0e10\n").unwrap();
        let text = to_toml(&ast).unwrap();
        assert!(text.contains("inf"), "{text}");
        assert!(text.contains("nan"), "{text}");
    }
}
