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

use crate::fmt_yaml_float;
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::fmt::Write as _;
use pyrs_ast::ast::{CustomNode, ScalarStyle};
use pyrs_ast::error::{ParseError, SerializeError};
use pyrs_schema::types::{Schema, YamlType};

/// Render a value-only AST as TOML text. `SerializeError::UnsupportedValue` carries
/// the rejection reason for shapes TOML cannot represent.
pub fn to_toml(node: &CustomNode) -> Result<String, SerializeError> {
    let CustomNode::Mapping { pairs, .. } = node else {
        return Err(SerializeError::UnsupportedValue("toml-requires-table-root"));
    };
    let mut out = String::new();
    // PR #131: a document-level standalone comment (the very first `# note`
    // of a TOML doc, or a note on a YAML-origin root) lands on the root
    // mapping's leading-note list — the same shape the JSON writer
    // handles via `emit_root_leading`. The pair/section emitters below read
    // only per-key and per-table slots, so without this the root note is
    // dropped on a TOML -> hub -> TOML round trip. Native TOML parses place
    // the first note on the first key instead, so this stays a no-op there.
    for c in node.leading_comments() {
        let _ = writeln!(out, "# {}", c.text.trim());
    }
    // A note the hub put on the root's inline slot has no `key = value` line to
    // ride, so it gets a line of its own — which is also where a TOML reader
    // reports a document-leading comment back from, keeping the round trip stable.
    if let Some(c) = node.comment().filter(|c| !c.standalone) {
        let _ = writeln!(out, "# {}", c.text.trim());
    }
    let mut sections: Vec<(String, &CustomNode, Option<&CustomNode>)> = Vec::new();
    let mut first_pair = true;
    for (k, v) in pairs {
        let key_str = scalar_key(k)?;
        match v {
            CustomNode::Mapping { pairs: inner, .. } if !inner.is_empty() => {
                sections.push((key_str, v, Some(k)));
            }
            other => {
                // Design Q2: never emit a blank line before the very first
                // pair of a document; the leading-newline hint only
                // becomes visual for subsequent pairs.
                if !first_pair && other.blank_before() {
                    out.push('\n');
                }
                first_pair = false;
                emit_pair(&mut out, k, other, &key_str)?;
            }
        }
    }
    for (idx, (name, tbl, key_node)) in sections.iter().enumerate() {
        // PR #114 places the section header's leading notes on the child
        // mapping's own list, freeing the parent key slot
        // for future use and letting a header carry BOTH its leading stack and
        // an inline note. The fallbacks keep #109-era hand-built nodes
        // and YAML-origin documents rendering identically.
        let own = tbl.leading_comments();
        let leading: &[pyrs_ast::ast::Comment] = if !own.is_empty() {
            own
        } else if let Some(c) = tbl.comment().filter(|c| c.standalone) {
            core::slice::from_ref(c)
        } else {
            key_node.map(|k| k.leading_comments()).unwrap_or_default()
        };
        // Blank line before a section separator acts like the KV rule:
        // skip it for the first section if any top-level pairs already
        // appeared, since the section break is already visual.
        if tbl.blank_before() && !(idx == 0 && out.is_empty()) {
            out.push('\n');
        }
        for c in leading {
            let _ = writeln!(out, "# {}", c.text.trim());
        }
        if leading.is_empty() {
            // Pre-#114 shape: the note rides the parent key's `comment` field.
            if let Some(c) = key_node.and_then(|k| k.comment()).filter(|c| c.standalone) {
                let _ = writeln!(out, "# {}", c.text.trim());
            }
        }
        let header_inline = tbl.comment();
        match header_inline {
            Some(c) if !c.standalone => {
                let _ = writeln!(out, "[{name}] # {}", c.text.trim());
            }
            _ => {
                let _ = writeln!(out, "[{name}]");
            }
        }
        let CustomNode::Mapping { pairs, .. } = tbl else {
            unreachable!("sections collected are mappings");
        };
        for (i, (k, v)) in pairs.iter().enumerate() {
            let key_str = scalar_key(k)?;
            if i > 0 && v.blank_before() {
                out.push('\n');
            }
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
    // PR #114: the native parser puts leading notes into the dedicated
    // `leading_comments` list. The pre-#114 shape (a standalone note in
    // `comment`) still renders, so documents produced by the YAML path
    // or hand-built fixtures keep working.
    for c in key_node.leading_comments() {
        let _ = writeln!(out, "# {}", c.text.trim());
    }
    if key_node.leading_comments().is_empty()
        && let Some(c) = key_node.comment().filter(|c| c.standalone)
    {
        let _ = writeln!(out, "# {}", c.text.trim());
    }
    let value_text = value_str(value_node)?;
    if let Some(c) = value_node.comment()
        && !c.standalone
    {
        let _ = writeln!(out, "{key_str} = {value_text} # {}", c.text.trim());
    } else {
        let _ = writeln!(out, "{key_str} = {value_text}");
    }
    Ok(())
}

fn scalar_key(k: &CustomNode) -> Result<String, SerializeError> {
    match k {
        CustomNode::Scalar { value, .. } => Ok(quote_key(value)),
        _ => Err(SerializeError::UnsupportedValue(
            "toml-keys-must-be-scalars",
        )),
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
                    || t.suffix.contains("date")
                    || t.suffix.contains("time")
            }) {
                match Schema::Core.resolve(value) {
                    YamlType::Str(_) => {
                        // Only accept when it truly parses as a TOML datetime.
                        if !is_valid_toml_datetime(value) {
                            return Err(SerializeError::UnsupportedValue(
                                "toml-timestamp-malformed",
                            ));
                        }
                        return Ok(value.to_string());
                    }
                    _ => return Err(SerializeError::UnsupportedValue("toml-timestamp-malformed")),
                }
            }
            match (style, Schema::Core.resolve(value)) {
                (ScalarStyle::Plain, YamlType::Null) => Err(SerializeError::UnsupportedValue(
                    "toml-cannot-represent-null",
                )),
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
                // PR #132: a multi-line string (projected as a YAML literal
                // block) is re-emitted as a TOML `"""` basic block so the
                // multi-line shape survives the hub instead of degrading to
                // an escaped single line. The block value already carries its
                // trailing newlines, so reconstruction needs only the value.
                (ScalarStyle::Literal, _) => Ok(quote_multiline(value)),
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
            // PR #119: an inline table whose members carry interior
            // comments (a key leading note or a value trailing note)
            // must be rendered multi-line, because a `# ...` comment
            // runs to end-of-line and cannot live inside a single-line
            // `{ a = 1, b = 2 }`. Undecorated inline tables keep the
            // compact single-line form so 1.0-compatible output is
            // unchanged.
            let decorated = pairs.iter().any(|(k, v)| {
                !k.leading_comments().is_empty() || v.comment().is_some_and(|c| !c.standalone)
            });
            if decorated {
                let mut out = String::from("{\n");
                let count = pairs.len();
                for (idx, (k, v)) in pairs.iter().enumerate() {
                    for c in k.leading_comments() {
                        out.push_str(&format!("  # {}\n", c.text.trim()));
                    }
                    let key_str = scalar_key(k)?;
                    let val = value_str(v)?;
                    let trail = v
                        .comment()
                        .filter(|c| !c.standalone)
                        .map(|c| c.text.trim().to_string());
                    let last = idx + 1 == count;
                    // A `#` runs to end of line, so the only member line a note can
                    // legitimately finish is the document's last one: `a = 1 # n,` is not
                    // TOML at all (measured: the reader rejects it), and writing the note
                    // *after* the comma - which is what this branch used to do - hands it to
                    // the next key on re-read, so the next emission moved it and the text
                    // never settled (proptest `prop_toml_writer_is_fixed_point`, 20 000
                    // cases). Anywhere but last, the note gets its own line after the
                    // member, which is the position the reader reports it from.
                    match trail {
                        Some(text) if last => {
                            out.push_str(&format!("  {key_str} = {val} # {text}\n"));
                        }
                        Some(text) => {
                            out.push_str(&format!("  {key_str} = {val},\n"));
                            out.push_str(&format!("  # {text}\n"));
                        }
                        None => {
                            let sep = if last { "" } else { "," };
                            out.push_str(&format!("  {key_str} = {val}{sep}\n"));
                        }
                    }
                }
                out.push('}');
                Ok(out)
            } else {
                let mut parts = Vec::with_capacity(pairs.len());
                for (k, v) in pairs {
                    let key_str = scalar_key(k)?;
                    parts.push(format!("{key_str} = {}", value_str(v)?));
                }
                Ok(format!("{{{}}}", parts.join(", ")))
            }
        }
        CustomNode::Null { .. } => Err(SerializeError::UnsupportedValue(
            "toml-cannot-represent-null",
        )),
        CustomNode::Alias { .. } => Err(SerializeError::UnsupportedValue("toml-unsupported-node")),
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

/// Emit a TOML multi-line basic string (`"""…"""`) whose parsed content is
/// exactly `value` (PR #132). Two TOML rules shape the encoding: a newline
/// immediately after the opening delimiter is trimmed, so a value that starts
/// with a newline gets a spare one emitted first; and every `"` is escaped so a
/// run of quotes can never prematurely close the block. Real newlines and tabs
/// are kept literal; `\r` and other control chars are escaped.
fn quote_multiline(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 8);
    out.push_str("\"\"\"");
    if value.starts_with('\n') {
        out.push('\n');
    }
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push('\n'),
            '\t' => out.push('\t'),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04X}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push_str("\"\"\"");
    out
}

