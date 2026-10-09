//! CustomNode -> JSON text.
//!
//! Projection rules mirror the historical `to_json` behaviour so the engine
//! swap is behaviour-compatible, with one improvement: plain numbers that
//! carry source spelling (`1e3`) emit verbatim.
//!
//! - plain scalars resolve through the core schema: `true`/`false`/`null`
//!   and finite numbers pass as JSON literals; anything resolving to a
//!   string quotes; non-finite floats emit their text quoted (JSON has no
//!   NaN/Infinity spelling, same as the previous `serde_json` path) - under
//!   JSON5 the dialect's own bare token is emitted instead, derived from the
//!   *value*, never from a word that a YAML string could also spell;
//! - quoted scalars are always JSON strings (never re-typed);
//! - mapping keys are scalar text (resolved scalars stringify); non-scalar
//!   keys are a stable error;
//! - aliases/tags of collection nodes and typed collections are stable
//!   errors: JSON cannot represent them (the caller decides how to report).

use crate::parser::DEFAULT_MAX_DEPTH;
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::Write as _;
use pyrs_ast::ast::{CustomNode, ScalarStyle};
use pyrs_ast::error::{DepthError, SerializeError};
use pyrs_schema::types::{Schema, YamlType};

/// Which JSON-family dialect `write_value` targets. `Json5` is a superset
/// of `Jsonc` (comments) that additionally restores JSON5-only spellings:
/// single-quoted strings and the `0x…` / `.5` / `+7` numeric forms (PR #121),
/// plus the bare `Infinity` / `NaN` tokens of a non-finite float. The quote
/// style and the hex / dot / plus forms come straight off the AST (single-quoted
/// strings carry `ScalarStyle::SingleQuoted`, JSON5 numbers keep their source text
/// as plain scalars), but the infinities are derived from the resolved *value*: the
/// hub spells them `.inf` / `-.inf` / `.nan` so YAML reads them back as numbers
/// (#312), and those words would otherwise be indistinguishable from a string that
/// happens to spell them.
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

