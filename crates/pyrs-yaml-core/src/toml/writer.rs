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
    let mut sections: Vec<(String, &CustomNode, Option<&CustomNode>)> = Vec::new();
    for (k, v) in pairs {
        let key_str = scalar_key(k)?;
        match v {
            CustomNode::Mapping { pairs: inner, .. } if !inner.is_empty() => {
                sections.push((key_str, v, Some(k)));
            }
            other => {
                emit_pair(&mut out, k, other, &key_str)?;
            }
        }
    }
    for (name, tbl, key_node) in sections {
        // Standalone comment block sits above the header on its own line;
        // the mapping's own meta.comment rides on the header line after
        // `]` as an inline trailing note.
        if let Some(k) = key_node
            && let Some(c) = k.comment()
            && c.standalone
        {
            let _ = writeln!(out, "# {}", c.text);
        }
        let header_inline = tbl.comment();
        match header_inline {
            Some(c) if !c.standalone => {
                let _ = writeln!(out, "[{name}] # {}", c.text);
            }
            _ => {
                let _ = writeln!(out, "[{name}]");
            }
        }
        let CustomNode::Mapping { pairs, .. } = tbl else {
            unreachable!("sections collected are mappings");
        };
        for (k, v) in pairs {
            let key_str = scalar_key(k)?;
            emit_pair(&mut out, k, v, &key_str)?;
        }
    }
    Ok(out)
}

/// Emit one `key = value` line together with any leading (standalone) or
/// trailing (inline) comments attached via `NodeMeta::comment`.
fn emit_pair(
    out: &mut String,
    key_node: &CustomNode,
    value_node: &CustomNode,
    key_str: &str,
) -> Result<(), SerializeError> {
    if let Some(c) = key_node.comment()
        && c.standalone
    {
        let _ = writeln!(out, "# {}", c.text);
    }
    let value_text = value_str(value_node)?;
    if let Some(c) = value_node.comment()
        && !c.standalone
    {
        let _ = writeln!(out, "{key_str} = {value_text} # {}", c.text);
    } else {
        let _ = writeln!(out, "{key_str} = {value_text}");
    }
    Ok(())
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
                // Fidelity pass-through: when the plain scalar's textual
                // form is already a valid TOML integer literal (`0xDEAD`,
                // `-0`, plain decimal), emit it verbatim. Otherwise fall
                // back to the canonical `i64` rendering.
                (ScalarStyle::Plain, YamlType::Int(i)) => {
                    if is_toml_int_literal(value) {
                        Ok(value.to_string())
                    } else {
                        Ok(i.to_string())
                    }
                }
                // Same rule for floats: an exponent spelling (`1e10`) or
                // an `inf`/`nan` form the source produced is passed
                // through; everything else is normalised via
                // `fmt_toml_float`.
                (ScalarStyle::Plain, YamlType::Float(f)) => {
                    if is_toml_float_literal(value) {
                        Ok(value.to_string())
                    } else {
                        Ok(fmt_toml_float(f))
                    }
                }
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

/// Whether the plain scalar text is already a legal TOML integer
/// literal (decimal, `0x` hex, `0o` octal, `0b` binary, with optional
/// leading `-`). Used by the writer to decide whether to pass through
/// verbatim instead of re-rendering from `i64`.
fn is_toml_int_literal(text: &str) -> bool {
    let (sign_rest, signed) = match text.strip_prefix('-') {
        Some(rest) => (rest, true),
        None => (text, false),
    };
    let _ = signed;
    // Reject `+` prefix and empty remainder.
    if sign_rest.is_empty() || sign_rest.starts_with('+') {
        return false;
    }
    // Radix-prefixed forms: YAML Core parses these too, so the AST
    // value can carry them and we want to preserve the spelling.
    if sign_rest.len() > 2 {
        let (prefix, digits) = (&sign_rest[..2], &sign_rest[2..]);
        let radix_ok = match prefix {
            "0x" => !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_hexdigit()),
            "0o" => !digits.is_empty() && digits.bytes().all(|b| matches!(b, b'0'..=b'7')),
            "0b" => !digits.is_empty() && digits.bytes().all(|b| matches!(b, b'0' | b'1')),
            _ => false,
        };
        if radix_ok {
            return true;
        }
    }
    // Plain decimal: every char is `[0-9]` and either single-digit or
    // not starting with `0` (TOML 1.0 disallows leading zeros).
    if sign_rest.bytes().all(|b| b.is_ascii_digit()) {
        return sign_rest.len() == 1 || !sign_rest.starts_with('0');
    }
    false
}

