//! CustomNode -> JSON text.
//!
//! Projection rules mirror the historical `to_json` behaviour so the engine
//! swap is behaviour-compatible, with one improvement: plain numbers that
//! carry source spelling (`1e3`) emit verbatim.
//!
//! - plain scalars resolve through the core schema: `true`/`false`/`null`
//!   and finite numbers pass as JSON literals; anything resolving to a
//!   string quotes; non-finite floats emit their text quoted (JSON has no
//!   NaN/Infinity spelling, same as the previous `serde_json` path);
//! - quoted scalars are always JSON strings (never re-typed);
//! - mapping keys are scalar text (resolved scalars stringify); non-scalar
//!   keys are a stable error;
//! - aliases/tags of collection nodes and typed collections are stable
//!   errors: JSON cannot represent them (the caller decides how to report).

use crate::ast::{CustomNode, ScalarStyle};
use crate::error::{DepthError, SerializeError};
use crate::json::parser::DEFAULT_MAX_DEPTH;
use crate::parser::yaml::{Schema, YamlType};
use std::fmt::Write as _;

/// Compact JSON.
pub fn to_json_text(node: &CustomNode) -> Result<String, SerializeError> {
    let mut out = String::new();
    write_value(node, false, 0, &mut Vec::new(), &mut out, false)?;
    Ok(out)
}

/// JSON pretty-printed with `indent` spaces per level (0 = compact).
pub fn to_json_text_pretty(node: &CustomNode, indent: usize) -> Result<String, SerializeError> {
    if indent == 0 {
        return to_json_text(node);
    }
    let mut out = String::new();
    write_value(node, true, indent, &mut Vec::new(), &mut out, false)?;
    Ok(out)
}

/// Compact JSONC: same as [`to_json_text`] but emits `//` comments at the
/// positions the parser recorded them (`NodeMeta::comment`). Comments
/// that live on scalars or collection nodes are rendered as a trailing
/// `// …` after the value; there is no standalone-line placement in the
/// compact form because the writer never inserts line breaks.
pub fn to_jsonc_text(node: &CustomNode) -> Result<String, SerializeError> {
    let mut out = String::new();
    write_value(node, false, 0, &mut Vec::new(), &mut out, true)?;
    Ok(out)
}

/// JSONC pretty-printed. Standalone comments occupy their own line above
/// the pair they annotate; trailing inline comments sit after the value
/// and before the `,` that closes the pair.
pub fn to_jsonc_text_pretty(node: &CustomNode, indent: usize) -> Result<String, SerializeError> {
    if indent == 0 {
        return to_jsonc_text(node);
    }
    let mut out = String::new();
    write_value(node, true, indent, &mut Vec::new(), &mut out, true)?;
    Ok(out)
}

/// The textual form of a mapping key (resolved scalars stringify; the JSON
/// object domain has no other legal keys).
pub fn key_text(key: &CustomNode) -> Result<String, SerializeError> {
    match key {
        CustomNode::Scalar { value, .. } => Ok(value.to_string()),
        CustomNode::Null { .. } => Ok("null".to_string()),
        _ => Err(SerializeError::Internal("json-object-key")),
    }
}

fn write_value(
    node: &CustomNode,
    pretty: bool,
    step: usize,
    stack: &mut Vec<usize>,
    out: &mut String,
    comments: bool,
) -> Result<(), SerializeError> {
    stack.push(0);
    if stack.len() > DEFAULT_MAX_DEPTH {
        stack.pop();
        return Err(SerializeError::MaxDepthExceeded(DepthError(
            DEFAULT_MAX_DEPTH,
        )));
    }
    let r = write_value_inner(node, pretty, step, stack, out, comments);
    stack.pop();
    r
}

/// Render a `// comment` suffix at the end of the current line. Newlines
/// inside the comment body are stripped so a single-line comment cannot
/// escape into the following output.
fn emit_inline_comment(node: &CustomNode, out: &mut String) {
    if let Some(c) = node.comment()
        && !c.standalone
    {
        let _ = write!(out, " // {}", c.text.replace(['\n', '\r'], " "));
    }
}