/// Emit a root container's own leading (standalone) comments before the
/// top-level value. Nested members get theirs via the pair-loop
/// `emit_standalone_comment`; the outermost node has no preceding key
/// slot, so a document-leading `#`/`//` comment would otherwise drop.
fn emit_root_leading(node: &CustomNode, out: &mut String) {
    for c in node.leading_comments() {
        let _ = writeln!(out, "// {}", c.text.replace(['\n', '\r'], " ").trim());
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

/// Render a `// comment` suffix at the end of the current line. The comment is
/// newline-terminated (and any newlines inside its body are flattened to spaces)
/// because `//` runs to end-of-line: without the terminator the following `,` or
/// `}` would be commented out, producing output that cannot re-parse.
fn emit_inline_comment(node: &CustomNode, out: &mut String) {
    if let Some(c) = node.comment()
        && !c.standalone
    {
        let _ = writeln!(out, " // {}", c.text.replace(['\n', '\r'], " ").trim());
    }
}

/// Emit a standalone comment block on its own line, already indented to
/// `step * level` — every note the node carries, in source order.
///
/// PR #115 puts standalone notes onto the dedicated `leading_comments`
/// slot on the parser side; hand-built fixtures and pre-#115 shapes
/// that still write into `comment` (with `standalone = true`) keep
/// rendering thanks to the normalised read in `leading_comments()`.
fn emit_standalone_comment(node: &CustomNode, step: usize, level: usize, out: &mut String) {
    for c in node.leading_comments() {
        indent(out, step, level);
        let _ = writeln!(out, "// {}", c.text.replace(['\n', '\r'], " ").trim());
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
    // dot, leading `+`) emit as-is — they are legal JSON5 bare tokens,
    // unlike strict JSON which would have to quote them. `Infinity` / `NaN`
    // are deliberately NOT in this set: see `is_json5_number`.
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
        // Non-finite floats. JSON5 has a bare token for the value, so emitting it keeps the number
        // a number in the dialect that can spell it; strict JSON / JSONC have no such spelling and
        // quote the text instead of inventing a literal. The token comes from the resolved *value*,
        // never from the word, because a YAML plain scalar `Infinity` is a string under the core
        // schema and must stay a string (#312).
        YamlType::Float(f) if !f.is_finite() && mode.json5() => {
            out.push_str(if f.is_nan() {
                "NaN"
            } else if f > 0.0 {
                "Infinity"
            } else {
                "-Infinity"
            });
        }
        // Non-finite floats in a dialect that cannot spell them: quote the text so nothing is
        // silently dropped, matching the previous `serde_json` path's fallback for unrepresentable
        // values. This is the open policy question in `ROADMAP.md` (Planned item 2), not a fix here.
        YamlType::Float(_) => write_json_string(value, out),
        YamlType::Str(_) => write_json_string(value, out),
    }
}

/// Whether `text` is a JSON5-only number spelling the parser stored
/// verbatim: a hexadecimal integer, a leading-dot / trailing-dot decimal, or a
/// leading-`+` form.
///
/// `Infinity` and `NaN` are absent on purpose. The hub spells those values `.inf` /
/// `-.inf` / `.nan` (see `crate::parser`), so the words reach this writer only when a document
/// really holds a *string* that happens to be spelled like a JSON5 literal; treating it as a number
/// was the second half of #312, and the bare token for a value is emitted by `write_plain` from the
/// resolved float instead.
fn is_json5_number(text: &str) -> bool {
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
    // `f64::trunc` is an std-only inherent method; under `no_std` the same
    // integral-value test is exact via an integer round-trip: the `1e15` guard
    // keeps `f` under 2^53, so the f64→i64→f64 cast is lossless (and the cast
    // itself is saturating, never UB, even if the guard were removed).
    if f.abs() < 1e15 && (f as i64) as f64 == f {
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
    out.extend(core::iter::repeat_n(' ', step * level));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::from_json;

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

    // YAML-source projections (`parse -> to_json_text*`) need the YAML parser
    // and live in `pyrs-yaml-core` `src/integration/json_family.rs`; this
    // crate must not reach back into the YAML parser.

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
        let n = crate::from_jsonc(&src[..src.len() - 1]).unwrap();
        let out = to_jsonc_text_pretty(&n, 2).unwrap();
        assert!(out.contains("\"port\": 8080 // default port"), "{out}");
    }

    #[test]
    fn jsonc_preserves_standalone_line_comment() {
        // A `// ...` on its own line above a pair rides onto the key
        // node's `standalone` slot, then back to its own line before
        // the `key: value` pair.
        let src = "{\n  // section header\n  \"k\": 1\n}";
        let n = crate::from_jsonc(src).unwrap();
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
        let n = crate::from_jsonc(src).unwrap();
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
        let n = crate::from_jsonc(src).unwrap();
        let out = to_jsonc_text_pretty(&n, 2).unwrap();
        assert!(out.contains("// lead"), "missing leading: {out}");
        assert!(out.contains("// trail"), "missing trailing: {out}");
    }

    #[test]
    fn jsonc_block_comment_renders_as_line_comment() {
        // Block comments collapse to `//` on emit — the AST stores only
        // the body text, matching the YAML receiver's `Comment` model.
        let src = "{\n  /* note */\n  \"a\": 1\n}";
        let n = crate::from_jsonc(src).unwrap();
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
        let n = crate::from_jsonc("{\"a\": 1 // x\n}").unwrap();
        let s = to_json_text(&n).unwrap();
        assert!(!s.contains("//"), "{s}");
        assert!(!s.contains('x'), "{s}");
    }

    #[test]
    fn json5_restores_single_quoted_strings() {
        // PR #121: a JSON5 single-quoted string round-trips back to
        // single quotes; the strict / JSONC writers still emit double
        // quotes for the same AST (JSON has no single-quote form).
        let n = crate::from_json5("{ 'name': 'chen' }").unwrap();
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
        // `Infinity` / `-Infinity` / `NaN` are in this list because the dialect output is identical
        // either way; they now reach it through the hub's `.inf` / `.nan` spelling and the
        // value-derived token below, not through a preserved word.
        for src in ["0xDECAF", ".5", "5.", "+7", "Infinity", "-Infinity", "NaN"] {
            let n = crate::from_json5(src).unwrap();
            let j5 = to_json5_text(&n).unwrap();
            assert_eq!(j5, src, "json5 should keep {src}");
        }
    }

    // YAML-source root-comment fidelity (`parse → to_jsonc_text_pretty`) is an
    // engine-level scenario and lives in `pyrs-yaml-core`
    // `src/integration/json_family.rs`; this crate must not reach back into
    // the YAML parser.

    #[test]
    fn json5_infinity_is_written_from_the_value_not_from_a_word() {
        // #312's writer half. The hub carries YAML's own float spelling, so the dialect's token has to
        // be derived by resolving it: emitting the text would hand a reader `".inf"`, a string in
        // every JSON dialect. These nodes are built directly rather than parsed, because `.inf` is the
        // hub's spelling and not a JSON5 token - the JSON5 reader would be right to reject it.
        for (hub, want) in [
            (".inf", "Infinity"),
            ("-.inf", "-Infinity"),
            (".nan", "NaN"),
        ] {
            let n = CustomNode::plain_scalar(hub);
            let j5 = to_json5_text(&n).unwrap();
            assert_eq!(j5, want, "json5 should spell {hub} as {want}");
            // The dialect's own token reads back to the same hub spelling, which is what makes the
            // pair a round trip rather than a one-way rendering.
            let back = crate::from_json5(&j5).unwrap();
            assert_eq!(
                to_json5_text(&back).unwrap(),
                j5,
                "{j5} must be a fixed point"
            );
            let strict = to_json_text(&n).unwrap();
            assert_eq!(
                strict,
                format!("\"{hub}\""),
                "strict JSON has no spelling for {hub}; quoting is the open policy question, not a \
                 number"
            );
        }
    }

    #[test]
    fn a_string_that_spells_infinity_stays_a_string() {
        // The other half of #312, and the reason `is_json5_number` no longer lists these words. Under
        // the core schema `Infinity` is a *string* - the hub's spelling of the number is `.inf` - so
        // emitting the word bare turned text into a number on the way out. This was pinned as a known
        // limitation (`test_json5_ambiguous_spellings_are_bare` in the Python suite); the
        // value-derived writer closes it.
        //
        // `NaN` is deliberately not in this list: the core schema resolves it to a float, so a plain
        // `NaN` *is* a number and the dialect emits it bare. Only the spellings core reads as strings
        // are the ambiguity.
        for word in ["Infinity", "-Infinity", "+Infinity"] {
            let n = CustomNode::plain_scalar(word);
            let j5 = to_json5_text(&n).unwrap();
            assert_eq!(j5, format!("\"{word}\""), "{word} is a string, not a token");
            // And reading that back keeps it a string: the type survives the round trip.
            let back = crate::from_json5(&j5).unwrap();
            assert_eq!(to_json5_text(&back).unwrap(), j5);
        }
    }

    #[test]
    fn json5_round_trip_is_idempotent() {
        // parse -> emit -> parse -> emit reaches a fixed point, and the
        // intermediate AST carries no lost comments or exotic numbers.
        let src = "{ a: 0x1F, b: .5, c: 'str', d: Infinity }";
        let once = to_json5_text(&crate::from_json5(src).unwrap()).unwrap();
        let twice = to_json5_text(&crate::from_json5(&once).unwrap()).unwrap();
        assert_eq!(once, twice, "not a fixed point:\n{once}\n{twice}");
        assert!(once.contains("0x1F"), "{once}");
        assert!(once.contains(".5"), "{once}");
    }
}
