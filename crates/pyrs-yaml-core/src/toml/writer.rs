//! CustomNode -> TOML text.
//!
//! Projection rules mirror the historical `to_toml` behaviour so the engine
//! swap is behaviour-compatible:
//!
//! - top-level non-table values are emitted first as `key = value` lines
//!   (TOML requires this ordering, since `[table]` headers close the top
//!   scope for later bare `key = value` assignments);
//! - direct sub-mappings are emitted as `[name]` sections with their own
//!   key-value lines; deeper or mixed-shape descendants are emitted as
//!   inline tables (`{ k = v, ... }`);
//! - empty mappings render as `{}` to preserve the "explicit table" nature
//!   at the top level (`[name]` header, no children) and as inline `{}`
//!   at value positions;
//! - null values, aliases, and non-scalar keys are stable errors: TOML
//!   cannot represent them.

use crate::ast::{CustomNode, ScalarStyle};
use crate::error::{ParseError, SerializeError};
use crate::parser::yaml::{Schema, YamlType};
use crate::toml::fmt_yaml_float;
use std::fmt::Write as _;

/// Render a value-only AST as TOML text. `SerializeError::Internal` carries
/// the rejection reason for shapes TOML cannot represent.
pub fn to_toml(node: &CustomNode) -> Result<String, SerializeError> {
    let CustomNode::Mapping { pairs, .. } = node else {
        return Err(SerializeError::Internal("toml-requires-table-root"));
    };
    let mut out = String::new();
    let mut sections: Vec<(String, &CustomNode)> = Vec::new();
    for (k, v) in pairs {
        let key_str = scalar_key(k)?;
        match v {
            CustomNode::Mapping { pairs: inner, .. } if !inner.is_empty() => {
                sections.push((key_str, v));
            }
            other => {
                let _ = writeln!(out, "{key_str} = {}", value_str(other)?);
            }
        }
    }
    for (name, tbl) in sections {
        let _ = writeln!(out, "[{name}]");
        let CustomNode::Mapping { pairs, .. } = tbl else {
            unreachable!("sections collected are mappings");
        };
        for (k, v) in pairs {
            let key_str = scalar_key(k)?;
            let _ = writeln!(out, "{key_str} = {}", value_str(v)?);
        }
    }
    Ok(out)
}

fn scalar_key(k: &CustomNode) -> Result<String, SerializeError> {
    match k {
        CustomNode::Scalar { value, .. } => Ok(quote_key(value)),
        _ => Err(SerializeError::Internal("toml-keys-must-be-scalars")),
    }
}

/// Emit a scalar's TOML spelling.
fn value_str(node: &CustomNode) -> Result<String, SerializeError> {
    match node {
        CustomNode::Scalar {
            value, style, meta, ..
        } => {
            // Tagged timestamps round-trip through strict RFC 3339 validation
            // so a malformed text never silently becomes a TOML string.
            if meta.tag.as_ref().is_some_and(|t| {
                t.suffix.contains("timestamp")
                    || t.suffix.contains("datetime")
                    || t.suffix.contains("localDate")
                    || t.suffix.contains("localTime")
                    || t.suffix.contains("localDatetime")
            }) {
                match Schema::Core.resolve(value) {
                    YamlType::Str(_) => {
                        // Only accept when it truly parses as a TOML datetime.
                        if !is_valid_toml_datetime(value) {
                            return Err(SerializeError::Internal("toml-timestamp-malformed"));
                        }
                        return Ok(value.to_string());
                    }
                    _ => return Err(SerializeError::Internal("toml-timestamp-malformed")),
                }
            }
            match (style, Schema::Core.resolve(value)) {
                (ScalarStyle::Plain, YamlType::Null) => {
                    Err(SerializeError::Internal("toml-cannot-represent-null"))
                }
                (ScalarStyle::Plain, YamlType::Bool(b)) => Ok(b.to_string()),
                (ScalarStyle::Plain, YamlType::Int(i)) => Ok(i.to_string()),
                (ScalarStyle::Plain, YamlType::Float(f)) => Ok(fmt_toml_float(f)),
                // Everything else is text: quoted scalars verbatim, plain
                // strings resolved-to-Str.
                _ => Ok(quote_basic(value)),
            }
        }
        CustomNode::Sequence { items, .. } => {
            let mut parts = Vec::with_capacity(items.len());
            for it in items {
                parts.push(value_str(it)?);
            }
            Ok(format!("[{}]", parts.join(", ")))
        }
        CustomNode::Mapping { pairs, .. } => {
            let mut parts = Vec::with_capacity(pairs.len());
            for (k, v) in pairs {
                let key_str = scalar_key(k)?;
                parts.push(format!("{key_str} = {}", value_str(v)?));
            }
            Ok(format!("{{{}}}", parts.join(", ")))
        }
        CustomNode::Null { .. } => Err(SerializeError::Internal("toml-cannot-represent-null")),
        CustomNode::Alias { .. } => Err(SerializeError::Internal("toml-unsupported-node")),
    }
}

fn fmt_toml_float(f: f64) -> String {
    // Reuse the YAML float formatter (which already produces TOML-valid
    // `.inf`/`-.inf`/`.nan` and decimal spelling) then normalise the
    // leading `.` back to `inf`/`nan` for TOML.
    let s = fmt_yaml_float(f);
    match s.as_str() {
        ".inf" => "inf".to_string(),
        "-.inf" => "-inf".to_string(),
        ".nan" => "nan".to_string(),
        _ => s,
    }
}

fn quote_key(s: &str) -> String {
    let bare = !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if bare { s.to_string() } else { quote_basic(s) }
}

fn quote_basic(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => {
                // Collapse a run of newlines into `\n\n…` (TOML 1.0 has no
                // single char for CRLF; a `\r\n` pair is only valid inside
                // a multiline basic string, so we degrade to `\n`).
                out.push_str("\\n");
                while it.peek() == Some(&'\n') {
                    it.next();
                    out.push('\n');
                }
            }
            '\r' => out.push_str("\\r"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04X}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn is_valid_toml_datetime(s: &str) -> bool {
    // Reuse the parser's strict validator via a probe. The parser module
    // owns the grammar so date-time rules live in one place.
    crate::toml::parser::DateTimeProbe.check_all(s)
}

// Silence "unused import" for ParseError under non-test builds.
#[allow(dead_code)]
fn _unused_parse_error(e: ParseError) -> String {
    e.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use crate::toml::from_toml;

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
    fn float_specials_map_both_ways() {
        let ast = from_toml("i = inf\nm = nan\nbig = 1.0e10\n").unwrap();
        let text = to_toml(&ast).unwrap();
        assert!(text.contains("inf"), "{text}");
        assert!(text.contains("nan"), "{text}");
    }

    #[test]
    fn emits_top_level_pairs_before_sections() {
        // Regression: toml_edit put top-level pairs first; the writer must
        // preserve that invariant because TOML requires it.
        let src = "title = \"x\"\n\n[srv]\nport = 1\n";
        let ast = from_toml(src).unwrap();
        let text = to_toml(&ast).unwrap();
        let title_idx = text.find("title = ").expect("title line");
        let srv_idx = text.find("[srv]").expect("[srv] section");
        assert!(
            title_idx < srv_idx,
            "sections must follow top-level: {text}"
        );
    }
}