/// Emit a standalone comment block on its own line, already indented to
/// `step * level`.
fn emit_standalone_comment(node: &CustomNode, step: usize, level: usize, out: &mut String) {
    if let Some(c) = node.comment()
        && c.standalone
    {
        indent(out, step, level);
        let _ = writeln!(out, "// {}", c.text.replace(['\n', '\r'], " "));
    }
}

fn write_value_inner(
    node: &CustomNode,
    pretty: bool,
    step: usize,
    stack: &mut Vec<usize>,
    out: &mut String,
    comments: bool,
) -> Result<(), SerializeError> {
    match node {
        CustomNode::Null { .. } => out.push_str("null"),
        CustomNode::Scalar {
            value,
            style: ScalarStyle::Plain,
            ..
        } => write_plain(value, out),
        CustomNode::Scalar { value, .. } => write_json_string(value, out),
        CustomNode::Sequence { items, .. } => {
            if items.is_empty() {
                out.push_str("[]");
            } else if pretty {
                out.push_str("[\n");
                for (i, item) in items.iter().enumerate() {
                    if comments {
                        emit_standalone_comment(item, step, stack.len(), out);
                    }
                    indent(out, step, stack.len());
                    write_value(item, pretty, step, stack, out, comments)?;
                    if comments {
                        emit_inline_comment(item, out);
                    }
                    if i + 1 < items.len() {
                        out.push_str(",\n");
                    } else {
                        out.push('\n');
                    }
                }
                indent(out, step, stack.len() - 1);
                out.push(']');
            } else {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_value(item, pretty, step, stack, out, comments)?;
                    if comments {
                        emit_inline_comment(item, out);
                    }
                }
                out.push(']');
            }
        }
        CustomNode::Mapping { pairs, .. } => {
            if pairs.is_empty() {
                out.push_str("{}");
            } else if pretty {
                out.push_str("{\n");
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if comments {
                        emit_standalone_comment(k, step, stack.len(), out);
                    }
                    indent(out, step, stack.len());
                    write_json_string(&key_text(k)?, out);
                    out.push_str(": ");
                    write_value(v, pretty, step, stack, out, comments)?;
                    if comments {
                        emit_inline_comment(v, out);
                    }
                    if i + 1 < pairs.len() {
                        out.push_str(",\n");
                    } else {
                        out.push('\n');
                    }
                }
                indent(out, step, stack.len() - 1);
                out.push('}');
            } else {
                out.push('{');
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_json_string(&key_text(k)?, out);
                    out.push(':');
                    write_value(v, pretty, step, stack, out, comments)?;
                    if comments {
                        emit_inline_comment(v, out);
                    }
                }
                out.push('}');
            }
        }
        // JSON cannot carry alias indirection; stable reason keys keep
        // process-level matching possible (tags are resolved away, the
        // historical serde_json projection behaviour).
        CustomNode::Alias { .. } => {
            return Err(SerializeError::Internal("json-cannot-represent-alias"));
        }
    }
    Ok(())
}