/// Whether the plain scalar text is a legal TOML float spelling that we
/// want to preserve verbatim (exponent form or explicit fraction whose
/// canonical `f64` rendering drifts from source). `.inf`/`nan` keyword
/// forms are intentionally excluded because the writer canonicalises
/// them to TOML's `inf`/`nan` through `fmt_toml_float`.
fn is_toml_float_literal(text: &str) -> bool {
    // Any `e`/`E` exponent form is worth preserving.
    if text.contains('e') || text.contains('E') {
        // Reject underscore-separated forms and non-numeric shapes;
        // those came from elsewhere (YAML loader, hand-built) and must
        // fall through to canonical rendering.
        return !text.contains('_');
    }
    false
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

    #[test]
    fn preserves_radix_integer_source_spelling() {
        // Fidelity #108: hex and octal integers round-trip with their
        // original prefix + case intact. Underscore separators are
        // stripped (YAML Core does not accept them) and binary integers
        // canonicalise to decimal (YAML Core does not read `0b101`).
        for (src, expected) in [
            ("v = 0xDEADBEEF\n", "v = 0xDEADBEEF\n"),
            ("v = 0o755\n", "v = 0o755\n"),
            ("v = -0x1F\n", "v = -31\n"),
            ("v = 0xFF_FF\n", "v = 0xFFFF\n"),
            ("v = 0b1101_0110\n", "v = 214\n"),
        ] {
            let ast = from_toml(src).unwrap();
            let text = to_toml(&ast).unwrap();
            assert_eq!(text, expected, "radix fidelity lost on {src:?}");
        }
    }

    #[test]
    fn preserves_exponent_float_source_spelling() {
        // Fidelity #108: exponent floats round-trip byte-for-byte.
        for src in [
            "v = 1e10\n",
            "v = -3.14e-2\n",
            "v = 6.02E23\n",
            "v = 0.0e1\n",
        ] {
            let ast = from_toml(src).unwrap();
            let text = to_toml(&ast).unwrap();
            assert_eq!(text, src, "float fidelity lost on {src:?}");
        }
    }

    #[test]
    fn canonicalises_underscore_separators_and_plus_signs() {
        // Deliberate divergence: underscore separators and explicit `+`
        // signs are NOT in YAML Core's numeric grammar, so we canonicalise
        // them to plain decimal (matches #107 behaviour). Users who want
        // the spelling preserved should switch to a TOML-native editor.
        let ast = from_toml("a = 1_000\nb = +42\n").unwrap();
        let text = to_toml(&ast).unwrap();
        assert!(text.contains("a = 1000"), "{text}");
        assert!(text.contains("b = 42"), "{text}");
    }

    #[test]
    fn parse_emit_is_idempotent_on_mixed_forms() {
        // Full-shape round trip: every numeric spelling we support
        // stabilises on the second pass, so `to_toml(from_toml(x))`
        // is a fixed point after one application.
        let src = "hex = 0xDEADBEEF\ndec = 42\nexp = 1e10\nneg = -3\nzero = 0\n";
        let once = to_toml(&from_toml(src).unwrap()).unwrap();
        let twice = to_toml(&from_toml(&once).unwrap()).unwrap();
        assert_eq!(once, twice);
        assert_eq!(once, src);
    }

    #[test]
    fn preserves_trailing_inline_comment_on_kv() {
        // Fidelity #109: `key = val # comment` retains the same-line note.
        let src = "port = 8080 # default\nhost = \"localhost\"\n";
        let out = to_toml(&from_toml(src).unwrap()).unwrap();
        assert_eq!(out, src, "{out}");
    }

    #[test]
    fn preserves_standalone_comment_above_kv() {
        // A `# ...` line above a pair rides into the AST and back out.
        let src = "# section marker\nkey = 1\n";
        let out = to_toml(&from_toml(src).unwrap()).unwrap();
        assert_eq!(out, src, "{out}");
    }

    #[test]
    fn preserves_section_header_trailing_and_leading() {
        // `[name] # note` retains the inline note; a `# ...` block on
        // the line above the header retains its own slot.
        let src = "# above the section\n[srv] # inline\nport = 1\n";
        let out = to_toml(&from_toml(src).unwrap()).unwrap();
        assert_eq!(out, src, "{out}");
    }

    #[test]
    fn preserves_comments_across_mixed_pairs_and_sections() {
        let src =
            "# top\nkey = 1 # same line\n\n# before section\n[sec] # note\ninner = 2 # tail\n";
        let out = to_toml(&from_toml(src).unwrap()).unwrap();
        // The blank-line separator is intentionally not preserved (see
        // design doc: PR #109 covers comment slots only; whitespace
        // fidelity would need a NodeMeta::blank_before field).
        assert!(out.contains("# top\nkey = 1 # same line"), "{out}");
        assert!(out.contains("# before section\n[sec] # note"), "{out}");
        assert!(out.contains("inner = 2 # tail"), "{out}");
    }
}