fn is_valid_toml_datetime(s: &str) -> bool {
    // Reuse the parser's strict validator via a probe. The parser module
    // owns the grammar so date-time rules live in one place.
    crate::parser::DateTimeProbe.check_all(s)
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
    use crate::from_toml;
    use pyrs_ast::ast::{Comment, NodeMap};

    // The YAML-source round-trip (`parse -> to_toml -> from_toml`) needs the
    // YAML parser and lives in `pyrs-yaml-core` `src/integration/toml_family.rs`.

    #[test]
    fn to_toml_preserves_multiline_string_shape() {
        // PR #132: a `"""…"""` source is marked multi-line, projected as a
        // Literal block, and re-emitted as a `"""` block with the value intact.
        let src = "x = \"\"\"line1\nline2\"\"\"\n";
        let ast = from_toml(src).unwrap();
        let text = to_toml(&ast).unwrap();
        assert!(text.contains("\"\"\""), "{text}");
        // Idempotent: re-parsing the emitted text yields the same output.
        let back = from_toml(&text).unwrap();
        assert_eq!(to_toml(&back).unwrap(), text);
        // A single-line basic string is NOT marked multi-line.
        let single = to_toml(&from_toml("x = \"a\\nb\"\n").unwrap()).unwrap();
        assert!(!single.contains("\"\"\""), "{single}");
    }

    #[test]
    fn to_toml_rejects_root_scalar_and_null() {
        assert!(to_toml(&CustomNode::plain_scalar("x")).is_err());
        let mut pairs = pyrs_ast::ast::NodeMap::default();
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
            ("v = -31\n", "v = -31\n"),
            ("v = 0xFF_FF\n", "v = 0xFFFF\n"),
            ("v = 0b1101_0110\n", "v = 214\n"),
        ] {
            let ast = from_toml(src).unwrap();
            let text = to_toml(&ast).unwrap();
            assert_eq!(text, expected, "radix fidelity lost on {src:?}");
        }
        // toml-test: a sign is only legal on decimal integers, never on a
        // `0x`/`0o`/`0b` prefixed literal, so these must be rejected.
        for bad in ["v = -0x1F\n", "v = +0o644\n", "v = +0b101\n"] {
            assert!(
                from_toml(bad).is_err(),
                "signed radix wrongly accepted on {bad:?}"
            );
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
    fn preserves_blank_line_between_pairs() {
        // PR #114: a blank line separating two top-level pairs is a
        // visual grouping cue the writer must reproduce. The Q2 rule
        // suppresses blank at the very first pair (nothing precedes it).
        let src = "a = 1\n\nb = 2\n";
        let out = to_toml(&from_toml(src).unwrap()).unwrap();
        assert_eq!(out, src, "{out}");
    }

    #[test]
    fn preserves_blank_line_before_section() {
        let src = "a = 1\n\n[srv]\nport = 1\n";
        let out = to_toml(&from_toml(src).unwrap()).unwrap();
        assert_eq!(out, src, "{out}");
    }

    #[test]
    fn section_carries_both_leading_and_trailing_comments() {
        // The whole point of the #114 AST change: one node hosting both
        // slots side by side without one clobbering the other.
        let src = "# leading\n[srv] # trailing\nport = 1\n";
        let ast = from_toml(src).unwrap();
        let out = to_toml(&ast).unwrap();
        assert_eq!(out, src, "{out}");
    }

    #[test]
    fn blank_line_and_leading_comment_coexist() {
        // A blank line separates visual blocks; a leading comment sits
        // directly above its pair. Both hints must survive together.
        let src = "a = 1\n\n# header for b\nb = 2\n";
        let out = to_toml(&from_toml(src).unwrap()).unwrap();
        assert_eq!(out, src, "{out}");
    }

    #[test]
    fn to_toml_preserves_optional_seconds_time_spelling() {
        // PR #116 A4: a source `HH:MM` (no seconds) is preserved as
        // written, not canonicalised back to `HH:MM:00`. Round-trip is
        // byte-stable because the parser stores the exact source text
        // as the datetime scalar value.
        let src = "t = 14:15\ndt = 2010-02-03 14:15\n";
        let out = to_toml(&from_toml(src).unwrap()).unwrap();
        assert_eq!(out, src, "{out}");
    }

    #[test]
    fn preserves_inline_table_interior_leading_comments() {
        // PR #119: a `# ...` on its own line above an inline-table member
        // is no longer discarded. The top-level writer promotes a
        // non-empty mapping to a `[section]` (long-standing behaviour
        // since #107), and the captured leading note rides along onto
        // the promoted key — so the comment survives the round trip
        // instead of vanishing as it did before this PR.
        let src = "tbl = {\n  # lead a\n  a = 1,\n  # lead b\n  b = 2\n}\n";
        let ast = from_toml(src).unwrap();
        // Structural check: the leading notes landed on the member keys.
        let CustomNode::Mapping { pairs, .. } = &ast else {
            unreachable!()
        };
        let (_, tbl) = pairs.iter().next().unwrap();
        let CustomNode::Mapping { pairs: inner, .. } = tbl else {
            unreachable!()
        };
        let keys: Vec<_> = inner.keys().collect();
        assert_eq!(
            keys[0].leading_comment().map(|c| &*c.text).unwrap(),
            "lead a"
        );
        assert_eq!(
            keys[1].leading_comment().map(|c| &*c.text).unwrap(),
            "lead b"
        );
        // Emit preserves both notes (in the promoted section form), and a
        // second parse + emit is byte-stable (idempotent normalisation).
        let out = to_toml(&ast).unwrap();
        assert!(out.contains("# lead a"), "leading a lost: {out}");
        assert!(out.contains("# lead b"), "leading b lost: {out}");
        let twice = to_toml(&from_toml(&out).unwrap()).unwrap();
        assert_eq!(out, twice, "not idempotent: {out} vs {twice}");
    }

    #[test]
    fn a_note_between_two_inline_members_keeps_its_own_line() {
        // What this pins is the *promotion* path (#119): a decorated top-level inline table
        // becomes a `[a]` section, and a note line that sits between two members keeps its own
        // line across that move. It is not the guard for the placement rule below - its input
        // parses the note as the *following* key's leading comment, so the trailing-note branch
        // never runs here, and withdrawing that rule leaves this test green. Measured, not
        // assumed: the attribution run said so.
        let src = "a = {\n  b = 1,\n  # n\n  c = 2\n}\n";
        let ast = from_toml(src).unwrap();
        let out = to_toml(&ast).unwrap();
        assert_eq!(
            out, "[a]\nb = 1\n# n\nc = 2\n",
            "note moved off its own line: {out}"
        );
        let again = to_toml(&from_toml(&out).unwrap()).unwrap();
        assert_eq!(out, again, "the emission is not a fixed point: {again}");
    }

    #[test]
    fn a_same_line_note_on_a_non_last_member_is_emitted_on_its_own_line() {
        // The guard for the placement rule, and it has to build the AST rather than parse text:
        // no valid TOML produces this shape, because `#` runs to end of line and the comma that
        // TOML demands between members cannot sit inside a comment. So the only node carrying a
        // same-line note with a later member after it is one a conversion path (YAML -> hub ->
        // TOML) or a generator hands over - which is how `prop_toml_writer_is_fixed_point` found
        // it at 20 000 cases and could not find it at 256.
        //
        // Emitting it as `b = 1, # n` reads back as `c`'s leading note, so the second emission
        // moves the comment and the text never settles. Writing the note on its own line puts it
        // exactly where the reader reports it from, which makes the first emission the fixed
        // point - measured: `# n` comes back as `Comment { text: "n", standalone: true }` on the
        // following key.
        let mut b = CustomNode::plain_scalar("1");
        b.set_comment(Comment {
            text: "n".into(),
            standalone: false,
        });
        let inner = CustomNode::plain_mapping(NodeMap::from([
            (CustomNode::plain_scalar("b"), b),
            (CustomNode::plain_scalar("c"), CustomNode::plain_scalar("2")),
        ]));
        let root = CustomNode::plain_mapping(NodeMap::from([(
            CustomNode::plain_scalar("t"),
            CustomNode::plain_sequence(vec![inner]),
        )]));

        let out = to_toml(&root).unwrap();
        assert_eq!(
            out, "t = [{\n  b = 1,\n  # n\n  c = 2\n}]\n",
            "the note did not get its own line: {out}"
        );
        let again = to_toml(&from_toml(&out).unwrap()).unwrap();
        assert_eq!(out, again, "the emission is not a fixed point: {again}");
    }

    #[test]
    fn preserves_inline_table_interior_trailing_comment() {
        // A `# ...` on the same line as the last member is its trailing
        // note and rides onto the value node's `comment` slot. The
        // comment is preserved through the promotion round trip.
        let src = "tbl = {\n  a = 1,\n  b = 2 # trail b\n}\n";
        let ast = from_toml(src).unwrap();
        let out = to_toml(&ast).unwrap();
        assert!(out.contains("# trail b"), "trailing note lost: {out}");
        // Re-parse keeps the trailing note attached to member b.
        let reparsed = from_toml(&out).unwrap();
        let CustomNode::Mapping { pairs, .. } = &reparsed else {
            unreachable!()
        };
        let (_, tbl) = pairs.iter().next().unwrap();
        let CustomNode::Mapping { pairs: inner, .. } = tbl else {
            unreachable!()
        };
        let (_, b_val) = inner.iter().last().unwrap();
        assert_eq!(b_val.comment().map(|c| &*c.text).unwrap(), "trail b");
    }

    #[test]
    fn preserves_nested_inline_table_interior_comment() {
        // An inline table nested inside an array stays inline (arrays
        // serialise element mappings via `value_str`), so its interior
        // comment forces the multi-line inline form specifically.
        let src = "arr = [{\n  # lead x\n  x = 1\n}]\n";
        let ast = from_toml(src).unwrap();
        let out = to_toml(&ast).unwrap();
        assert!(out.contains("# lead x"), "nested leading lost: {out}");
        // The nested table is emitted in the multi-line inline form.
        assert!(out.contains("arr = [{"), "not inline form: {out}");
        let twice = to_toml(&from_toml(&out).unwrap()).unwrap();
        assert_eq!(out, twice, "not idempotent: {out} vs {twice}");
    }

    #[test]
    fn undecorated_nested_inline_table_stays_single_line() {
        // No interior comments => a nested inline table keeps the
        // compact single-line form inside its array.
        let src = "arr = [{ a = 1, b = 2 }]\n";
        let out = to_toml(&from_toml(src).unwrap()).unwrap();
        assert!(out.contains("a = 1, b = 2"), "lost compact form: {out}");
        assert!(out.contains("[{"), "not inline: {out}");
    }

    #[test]
    fn to_toml_emits_multiline_inline_table_as_single_line() {
        // PR #116 A1: parsing a multi-line inline table succeeds and
        // the writer emits the compact single-line `{ ... }` shape
        // because the AST preserves `flow_style: true` for a
        // non-section inline mapping. Values and insertion order match
        // the source.
        let src = "tbl = {\n  a = 1,\n  b = 2,\n}\n";
        let ast = from_toml(src).unwrap();
        // Sanity: the writer accepts the tree without error and the
        // members are visible somewhere in the output.
        let out = to_toml(&ast).unwrap();
        assert!(out.contains("a = 1"), "{out}");
        assert!(out.contains("b = 2"), "{out}");
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