fn write_plain(value: &str, out: &mut String) {
    // Fidelity first: when the text already spells a JSON number (`1e3`,
    // `-0`, `1.0`), pass it through unchanged so large-precision integers
    // and explicit signed-zero survive a `from_json → to_json` round trip
    // without an f64/i64 detour.
    if is_json_number(value) {
        out.push_str(value);
        return;
    }
    match Schema::Core.resolve(value) {
        // JSON's literal spellings for null / bool / int are canonical:
        // YAML allows `~`, `Null`, `TRUE`, `0x1F`, `0o17` — a JSON reader
        // would reject any of those, so we always normalise to the JSON
        // form rather than passing the source text through.
        YamlType::Null => out.push_str("null"),
        YamlType::Bool(true) => out.push_str("true"),
        YamlType::Bool(false) => out.push_str("false"),
        YamlType::Int(i) => {
            let _ = write!(out, "{i}");
        }
        // Non-source-spelled finite floats (hex/oct forms resolved as ints
        // already returned above; this branch catches YAML notations like
        // `0.5e1` or `1_000` where the source text is not JSON-valid).
        YamlType::Float(f) if f.is_finite() => {
            out.push_str(&canonical_float(f));
        }
        // Non-finite floats (`.inf`, `.nan`) are not JSON literals; quote
        // their text so nothing is silently dropped, matching the previous
        // `serde_json` path's fallback for unrepresentable values.
        YamlType::Float(_) => write_json_string(value, out),
        YamlType::Str(_) => write_json_string(value, out),
    }
}

