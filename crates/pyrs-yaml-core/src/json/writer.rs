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

/// Which JSON-family dialect `write_value` targets. `Json5` is a superset
/// of `Jsonc` (comments) that additionally restores JSON5-only spellings:
/// single-quoted strings and the `0x…` / `.5` / `+7` / `Infinity` / `NaN`
/// numeric forms (PR #121). The quote style and number form come straight
/// off the AST (single-quoted strings carry `ScalarStyle::SingleQuoted`;
/// JSON5 numbers keep their source text as plain scalars), so this writer
/// is purely a projection of what the parser already preserved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Strict RFC 8259: no comments ever emitted.
    Json,
    /// JSONC: emit `//` comments, everything else RFC 8259.
    Jsonc,
    /// JSON5: emit comments plus single-quoted strings and JSON5 numbers.
    Json5,
}

impl Mode {
    fn comments(self) -> bool {
        !matches!(self, Mode::Json)
    }
    fn json5(self) -> bool {
        matches!(self, Mode::Json5)
    }
}

/// Compact JSON.
pub fn to_json_text(node: &CustomNode) -> Result<String, SerializeError> {
    let mut out = String::new();
    write_value(node, false, 0, &mut Vec::new(), &mut out, Mode::Json)?;
    Ok(out)
}

/// JSON pretty-printed with `indent` spaces per level (0 = compact).
pub fn to_json_text_pretty(node: &CustomNode, indent: usize) -> Result<String, SerializeError> {
    if indent == 0 {
        return to_json_text(node);
    }
    let mut out = String::new();
    write_value(node, true, indent, &mut Vec::new(), &mut out, Mode::Json)?;
    Ok(out)
}

/// Compact JSONC: same as [`to_json_text`] but emits `//` comments at the
/// positions the parser recorded them (`NodeMeta::comment`). Comments
/// that live on scalars or collection nodes are rendered as a trailing
/// `// …` after the value; there is no standalone-line placement in the
/// compact form because the writer never inserts line breaks.
pub fn to_jsonc_text(node: &CustomNode) -> Result<String, SerializeError> {
    let mut out = String::new();
    emit_root_leading(node, &mut out);
    write_value(node, false, 0, &mut Vec::new(), &mut out, Mode::Jsonc)?;
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
    emit_root_leading(node, &mut out);
    write_value(node, true, indent, &mut Vec::new(), &mut out, Mode::Jsonc)?;
    Ok(out)
}

/// Compact JSON5: emits `//` comments, restores single-quoted strings
/// (any string the parser tagged `ScalarStyle::SingleQuoted`) and writes
/// the JSON5 numeric forms (`0x…`, `.5`, `5.`, `+7`, `Infinity`, `NaN`)
/// verbatim. Object keys are always quoted (JSON5 permits bare keys but
/// quoting is lossless and simpler). Round-trips `from_json5` output.
pub fn to_json5_text(node: &CustomNode) -> Result<String, SerializeError> {
    let mut out = String::new();
    emit_root_leading(node, &mut out);
    write_value(node, false, 0, &mut Vec::new(), &mut out, Mode::Json5)?;
    Ok(out)
}

/// JSON5 pretty-printed. See [`to_json5_text`] for the JSON5-only rules.
pub fn to_json5_text_pretty(node: &CustomNode, indent: usize) -> Result<String, SerializeError> {
    if indent == 0 {
        return to_json5_text(node);
    }
    let mut out = String::new();
    emit_root_leading(node, &mut out);
    write_value(node, true, indent, &mut Vec::new(), &mut out, Mode::Json5)?;
    Ok(out)
}

