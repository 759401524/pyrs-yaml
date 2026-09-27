//! JSON ⇄ `CustomNode` projection.
//!
//! Direction to-JSON mirrors the Python load pipeline: plain scalars
//! resolve through the core schema, quoted scalars are text (YAML 1.2),
//! so a plain key or value never changes type between the two views.
//! Direction from-JSON mirrors the bindings `from_json` helper.

use pyrs_yaml_core::ast::{CustomNode, NodeMeta, ScalarStyle};
use pyrs_yaml_core::parser::yaml::schema::needs_quotes;
use pyrs_yaml_core::parser::yaml::{Schema, YamlType};

/// AST -> JSON value using the core resolution pipeline.
///
/// Keys arrive as raw text (never resolved, mirroring `to_dict`); values
/// resolve through the core schema when plain, and are text when quoted
/// (YAML 1.2 rules). serde_json's `preserve_order` feature keeps the
/// document's key order in the output.
pub fn node_to_json(node: &CustomNode) -> Result<serde_json::Value, String> {
    Ok(match node {
        CustomNode::Scalar { value, style, .. } => {
            if matches!(style, ScalarStyle::Plain) {
                match Schema::Core.resolve(value) {
                    YamlType::Null => serde_json::Value::Null,
                    YamlType::Bool(b) => serde_json::Value::Bool(b),
                    YamlType::Int(i) => serde_json::json!(i),
                    YamlType::Float(f) if f.is_finite() => serde_json::json!(f),
                    // JSON has no spelling for inf/nan: surface the scalar
                    // text rather than silently corrupting it.
                    YamlType::Float(_) => serde_json::Value::String(value.to_string()),
                    YamlType::Str(s) => serde_json::Value::String(s.into_owned()),
                }
            } else {
                serde_json::Value::String(value.to_string())
            }
        }
        CustomNode::Null { .. } => serde_json::Value::Null,
        CustomNode::Sequence { items, .. } => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(node_to_json(item)?);
            }
            serde_json::Value::Array(out)
        }
        CustomNode::Mapping { pairs, .. } => {
            let mut obj = serde_json::Map::new();
            for (k, v) in pairs {
                obj.insert(key_text(k)?, node_to_json(v)?);
            }
            serde_json::Value::Object(obj)
        }
        CustomNode::Alias { name } => return Err(format!("unresolved alias *{name}")),
    })
}

fn key_text(k: &CustomNode) -> Result<String, String> {
    match k {
        CustomNode::Scalar { value, .. } => Ok(value.to_string()),
        other => Err(format!("non-scalar mapping key: {other:?}")),
    }
}

/// Parse JSON text into the AST with the bindings `from_json` quoting
/// policy: strings that would re-resolve to another type are quoted,
/// plain text stays plain (minimal-quoting).
pub fn json_to_node(text: &str) -> Result<CustomNode, String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("JSON parse error: {e}"))?;
    Ok(json_value_to_node(&value))
}

/// A JSON string projected onto the AST: quoted only when it would
/// otherwise re-resolve (numbers, bool words, null words, whitespace).
fn str_to_scalar(s: &str) -> CustomNode {
    if needs_quotes(s) {
        CustomNode::Scalar {
            value: s.into(),
            style: ScalarStyle::DoubleQuoted,
            chomping: Default::default(),
            meta: NodeMeta::default(),
        }
    } else {
        CustomNode::plain_scalar(s)
    }
}

fn json_value_to_node(value: &serde_json::Value) -> CustomNode {
    // Number identities survive via text round-trip; re-parsing is the
    // bindings' own approach (json_value_to_node in py/python_types.rs).
    match value {
        serde_json::Value::Null => CustomNode::plain_null(),
        serde_json::Value::Bool(b) => CustomNode::plain_scalar(b.to_string()),
        serde_json::Value::Number(n) => CustomNode::plain_scalar(n.to_string()),
        serde_json::Value::String(s) => str_to_scalar(s),
        serde_json::Value::Array(items) => {
            CustomNode::plain_sequence(items.iter().map(json_value_to_node).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut pairs = indexmap::IndexMap::new();
            for (k, v) in obj {
                pairs.insert(str_to_scalar(k), json_value_to_node(v));
            }
            CustomNode::plain_mapping(pairs)
        }
    }
}