fn is_json_number(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut i = usize::from(bytes.first() == Some(&b'-'));
    let start = i;
    match bytes.get(i) {
        Some(b'0') => {
            i += 1;
        }
        Some(b'1'..=b'9') => {
            while matches!(bytes.get(i), Some(b'0'..=b'9')) {
                i += 1;
            }
        }
        _ => return false,
    }
    if i == start {
        return false;
    }
    if bytes.get(i) == Some(&b'.') {
        i += 1;
        let fs = i;
        while matches!(bytes.get(i), Some(b'0'..=b'9')) {
            i += 1;
        }
        if i == fs {
            return false;
        }
    }
    if matches!(bytes.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(bytes.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let es = i;
        while matches!(bytes.get(i), Some(b'0'..=b'9')) {
            i += 1;
        }
        if i == es {
            return false;
        }
    }
    i == bytes.len()
}

fn canonical_float(f: f64) -> String {
    if f.trunc() == f && f.abs() < 1e15 {
        format!("{f}.0")
    } else {
        format!("{f}")
    }
}

fn write_json_string(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn indent(out: &mut String, step: usize, level: usize) {
    out.extend(std::iter::repeat_n(' ', step * level));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::parser::from_json;
    use crate::parser;

    #[test]
    fn round_trips_json_documents_byte_stably() {
        for text in [
            r#"{"a":1,"b":[true,null,"x"],"c":{},"d":[]}"#,
            "1e3",
            "\"escaped \\\" and \u{221a}\"",
            "[1.0,-0,0.5]",
        ] {
            let n = from_json(text).unwrap();
            let out = to_json_text(&n).unwrap();
            assert_eq!(out, text, "identity for {text:?}");
        }
    }

    #[test]
    fn yaml_nodes_project_like_the_serde_json_path() {
        let node = parser::parse(
            "s: hello\nn: 42\nf: 1.5\nb: true\nq: \"52\"\nnl: ~\nlist: [1, two]\n",
            Schema::Core,
        )
        .unwrap();
        let text = to_json_text_pretty(&node, 2).unwrap();
        assert!(text.contains("\"s\": \"hello\""), "{text}");
        assert!(text.contains("\"n\": 42"), "{text}");
        assert!(text.contains("\"f\": 1.5"), "{text}");
        assert!(text.contains("\"b\": true"), "{text}");
        assert!(text.contains("\"q\": \"52\""), "{text}"); // quoted stays string
        assert!(text.contains("\"nl\": null"), "{text}");
        assert!(text.contains("\"two\""), "{text}");
    }

    #[test]
    fn exotic_scalars_quote_or_normalize() {
        let node = parser::parse("a: 0x1F\nb: .inf\nc: !!str 7\n", Schema::Core).unwrap();
        let text = to_json_text(&node).unwrap();
        assert!(text.contains("\"a\":31"), "{text}"); // hex normalizes
        assert!(text.contains("\"b\":\".inf\""), "{text}"); // non-finite as text
        assert!(text.contains("\"c\":7"), "{text}"); // plain+tag resolves like serde path did
    }

    #[test]
    fn alias_and_non_scalar_keys_are_stable_errors() {
        let node = parser::parse("a: &x 1\nb: *x\n", Schema::Core).unwrap();
        let CustomNode::Mapping { pairs, .. } = &node else {
            unreachable!()
        };
        // resolve alias manually is out of scope; the alias node errors
        let (_, b_val) = pairs.get_index(1).unwrap();
        let err = to_json_text(b_val).unwrap_err();
        assert!(format!("{err:?}").contains("json-cannot-represent-alias"));
        // a sequence key cannot be a JSON object key
        let keyed = CustomNode::Mapping {
            pairs: [(
                CustomNode::plain_sequence(vec![CustomNode::plain_scalar("k")]),
                CustomNode::plain_scalar("v"),
            )]
            .into_iter()
            .collect(),
            flow_style: false,
            meta: Default::default(),
        };
        assert!(matches!(
            to_json_text(&keyed),
            Err(SerializeError::Internal("json-object-key"))
        ));
    }

    #[test]
    fn control_characters_and_escaping() {
        let text = to_json_text(&CustomNode::double_quoted_scalar("a\u{1}b")).unwrap();
        assert_eq!(text, "\"a\\u0001b\"");
        // from_json -> to_json keeps \u0001 escapes stable
        let n = from_json(r#""a\u0001b""#).unwrap();
        assert_eq!(to_json_text(&n).unwrap(), r#""a\u0001b""#);
    }

    #[test]
    fn pretty_shape_matches_serde_style() {
        let n = from_json(r#"{"a":[1,{"b":2}]}"#).unwrap();
        let text = to_json_text_pretty(&n, 2).unwrap();
        assert_eq!(
            text,
            "{\n  \"a\": [\n    1,\n    {\n      \"b\": 2\n    }\n  ]\n}"
        );
    }

    #[test]
    fn jsonc_preserves_trailing_line_comment() {
        // A `// ...` right after a value stays on the same line, before
        // the `,`. The parser attaches it to the value node's meta so the
        // writer can place it correctly without positional replay.
        let src = "{\n  \"port\": 8080 // default port\n}\n";
        let n = crate::json::from_jsonc(&src[..src.len() - 1]).unwrap();
        let out = to_jsonc_text_pretty(&n, 2).unwrap();
        assert!(out.contains("\"port\": 8080 // default port"), "{out}");
    }

    #[test]
    fn jsonc_preserves_standalone_line_comment() {
        // A `// ...` on its own line above a pair rides onto the key
        // node's `standalone` slot, then back to its own line before
        // the `key: value` pair.
        let src = "{\n  // section header\n  \"k\": 1\n}";
        let n = crate::json::from_jsonc(src).unwrap();
        let out = to_jsonc_text_pretty(&n, 2).unwrap();
        assert!(out.contains("// section header\n  \"k\": 1"), "{out}");
    }

    #[test]
    fn jsonc_block_comment_renders_as_line_comment() {
        // Block comments collapse to `//` on emit — the AST stores only
        // the body text, matching the YAML receiver's `Comment` model.
        let src = "{\n  /* note */\n  \"a\": 1\n}";
        let n = crate::json::from_jsonc(src).unwrap();
        let out = to_jsonc_text_pretty(&n, 2).unwrap();
        assert!(out.contains("// note"), "{out}");
        assert!(!out.contains("/*"), "{out}");
    }

    #[test]
    fn strict_writer_ignores_comments() {
        // `to_json_text` stays strict RFC 8259: even if the AST carries
        // comments (from a JSONC parse), the strict writer emits no
        // `//` sequences. Guards the round-trip contract for consumers
        // who pass JSONC ASTs through the plain JSON path.
        let n = crate::json::from_jsonc("{\"a\": 1 // x\n}").unwrap();
        let s = to_json_text(&n).unwrap();
        assert!(!s.contains("//"), "{s}");
        assert!(!s.contains('x'), "{s}");
    }
}