/// Emit a root container's own leading (standalone) comment before the
/// top-level value. Nested members get theirs via the pair-loop
/// `emit_standalone_comment`; the outermost node has no preceding key
/// slot, so a document-leading `#`/`//` comment would otherwise drop.
fn emit_root_leading(node: &CustomNode, out: &mut String) {
    if let Some(c) = node.leading_comment() {
        let _ = writeln!(out, "// {}", c.text.replace(['\n', '\r'], " "));
    }
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

/// Write a JSON object key directly into `out`, avoiding the intermediate
/// `String` that [`key_text`] allocates per key (the hot path of `to_json`
/// on documents with many small keys). Produces byte-identical output to
/// `write_json_string(&key_text(k)?, out)`.
fn write_json_key(key: &CustomNode, out: &mut String) -> Result<(), SerializeError> {
    match key {
        CustomNode::Scalar { value, .. } => write_json_string(value, out),
        CustomNode::Null { .. } => write_json_string("null", out),
        _ => return Err(SerializeError::Internal("json-object-key")),
    }
    Ok(())
}

fn write_value(
    node: &CustomNode,
    pretty: bool,
    step: usize,
    stack: &mut Vec<usize>,
    out: &mut String,
    mode: Mode,
) -> Result<(), SerializeError> {
    stack.push(0);
    if stack.len() > DEFAULT_MAX_DEPTH {
        stack.pop();
        return Err(SerializeError::MaxDepthExceeded(DepthError(
            DEFAULT_MAX_DEPTH,
        )));
    }
    let r = write_value_inner(node, pretty, step, stack, out, mode);
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
///
/// PR #115 puts standalone notes onto the dedicated `leading_comment`
/// slot on the parser side; hand-built fixtures and pre-#115 shapes
/// that still write into `comment` (with `standalone = true`) keep
/// rendering thanks to the fallback read here.
fn emit_standalone_comment(node: &CustomNode, step: usize, level: usize, out: &mut String) {
    let c = node
        .leading_comment()
        .or_else(|| node.comment().filter(|c| c.standalone));
    if let Some(c) = c {
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
    mode: Mode,
) -> Result<(), SerializeError> {
    let comments = mode.comments();
    match node {
        CustomNode::Null { .. } => out.push_str("null"),
        CustomNode::Scalar {
            value,
            style: ScalarStyle::Plain,
            ..
        } => write_plain(value, mode, out),
        // PR #121: a single-quoted source string (JSON5-only) round-trips
        // as `'…'`; every other quoted scalar uses `"…"`.
        CustomNode::Scalar {
            value,
            style: ScalarStyle::SingleQuoted,
            ..
        } if mode.json5() => write_single_quoted(value, out),
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
                    write_value(item, pretty, step, stack, out, mode)?;
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
                    write_value(item, pretty, step, stack, out, mode)?;
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
                    write_json_key(k, out)?;
                    out.push_str(": ");
                    write_value(v, pretty, step, stack, out, mode)?;
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
                    write_json_key(k, out)?;
                    out.push(':');
                    write_value(v, pretty, step, stack, out, mode)?;
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

fn write_plain(value: &str, mode: Mode, out: &mut String) {
    // Fidelity first: when the text already spells a JSON number (`1e3`,
    // `-0`, `1.0`), pass it through unchanged so large-precision integers
    // and explicit signed-zero survive a `from_json → to_json` round trip
    // without an f64/i64 detour.
    if is_json_number(value) {
        out.push_str(value);
        return;
    }
    // PR #121: under JSON5, the number spellings the JSON5 parser accepts
    // and stores verbatim on a plain scalar (hexadecimal, leading/trailing
    // dot, leading `+`, `Infinity` / `NaN`) emit as-is — they are legal
    // JSON5 bare tokens, unlike strict JSON which would have to quote them.
    if mode.json5() && is_json5_number(value) {
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
        // `serde_json` path's fallback for unrepresentable values. Under
        // JSON5 the bare `Infinity` / `NaN` spellings were already handled
        // above, so this only catches YAML-flavoured infinities.
        YamlType::Float(_) => write_json_string(value, out),
        YamlType::Str(_) => write_json_string(value, out),
    }
}

/// Whether `text` is a JSON5-only number spelling the parser stored
/// verbatim: `Infinity` / `NaN` (optionally signed), a hexadecimal
/// integer, a leading-dot / trailing-dot decimal, or a leading-`+` form.
fn is_json5_number(text: &str) -> bool {
    match text {
        "Infinity" | "-Infinity" | "+Infinity" | "NaN" => return true,
        _ => {}
    }
    // Strip a single leading sign for the numeric checks below.
    let (sign_len, body) = match text.as_bytes().first() {
        Some(b'+') | Some(b'-') => (1, &text[1..]),
        _ => (0, text),
    };
    let _ = sign_len;
    // hexadecimal integer: 0x / 0X + at least one hex digit.
    if (body.starts_with("0x") || body.starts_with("0X"))
        && body.len() > 2
        && body[2..].bytes().all(|b| b.is_ascii_hexdigit())
    {
        return true;
    }
    // leading-dot `.5` or trailing-dot `5.` (JSON5 permits either).
    if let Some(rest) = body.strip_prefix('.') {
        return !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit());
    }
    if let Some(int) = body.strip_suffix('.') {
        return !int.is_empty() && int.bytes().all(|b| b.is_ascii_digit());
    }
    // leading `+` on an otherwise-strict JSON number (e.g. `+7`, `+1.5`).
    if text.starts_with('+') && is_json_number(body) {
        return true;
    }
    false
}

/// Emit a JSON5 single-quoted string. Only reached under `Mode::Json5`
/// for scalars the parser tagged `ScalarStyle::SingleQuoted`.
fn write_single_quoted(text: &str, out: &mut String) {
    out.push('\'');
    for c in text.chars() {
        match c {
            '\'' => out.push_str("\\'"),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04X}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('\'');
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
    // Fast path: when no byte needs escaping (the common case for short ASCII or
    // already-raw-UTF-8 strings), copy the whole slice in one `push_str` instead
    // of a per-char loop that re-encodes UTF-8 and checks capacity every char —
    // the measured hot spot of `to_json`. Byte-identical to the char loop, which
    // still runs verbatim from the first escapable byte. The boundary `pos` is an
    // ASCII byte (< 0x20, `"` or `\`), so it is always a `char` boundary.
    match text
        .as_bytes()
        .iter()
        .position(|&b| b < 0x20 || b == b'"' || b == b'\\')
    {
        None => out.push_str(text),
        Some(pos) => {
            out.push_str(&text[..pos]);
            for c in text[pos..].chars() {
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
    fn jsonc_member_carries_both_leading_and_trailing_comments() {
        // PR #115: a member with a standalone comment on the line above
        // AND an inline note after its value keeps BOTH slots. The
        // leading note rides onto `NodeMeta::leading_comment`, the
        // trailing on `NodeMeta::comment` — impossible under the
        // single-slot model #112 shipped.
        let src = "{\n  // above\n  \"k\": 1 // after\n}";
        let n = crate::json::from_jsonc(src).unwrap();
        let out = to_jsonc_text_pretty(&n, 2).unwrap();
        assert!(out.contains("// above"), "missing leading: {out}");
        assert!(out.contains("// after"), "missing trailing: {out}");
        assert!(
            out.contains("// above\n  \"k\": 1 // after"),
            "wrong order: {out}"
        );
    }

    #[test]
    fn jsonc_array_element_carries_both_slots() {
        // Array items get the same treatment as object members: a
        // standalone note on its own line plus a trailing note after
        // the value both survive the round trip.
        let src = "[\n  // lead\n  1 // trail\n]";
        let n = crate::json::from_jsonc(src).unwrap();
        let out = to_jsonc_text_pretty(&n, 2).unwrap();
        assert!(out.contains("// lead"), "missing leading: {out}");
        assert!(out.contains("// trail"), "missing trailing: {out}");
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

    #[test]
    fn json5_restores_single_quoted_strings() {
        // PR #121: a JSON5 single-quoted string round-trips back to
        // single quotes; the strict / JSONC writers still emit double
        // quotes for the same AST (JSON has no single-quote form).
        let n = crate::json::from_json5("{ 'name': 'chen' }").unwrap();
        let j5 = to_json5_text(&n).unwrap();
        assert!(j5.contains("'name'") || j5.contains("'chen'"), "{j5}");
        let strict = to_json_text(&n).unwrap();
        assert!(
            !strict.contains('\''),
            "strict must not emit single quotes: {strict}"
        );
    }

    #[test]
    fn json5_emits_numeric_forms_verbatim() {
        // The JSON5 numeric spellings #120 parses stay verbatim through
        // to_json5_text, whereas the strict writer would quote / canonicalise.
        for src in ["0xDECAF", ".5", "5.", "+7", "Infinity", "-Infinity", "NaN"] {
            let n = crate::json::from_json5(src).unwrap();
            let j5 = to_json5_text(&n).unwrap();
            assert_eq!(j5, src, "json5 should keep {src}");
        }
    }

    #[test]
    fn jsonc_preserves_root_leading_comment() {
        // PR #122: a document-level standalone comment attaches to the
        // root container's `leading_comment` slot. Nested members emit
        // theirs via the pair loop; the outermost node has no preceding
        // key, so `emit_root_leading` is what keeps it from dropping.
        let ast = crate::parser::parse("# header\nport: 8080\n", crate::parser::yaml::Schema::Core)
            .unwrap();
        let out = to_jsonc_text_pretty(&ast, 2).unwrap();
        assert!(out.starts_with("// header\n"), "{out}");
        assert!(out.contains("\"port\": 8080"), "{out}");
    }

    #[test]
    fn json5_round_trip_is_idempotent() {
        // parse -> emit -> parse -> emit reaches a fixed point, and the
        // intermediate AST carries no lost comments or exotic numbers.
        let src = "{ a: 0x1F, b: .5, c: 'str', d: Infinity }";
        let once = to_json5_text(&crate::json::from_json5(src).unwrap()).unwrap();
        let twice = to_json5_text(&crate::json::from_json5(&once).unwrap()).unwrap();
        assert_eq!(once, twice, "not a fixed point:\n{once}\n{twice}");
        assert!(once.contains("0x1F"), "{once}");
        assert!(once.contains(".5"), "{once}");
    }
}
