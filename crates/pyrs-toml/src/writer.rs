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
use pyrs_ast::ast::{CustomNode, NodeMap, ScalarStyle};
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
        write_note(&mut out, &c.text);
    }
    // A note the hub put on the root's inline slot has no `key = value` line to
    // ride, so it gets a line of its own — which is also where a TOML reader
    // reports a document-leading comment back from, keeping the round trip stable.
    if let Some(c) = node.comment().filter(|c| !c.standalone) {
        write_note(&mut out, &c.text);
    }
    let mut sections: Vec<(&CustomNode, &CustomNode)> = Vec::new();
    // TOML puts every root key/value before any root table header, so a table the tree places
    // ahead of a later pair cannot keep its header form without moving that pair's key ahead of it
    // - which is what the round trip did to the mapping's order (proptest
    // `prop_toml_roundtrip_preserves_the_tree` at 20 000 cases: `0` holding a table and then `a`
    // came back as `a`, `0`). Such a table is written inline, at its own position. A document
    // parsed from TOML never reaches this, because header text already lists its pairs first.
    let is_section = |v: &CustomNode| {
        matches!(v, CustomNode::Mapping { pairs, flow_style, .. }
            if !pairs.is_empty() && !flow_style)
    };
    let last_pair = pairs.iter().rposition(|(_, v)| !is_section(v));
    let mut first_pair = true;
    let mut unscanned = true;
    for (idx, (k, v)) in pairs.iter().enumerate() {
        if unscanned && !k.is_identity_clean() {
            unscanned = false;
            reject_colliding_texts(pairs)?;
        }
        match v {
            CustomNode::Mapping {
                pairs: inner,
                flow_style,
                ..
            } if !inner.is_empty() && !flow_style => {
                // A section header is emitted after every top-level pair, so the key and its table
                // are held on to rather than written here. The name is *not* materialised: the
                // header arms below append the key through `push_scalar_key`, the same writer every
                // pair line uses, so this is the one place the section pass used to allocate.
                //
                // `flow_style` is the source's own answer to "is this a section?": a reader that saw
                // `p = { a = 1 }` marks the mapping flow, and only a reader that saw `[p]` leaves it
                // block. Ignoring that bit rewrote every root-level inline table as a header, which
                // is not a style change but a shape change - the TOML grammar makes a header's plain
                // keys precede it, so siblings moved (`p = { a = 1 } # t` + `q = 2` came back as
                // `q = 2` then `[p]`), the note re-homed from the pair line to the header line, and
                // one TOML round through the hub was not stable. Nested and array-position inline
                // tables already kept their form, because they reach `emit_pair`.
                if last_pair.is_some_and(|later| later > idx) {
                    if !first_pair && v.blank_before() {
                        out.push('\n');
                    }
                    first_pair = false;
                    emit_pair(&mut out, k, v)?;
                } else {
                    sections.push((k, v));
                }
            }
            other => {
                // Design Q2: never emit a blank line before the very first
                // pair of a document; the leading-newline hint only
                // becomes visual for subsequent pairs.
                if !first_pair && other.blank_before() {
                    out.push('\n');
                }
                first_pair = false;
                emit_pair(&mut out, k, other)?;
            }
        }
    }
    for (idx, (key_node, tbl)) in sections.iter().enumerate() {
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
            key_node.leading_comments()
        };
        // Blank line before a section separator acts like the KV rule:
        // skip it for the first section if any top-level pairs already
        // appeared, since the section break is already visual.
        if tbl.blank_before() && !(idx == 0 && out.is_empty()) {
            out.push('\n');
        }
        for c in leading {
            write_note(&mut out, &c.text);
        }
        if leading.is_empty() {
            // Pre-#114 shape: the note rides the parent key's `comment` field.
            if let Some(c) = key_node.comment().filter(|c| c.standalone) {
                write_note(&mut out, &c.text);
            }
        }
        // A TOML-origin node keeps the header's inline note on the section mapping; a YAML-origin one
        // hands it over on the KEY node's inline slot, because in `sec: # note` the note ends the
        // key's line. Reading only the mapping's slot dropped the note outright - the same defect
        // #146 fixed for an inline member, one level up.
        let header_inline = tbl
            .comment()
            .filter(|c| !c.standalone)
            .or_else(|| key_node.comment().filter(|c| !c.standalone));
        match header_inline {
            Some(c) if !c.standalone => {
                out.push('[');
                push_scalar_key(&mut out, key_node)?;
                out.push_str("] # ");
                push_note_text(&mut out, &c.text);
                out.push('\n');
            }
            _ => {
                out.push('[');
                push_scalar_key(&mut out, key_node)?;
                out.push_str("]\n");
            }
        }
        let CustomNode::Mapping { pairs, .. } = tbl else {
            unreachable!("sections collected are mappings");
        };
        let mut unscanned = true;
        for (i, (k, v)) in pairs.iter().enumerate() {
            if unscanned && !k.is_identity_clean() {
                unscanned = false;
                reject_colliding_texts(pairs)?;
            }
            if i > 0 && v.blank_before() {
                out.push('\n');
            }
            emit_pair(&mut out, k, v)?;
        }
    }
    Ok(out)
}

/// The text one TOML comment carrier can hold on its line: trimmed, and with every line break written
/// as a space. A TOML comment ends at the break and the reader refuses a bare carriage return outright,
/// so `# a\r!` is not TOML at all — the tail would reach the reader as content, or the writer's own
/// output would be rejected. Pushed rather than returned, so a text with no break — every note a TOML
/// reader can produce — reaches `out` without a temporary.
fn push_note_text(out: &mut String, text: &str) {
    let trimmed = text.trim();
    // Byte-wise, as in the YAML serializer: a byte equal to `\n` or `\r` can only be that ASCII
    // character in UTF-8, so the search is exact without decoding.
    if trimmed
        .as_bytes()
        .iter()
        .any(|b| *b == b'\n' || *b == b'\r')
    {
        out.push_str(&trimmed.replace(['\n', '\r'], " "));
    } else {
        out.push_str(trimmed);
    }
}

/// One standalone `# note` line. Pushed rather than `writeln!` because the document emitter writes
/// one of these per comment line and `String as fmt::Write` costs a `Formatter` and a `write_str`
/// call each time. Byte-identical to the macro it replaces.
fn write_note(out: &mut String, text: &str) {
    out.push_str("# ");
    push_note_text(out, text);
    out.push('\n');
}

/// Emit one `key = value` line together with any leading (standalone) or
/// trailing (inline) comments attached via `NodeMeta::comment`.
fn emit_pair(
    out: &mut String,
    key_node: &CustomNode,
    value_node: &CustomNode,
) -> Result<(), SerializeError> {
    // PR #114: the native parser puts leading notes into the dedicated
    // `leading_comments` list. The pre-#114 shape (a standalone note in
    // `comment`) still renders, so documents produced by the YAML path
    // or hand-built fixtures keep working.
    for c in key_node.leading_comments() {
        write_note(out, &c.text);
    }
    if key_node.leading_comments().is_empty()
        && let Some(c) = key_node.comment().filter(|c| c.standalone)
    {
        write_note(out, &c.text);
    }
    push_scalar_key(out, key_node)?;
    out.push_str(" = ");
    write_value(out, value_node)?;
    // A TOML-origin note on this line is reported on the VALUE node, which is what this used to read.
    // A YAML key-line note (`key: # note`) arrives on the KEY node's inline slot - the same blind slot
    // the section header had, one level into the body.
    let inline = value_node
        .comment()
        .filter(|c| !c.standalone)
        .or_else(|| key_node.comment().filter(|c| !c.standalone));
    if let Some(c) = inline {
        out.push_str(" # ");
        push_note_text(out, &c.text);
    }
    out.push('\n');
    Ok(())
}

/// Append a key's TOML spelling to `out`.
fn push_scalar_key(out: &mut String, k: &CustomNode) -> Result<(), SerializeError> {
    match k {
        CustomNode::Scalar { value, .. } => {
            push_key(out, value);
            Ok(())
        }
        _ => Err(SerializeError::UnsupportedValue(
            "toml-keys-must-be-scalars",
        )),
    }
}

/// Refuse a mapping whose keys are two keys in YAML but one key in TOML.
///
/// TOML spells a key by its text and nothing else, so `c: 0` beside `!A c: "!"` - a document the
/// reader accepts, because a tag is part of a key's identity in YAML - would emit `c = 0` and
/// `c = "!"`: text this crate's own reader rejects as a duplicate key. Emitting it is not a style
/// loss but an invalid document, so the pair is refused where it is written, exactly as null and
/// alias values already are.
///
/// This is the rare path. The emission loops ask [`CustomNode::is_identity_clean`] of the keys they
/// are already walking and reach here only once one of them is decorated - an undecorated `IndexMap`
/// already distinguishes its keys by exactly the text TOML writes. Clearing a whole mapping with a
/// second pass over its keys instead cost +5.27% on `to_toml_medium`, which is the floor's job.
fn reject_colliding_texts(pairs: &NodeMap<CustomNode, CustomNode>) -> Result<(), SerializeError> {
    let mut seen: Vec<&str> = Vec::with_capacity(pairs.len());
    for key in pairs.keys() {
        if let CustomNode::Scalar { value, .. } = key {
            if seen.contains(&value.as_ref()) {
                return Err(SerializeError::UnsupportedValue(
                    "toml-keys-collide-without-metadata",
                ));
            }
            seen.push(value.as_ref());
        }
    }
    Ok(())
}

/// Whether an array element owns a note the TOML spelling has to show.
///
/// `has_notes` first: it answers from two inline discriminants and never dereferences the boxed
/// decor, which is the whole point for the common case of an undecorated array - the compact form
/// then costs one such test per element instead of two view lookups.
fn element_has_note(item: &CustomNode) -> bool {
    item.has_notes()
        && (!item.leading_comments().is_empty() || item.comment().is_some_and(|c| !c.standalone))
}

/// Append a noted array as one element per line.
///
/// Its own fn because it is the rare shape of a hot branch: [`write_value`] already carries the
/// scalar, inline-table and compact-array arms, and a forty-line note layout does not belong in the
/// one recursive writer every node passes through.
fn write_notated_array(out: &mut String, items: &[CustomNode]) -> Result<(), SerializeError> {
    out.push_str("[\n");
    for (i, it) in items.iter().enumerate() {
        for c in it.leading_comments() {
            out.push_str("  # ");
            push_note_text(out, &c.text);
            out.push('\n');
        }
        out.push_str("  ");
        write_value(out, it)?;
        // The last element's line is the only one a note may finish: `1 # x,` is not TOML at all,
        // and a note written after the comma is reported by the reader as the *next* element's
        // leading note - so that placement moved the comment on every round and the text never
        // settled. Anywhere but last, the note gets its own line after the element, which is where
        // the reader hands it to the next one.
        match it.comment().filter(|c| !c.standalone) {
            Some(c) if i + 1 == items.len() => {
                out.push_str(" # ");
                push_note_text(out, &c.text);
            }
            Some(c) => {
                out.push_str(",\n  # ");
                push_note_text(out, &c.text);
            }
            None => {
                if i + 1 != items.len() {
                    out.push(',');
                }
            }
        }
        out.push('\n');
    }
    out.push(']');
    Ok(())
}

/// Append a node's TOML spelling to `out`.
///
/// Writing into the caller's buffer rather than returning a `String` is the second half of the
/// pair-emission change: every key and every value used to materialise a temporary of its own, and
/// an array or inline table nested a `Vec` of those on top of it before joining and copying the
/// result again. The allocation, not the scan, is what cost - the same lesson `write_basic_body`
/// records.
fn write_value(out: &mut String, node: &CustomNode) -> Result<(), SerializeError> {
    match node {
        CustomNode::Scalar {
            value, style, meta, ..
        } => {
            // Tagged timestamps round-trip through strict RFC 3339 validation
            // so a malformed text never silently becomes a TOML string.
            if meta.tag().is_some_and(|t| {
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
                        out.push_str(value);
                        return Ok(());
                    }
                    _ => return Err(SerializeError::UnsupportedValue("toml-timestamp-malformed")),
                }
            }
            let tag_kind = meta
                .tag()
                .and_then(|t| pyrs_schema::schema::standard_tag_kind(&t.handle, &t.suffix));
            // A standard tag states the type, so it decides before the text is consulted - the
            // loader has honoured it since #335 and this bridge reading only the text made
            // `!!bool yes` and `!!str 1.20` arrive in TOML as something other than what the
            // document said. A tag the text cannot satisfy is refused, not re-typed.
            let resolved = match tag_kind {
                Some(kind) => kind
                    .resolve(value)
                    .ok_or(SerializeError::UnsupportedValue("toml-tag-text-mismatch"))?,
                None => Schema::Core.resolve(value),
            };
            match (style, resolved) {
                (ScalarStyle::Plain, YamlType::Null) => Err(SerializeError::UnsupportedValue(
                    "toml-cannot-represent-null",
                )),
                (ScalarStyle::Plain, YamlType::Bool(b)) => {
                    out.push_str(if b { "true" } else { "false" });
                    Ok(())
                }
                // Fidelity pass-through: when the plain scalar's textual
                // form is already a valid TOML integer literal (`0xDEAD`,
                // plain decimal), emit it verbatim. Otherwise fall
                // back to the canonical `i64` rendering.
                //
                // A signed zero is the exception inside the literal set: TOML has no negative zero,
                // so `a = -0` re-reads as the integer `0` and the second emission moved
                // (libFuzzer `edit_yaml` crash-917e103b). Radix spellings survive a read because the
                // reader keeps their text; a redundant sign on zero is not one of them, and a fixed
                // point has to be spelled the way the reader spells it.
                // A tagged integer is an integer whatever quotes the YAML emitter
                // needed: the projection tags `-1_000` because Core cannot read it,
                // and a leading `-` makes the emitter quote the text. An untagged
                // quoted scalar stays a string - quoting is how YAML says "this is
                // text", so `"42"` must not become an int here.
                (_, YamlType::Int(i))
                    if tag_kind.is_some() || matches!(style, ScalarStyle::Plain) =>
                {
                    if is_toml_int_literal(value) {
                        out.push_str(value);
                    } else {
                        let _ = write!(out, "{i}");
                    }
                    Ok(())
                }
                // Same rule for floats: an exponent spelling (`1e10`) or
                // an `inf`/`nan` form the source produced is passed
                // through; everything else is normalised via
                // `fmt_toml_float`.
                (ScalarStyle::Plain, YamlType::Float(f)) => {
                    if is_toml_float_literal(value) {
                        out.push_str(value);
                    } else {
                        out.push_str(&fmt_toml_float(f));
                    }
                    Ok(())
                }
                // PR #132: a multi-line string (projected as a YAML literal
                // block) is re-emitted as a TOML `"""` basic block so the
                // multi-line shape survives the hub instead of degrading to
                // an escaped single line. The block value already carries its
                // trailing newlines, so reconstruction needs only the value.
                (ScalarStyle::Literal, _) => {
                    push_multiline(out, value);
                    Ok(())
                }
                // Everything else is text: quoted scalars verbatim, plain
                // strings resolved-to-Str.
                _ => {
                    push_quoted(out, value);
                    Ok(())
                }
            }
        }
        CustomNode::Sequence { items, .. } => {
            // An element carrying a note forces the array onto several lines, for the reason #119
            // recorded for inline tables: `#` runs to end of line, so a comment cannot live inside
            // `[1, 2]`. Undecorated arrays keep the compact form, so 1.0 output is unchanged.
            if items.iter().any(element_has_note) {
                return write_notated_array(out, items);
            }
            out.push('[');
            for (i, it) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                write_value(out, it)?;
            }
            out.push(']');
            Ok(())
        }
        CustomNode::Mapping { pairs, .. } => {
            let mut unscanned = true;
            // PR #119: an inline table whose members carry interior
            // comments (a key leading note or a value trailing note)
            // must be rendered multi-line, because a `# ...` comment
            // runs to end-of-line and cannot live inside a single-line
            // `{ a = 1, b = 2 }`. Undecorated inline tables keep the
            // compact single-line form so 1.0-compatible output is
            // unchanged.
            let decorated = pairs.iter().any(|(k, v)| {
                !k.leading_comments().is_empty()
                    || k.comment().is_some_and(|c| !c.standalone)
                    || v.comment().is_some_and(|c| !c.standalone)
            });
            if decorated {
                out.push_str("{\n");
                let count = pairs.len();
                for (idx, (k, v)) in pairs.iter().enumerate() {
                    if unscanned && !k.is_identity_clean() {
                        unscanned = false;
                        reject_colliding_texts(pairs)?;
                    }
                    for c in k.leading_comments() {
                        out.push_str("  # ");
                        push_note_text(out, &c.text);
                        out.push('\n');
                    }
                    // A note that ended its key's line reaches this writer on the key's *inline*
                    // slot - that is how the hub hands it over, because there a note finishing the
                    // key's line belongs to the key. TOML has no slot that reports a key's inline
                    // note back, so the note takes its own line above the member: the position the
                    // TOML reader reports a note from. Measured, the second emission is identical.
                    if let Some(c) = k.comment().filter(|c| !c.standalone) {
                        out.push_str("  # ");
                        push_note_text(out, &c.text);
                        out.push('\n');
                    }
                    let last = idx + 1 == count;
                    out.push_str("  ");
                    push_scalar_key(out, k)?;
                    out.push_str(" = ");
                    write_value(out, v)?;
                    // A `#` runs to end of line, so the only member line a note can
                    // legitimately finish is the document's last one: `a = 1 # n,` is not
                    // TOML at all (measured: the reader rejects it), and writing the note
                    // *after* the comma - which is what this branch used to do - hands it to
                    // the next key on re-read, so the next emission moved it and the text
                    // never settled (proptest `prop_toml_writer_is_fixed_point`, 20 000
                    // cases). Anywhere but last, the note gets its own line after the
                    // member, which is the position the reader reports it from.
                    match v.comment().filter(|c| !c.standalone) {
                        Some(c) if last => {
                            out.push_str(" # ");
                            push_note_text(out, &c.text);
                        }
                        Some(c) => {
                            out.push_str(",\n  # ");
                            push_note_text(out, &c.text);
                        }
                        None => {
                            if !last {
                                out.push(',');
                            }
                        }
                    }
                    out.push('\n');
                }
                out.push('}');
            } else {
                out.push('{');
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if unscanned && !k.is_identity_clean() {
                        unscanned = false;
                        reject_colliding_texts(pairs)?;
                    }
                    if i > 0 {
                        out.push_str(", ");
                    }
                    push_scalar_key(out, k)?;
                    out.push_str(" = ");
                    write_value(out, v)?;
                }
                out.push('}');
            }
            Ok(())
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

/// Append a key to `out`: bare when every byte allows it, quoted otherwise. The byte scan and the
/// character scan agree because a multi-byte sequence is made of bytes >= 0x80, none of which is
/// alphanumeric, `_` or `-`.
fn push_key(out: &mut String, s: &str) {
    let bare = !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    if bare {
        out.push_str(s);
    } else {
        push_quoted(out, s);
    }
}

/// Bytes a single-line basic string may not contain raw: the C0 codes and DEL (which
/// `parser::forbidden_control` refuses on the way in as well), plus the two characters that would
/// close or corrupt the string. Everything else - including every byte of a multi-byte UTF-8 sequence
/// - is legal literal content, so one byte scan decides whether the whole value can be copied at once.
const fn forbidden_single_line(b: u8) -> bool {
    matches!(b, 0x00..=0x1f | 0x7f | b'"' | b'\\')
}

/// The same for a multi-line basic string, where a tab and a newline are legal content. The ranges
/// leave exactly those two out: `..=0x08` stops before the tab, and `0x0b..` starts after `\n`.
const fn forbidden_multi_line(b: u8) -> bool {
    matches!(b, 0x00..=0x08 | 0x0b..=0x1f | 0x7f | b'"' | b'\\')
}

/// Index of the first byte at or after `pos` that `forbidden_single_line` rejects, or
/// `bytes.len()` when the run is all literal. The set is all ASCII - a multi-byte sequence has every
/// byte >= 0x80 and can never match - so the answer is an ASCII byte index and therefore a `char`
/// boundary, and the readers' sixteen-byte scan applies here too. SSE2 is part of the x86_64
/// baseline: no dependency, no runtime detection, no `std`.
#[inline]
fn forbidden_start(bytes: &[u8], pos: usize) -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        forbidden_start_sse2(bytes, pos)
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        forbidden_start_scalar(bytes, pos)
    }
}

fn forbidden_start_scalar(bytes: &[u8], mut pos: usize) -> usize {
    while pos < bytes.len() {
        if forbidden_single_line(bytes[pos]) {
            break;
        }
        pos += 1;
    }
    pos
}

/// Sixteen bytes per iteration; the last partial vector falls back to the scalar loop, so a value
/// that ends near a vector boundary keeps the old answer exactly.
#[cfg(target_arch = "x86_64")]
#[inline]
fn forbidden_start_sse2(bytes: &[u8], mut pos: usize) -> usize {
    use core::arch::x86_64::*;
    let len = bytes.len();
    // SAFETY: SSE2 is in the x86_64 baseline, and the loop only loads when `pos + 16 <= len`, so
    // every 16-byte vector lies inside the slice.
    unsafe {
        let quote = _mm_set1_epi8(b'"' as i8);
        let slash = _mm_set1_epi8(b'\\' as i8);
        // DEL is the one stop byte outside the C0 range, so it gets its own equality mask rather
        // than a wider comparison that would also catch the bytes of a multi-byte sequence.
        let del = _mm_set1_epi8(0x7f);
        let ctl = _mm_set1_epi8(0x1f);
        let zero = _mm_setzero_si128();
        while pos + 16 <= len {
            let v = _mm_loadu_si128(bytes.as_ptr().add(pos) as *const __m128i);
            // Saturating subtract makes `v <= 0x1f` the only zero, i.e. `v < 0x20` unsigned.
            let mut m = _mm_or_si128(
                _mm_or_si128(_mm_cmpeq_epi8(v, quote), _mm_cmpeq_epi8(v, slash)),
                _mm_cmpeq_epi8(_mm_subs_epu8(v, ctl), zero),
            );
            m = _mm_or_si128(m, _mm_cmpeq_epi8(v, del));
            let mask = _mm_movemask_epi8(m) as u32;
            if mask != 0 {
                return pos + mask.trailing_zeros() as usize;
            }
            pos += 16;
        }
    }
    forbidden_start_scalar(bytes, pos)
}

/// Append `"s"` as a single-line basic string to `out`.
fn push_quoted(out: &mut String, s: &str) {
    out.push('"');
    write_basic_body(s, out);
    out.push('"');
}

/// Append `s` to `out` as the body of a single-line basic string.
///
/// The bulk branch exists because most values need no escaping at all, and one scan plus one copy
/// beats a loop that encodes a code point at a time. It allocates nothing: an earlier attempt returned
/// `format!("\"{s}\"")` from here, which allocated and copied the whole string again and measured
/// worse than the loop it replaced - the mistake was the allocation, not the scan. The scan itself is
/// the cost on a long value (69.01% of `to_toml_long_strings`, `docs/dev/quality-ledger.md` (fi)), so
/// it runs sixteen bytes at a time and the clean prefix is copied; the char loop below starts at the
/// first forbidden byte and is the answer the scalar scan would have given.
fn write_basic_body(s: &str, out: &mut String) {
    let cut = forbidden_start(s.as_bytes(), 0);
    if cut == s.len() {
        out.push_str(s);
        return;
    }
    out.push_str(&s[..cut]);
    let mut it = s[cut..].chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => {
                // Every newline is escaped, including the rest of a run. A single-line basic string
                // may not contain a raw newline at all: the old branch wrote `\n` once and then the
                // remaining newlines literally, so `"a\n\nb"` came back out as `"a\n<LF>b"` - text
                // that `tomllib` and this crate's own reader both reject (toml-test encoder corpus).
                // The CRLF argument in the removed comment never applied: `\r` has its own arm below,
                // so a run reaching here is nothing but newlines.
                out.push_str("\\n");
                while it.peek() == Some(&'\n') {
                    it.next();
                    out.push_str("\\n");
                }
            }
            '\r' => out.push_str("\\r"),
            '\u{c}' => out.push_str("\\f"),
            // TOML forbids the C0 codes *and* DEL raw in a basic string; `parser::forbidden_control`
            // refuses the same set on the way in, so escaping only `< 0x20` left the writer emitting
            // text this crate's own reader rejects (toml-test encoder corpus, issue-tracked here).
            c if (c as u32) < 0x20 || c == '\u{7f}' => {
                let _ = write!(out, "\\u{:04X}", c as u32);
            }
            c => out.push(c),
        }
    }
}

/// Emit a TOML multi-line basic string (`"""…"""`) whose parsed content is
/// exactly `value` (PR #132). Two TOML rules shape the encoding: a newline
/// immediately after the opening delimiter is trimmed, so a value that starts
/// with a newline gets a spare one emitted first; and every `"` is escaped so a
/// run of quotes can never prematurely close the block. Real newlines and tabs
/// are kept literal; `\r` and other control chars are escaped.
/// Append `value` to `out` as a multi-line basic string.
fn push_multiline(out: &mut String, value: &str) {
    out.push_str("\"\"\"");
    if value.starts_with('\n') {
        out.push('\n');
    }
    if !value.bytes().any(forbidden_multi_line) {
        out.push_str(value);
    } else {
        for c in value.chars() {
            match c {
                '\\' => out.push_str("\\\\"),
                '"' => out.push_str("\\\""),
                '\n' => out.push('\n'),
                '\t' => out.push('\t'),
                '\r' => out.push_str("\\r"),
                c if (c as u32) < 0x20 || c == '\u{7f}' => {
                    // Same set as `push_quoted`: a multi-line basic string is still a basic string, so
                    // DEL is forbidden raw there too.
                    let _ = write!(out, "\\u{:04X}", c as u32);
                }
                c => out.push(c),
            }
        }
    }
    out.push_str("\"\"\"");
}

fn is_valid_toml_datetime(s: &str) -> bool {
    // Reuse the parser's strict validator via a probe. The parser module
    // owns the grammar so date-time rules live in one place.
    crate::parser::DateTimeProbe.check_all(s)
}

/// Whether the plain scalar text is a legal TOML integer literal: decimal
/// with an optional `+`/`-`, or an unsigned `0x` hex / `0o` octal / `0b`
/// binary form, with `_` allowed only between digits. Used by the writer to
/// decide whether to pass through verbatim instead of re-rendering from
/// `i64`. A signed zero is not a literal here: TOML has no negative zero, so
/// `-0`/`+0` re-read as `0` and the fixed point has to be spelled the way
/// the reader spells it.
fn is_toml_int_literal(text: &str) -> bool {
    let bytes = text.as_bytes();
    let (signed, body) = match bytes {
        [b'+' | b'-', rest @ ..] => (true, rest),
        _ => (false, bytes),
    };
    let Some(&first) = body.first() else {
        return false;
    };
    // A radix prefix is unsigned in TOML: `signed-int` only ever wraps a
    // decimal, so `-0x1f` is not an integer.
    if body.len() > 2 && first == b'0' {
        let radix: Option<fn(u8) -> bool> = match body[1] {
            b'x' => Some(|b: u8| b.is_ascii_hexdigit()),
            b'o' => Some(|b: u8| matches!(b, b'0'..=b'7')),
            b'b' => Some(|b: u8| matches!(b, b'0' | b'1')),
            _ => None,
        };
        if let Some(is_digit) = radix {
            return !signed && digits_with_underscores(&body[2..], is_digit);
        }
    }
    // Plain decimal: no leading zero unless the integer part is exactly `0`
    // (`0_1` cleans to `01`, which TOML refuses).
    digits_with_underscores(body, |b| b.is_ascii_digit())
        && (body.len() == 1 || first != b'0')
        && !(signed && body.iter().all(|&b| b == b'0'))
}

/// TOML's `underscore` rule: `_` may separate digits but may not lead,
/// trail or double up, because each `_` must sit between two digits.
fn digits_with_underscores(bytes: &[u8], is_digit: fn(u8) -> bool) -> bool {
    let mut last_was_digit = false;
    for &b in bytes {
        if b == b'_' {
            if !last_was_digit {
                return false;
            }
            last_was_digit = false;
        } else if is_digit(b) {
            last_was_digit = true;
        } else {
            return false;
        }
    }
    last_was_digit
}

/// Whether the plain scalar text is a legal TOML float spelling that we
/// want to preserve verbatim: an exponent form, whose canonical `f64`
/// rendering can drift from source. The whole grammar is checked rather than
/// just the presence of an exponent: the hub can hand this writer a spelling
/// YAML accepts and TOML does not. Core YAML floats allow leading zeros in
/// the integer part (`083e6`), while TOML's `float-int-part` is a `dec-int`;
/// passing it through wrote text the TOML reader rejects (libFuzzer
/// `edit_yaml` crash-71feb9d9). Underscore forms, `.inf`/`nan` keywords and
/// fraction-only spellings fall through to `fmt_toml_float`.
fn is_toml_float_literal(text: &str) -> bool {
    if text.contains('_') {
        return false;
    }
    let bytes = text.as_bytes();
    let mut i = 0;
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        i += 1;
    }
    let int_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    let int = &bytes[int_start..i];
    // TOML has no leading zeros in `float-int-part` unless it is exactly `0`.
    if int.is_empty() || (int.len() > 1 && int[0] == b'0') {
        return false;
    }
    if bytes.get(i) == Some(&b'.') {
        i += 1;
        let frac_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == frac_start {
            return false;
        }
    }
    if !matches!(bytes.get(i), Some(b'e' | b'E')) {
        // A fraction-only spelling would return from the hub as the
        // canonical rendering (the TOML reader keeps a float's source text
        // only for exponent forms), so the first emission is not the fixed
        // point and the export does not settle.
        return false;
    }
    i += 1;
    if matches!(bytes.get(i), Some(b'+' | b'-')) {
        i += 1;
    }
    let exp_start = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    i > exp_start && i == bytes.len()
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

    // The two quotiers asked for as values rather than as appends. Production writes straight into
    // the document buffer - that is the whole point of them - so only the assertions need a String
    // back, and building it here keeps a test-only path out of the shipped code.
    fn quoted(s: &str) -> String {
        let mut out = String::new();
        push_quoted(&mut out, s);
        out
    }

    fn multiline(value: &str) -> String {
        let mut out = String::new();
        push_multiline(&mut out, value);
        out
    }

    /// TOML has no signed zero: `a = -0` is legal text whose value is `0`. The integer pass-through
    /// kept the spelling it was handed, so a tree read from `-0` wrote `-0` again and a tree whose
    /// text came from the hub's `-0` (YAML `-0`, libFuzzer `edit_yaml` crash-917e103b) wrote text that
    /// re-read as `0` and moved on the second emission. A fixed point is spelled the way the reader
    /// spells it, so a signed zero is written as the plain zero.
    #[test]
    fn a_signed_zero_is_written_as_the_zero_the_reader_spells_back() {
        for text in ["a = -0\n", "a = +0\n"] {
            let ast = from_toml(text).expect("a signed zero is legal TOML");
            let once = to_toml(&ast).expect("an integer in a root table writes");
            assert_eq!(once, "a = 0\n", "{text} must be written as it reads back");
            let back = from_toml(&once).expect("the emission re-reads");
            let twice = to_toml(&back).expect("and re-writes");
            assert_eq!(once, twice, "not a fixed point: {once:?} -> {twice:?}");
        }
    }

    #[test]
    fn the_bulk_path_and_the_loop_escape_identically_over_every_byte() {
        // A second implementation, written from the TOML grammar rather than from this module, so the
        // bulk branch and the character loop cannot both be wrong in the same way. All 256 byte-values
        // are walked because that is precisely where the two branches disagree in risk: one table
        // decides which strings skip the loop, and the loop decides what escaping means.
        let reference = |s: &str| {
            let mut out = String::from("\"");
            for c in s.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\u{8}' => out.push_str("\\b"),
                    '\t' => out.push_str("\\t"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\u{c}' => out.push_str("\\f"),
                    c if (c as u32) < 0x20 || c == '\u{7f}' => {
                        out.push_str(&format!("\\u{:04X}", c as u32));
                    }
                    c => out.push(c),
                }
            }
            out.push('"');
            out
        };
        for b in 0u32..=0xff {
            let c = char::from_u32(b).expect("every byte is a Unicode scalar when widened");
            let s = c.to_string();
            assert_eq!(quoted(&s), reference(&s), "byte {b:#04x} escaped wrong");
        }

        // Multi-line: the predicate claims a tab and a newline may stay literal. Check that claim
        // against text that exercises it rather than trusting the table - and check the other side too,
        // because a predicate that is too generous emits raw CR, which no reader accepts.
        let literal = "line1\nline2\ttabbed";
        assert!(
            !literal.bytes().any(forbidden_multi_line),
            "predicate rejects legal literal content"
        );
        assert_eq!(multiline(literal), format!("\"\"\"{literal}\"\"\""));
        let needs_crlf = "a\r\nb";
        assert!(
            needs_crlf.bytes().any(forbidden_multi_line),
            "CR must leave the bulk path"
        );
        // The asymmetry is the point: a multi-line body may keep a newline literal, so only the CR is
        // escaped. (An earlier draft of this assertion demanded an escaped newline too, and was wrong
        // - the code was not.)
        assert_eq!(multiline(needs_crlf), "\"\"\"a\\r\nb\"\"\"");
    }

    /// The scalar text of the single value in a decoded one-key table, for comparing a round trip
    /// against the string it started as.
    fn text_of(node: &pyrs_ast::ast::CustomNode) -> String {
        let pyrs_ast::ast::CustomNode::Mapping { pairs, .. } = node else {
            panic!("expected a table, got {node:?}");
        };
        let (_, value) = pairs.iter().next().expect("one key-value pair");
        let pyrs_ast::ast::CustomNode::Scalar { value, .. } = value else {
            panic!("expected a scalar string, got {value:?}");
        };
        value.to_string()
    }
    use pyrs_ast::ast::Comment;

    // The YAML-source round-trip (`parse -> to_toml -> from_toml`) needs the
    // YAML parser and lives in `pyrs-yaml-core` `src/integration/toml_family.rs`.

    #[test]
    fn an_undecorated_inline_table_writes_compact_and_loads_as_1_0() {
        // The compact form is what the writer owes a table that carries no note, and this is the arm
        // the property tier reaches only when generation happens to draw a `flow_style` mapping
        // without a note on it. So the shapes are named here: a table read from `{…}` text, an empty
        // table, and a table nested in an array — each written back with no interior line break, and
        // each loadable under the 1.0 reader. Forcing the multi-line branch reddens this test, which
        // is what makes the claim a guard rather than a comment.
        for source in [
            "a = { b = 1 }\n",
            "a = { b = 1, c = 2 }\n",
            "a = {}\n",
            "list = [ { b = 1 }, { c = 2 } ]\n",
            "top = { mid = { deep = 1 } }\n",
        ] {
            let tree = crate::from_toml(source).unwrap_or_else(|e| panic!("{source:?}: {e}"));
            let text = crate::to_toml(&tree).unwrap_or_else(|e| panic!("{source:?}: {e}"));
            assert!(
                !text.contains("{\n"),
                "an undecorated table was written multi-line, which is 1.1 syntax:\n{text}"
            );
            crate::from_toml_v1_0(&text).unwrap_or_else(|e| {
                panic!("1.0 refused the writer's own text for {source:?}: {e}\n{text}")
            });
        }
    }

    #[test]
    fn the_1_0_reader_refuses_the_1_1_inline_table_freedom() {
        // The premise of `prop_toml_undecorated_output_loads_as_1_0`: a newline inside an inline table
        // is what TOML 1.1 added (the release lifted the multi-line-inline-table restriction, so a
        // comment can live inside `{…}`), and our own 1.0 reader must refuse it — otherwise the
        // dialect split is only a name and a 1.1-shaped document would load as 1.0 while nothing in
        // the wild could. Both halves are asserted: 1.0 refuses, 1.1 accepts, and the single-line
        // spelling — the one the writer owes an undecorated tree — loads under both.
        let multiline = "a = {\n  b = 1,\n}\n";
        assert!(
            crate::from_toml_v1_0(multiline).is_err(),
            "1.0 accepted a multi-line inline table, which is 1.1 syntax"
        );
        assert!(
            crate::from_toml(multiline).is_ok(),
            "1.1 refused its own syntax: {multiline}"
        );
        let commented = "a = { b = 1 # note\n }\n";
        assert!(
            crate::from_toml_v1_0(commented).is_err(),
            "1.0 accepted a comment inside an inline table"
        );
        for spelling in ["a = { b = 1 }\n", "a = {}\n"] {
            assert!(
                crate::from_toml_v1_0(spelling).is_ok(),
                "1.0 refused the compact inline table {spelling:?}"
            );
            assert!(
                crate::from_toml(spelling).is_ok(),
                "1.1 refused the compact inline table {spelling:?}"
            );
        }
    }

    #[test]
    fn the_two_quoting_forms_agree_with_their_readers() {
        // Both writers must produce text that reads back as the value it started with, for inputs on
        // both sides of every escaping boundary: quote, backslash, tab, newline, a run of newlines,
        // CR, DEL, a C0 code, non-ASCII text that must survive as UTF-8, and empty. The corpus gate
        // checks the same property against `tomllib`; this checks it per form, where a failure names
        // the value and the spelling instead of a percentage.
        for value in [
            "plain",
            "a b",
            "with \"quote\"",
            "back\\slash",
            "tab\there",
            "nl\nhere",
            "run\n\n\nof them",
            "del\u{7f}here",
            "caf\u{e9}",
            "\u{1}",
            "cr\rhere",
            "mixed \"q\"\ttab\u{7f}",
            "",
        ] {
            let one = quoted(value);
            let back = crate::from_toml(&format!("k = {one}\n"))
                .unwrap_or_else(|e| panic!("{value:?} -> {one:?} rejected: {e}"));
            assert_eq!(
                text_of(&back),
                value,
                "basic round trip broke for {value:?}: {one:?}"
            );
            let multi = multiline(value);
            let back = crate::from_toml(&format!("k = {multi}\n"))
                .unwrap_or_else(|e| panic!("{value:?} -> {multi:?} rejected: {e}"));
            assert_eq!(
                text_of(&back),
                value,
                "multi-line round trip broke for {value:?}: {multi:?}"
            );
        }
    }

    #[test]
    fn a_run_of_newlines_stays_escaped_in_a_single_line_string() {
        // toml-test `valid/string/hex-escape`: the basic-string writer escaped the first newline
        // and then emitted the rest of the run as *raw* newlines, which a single-line basic string
        // may not contain - `tomllib` reports `Illegal character '\n'`. The collapse was written for
        // the line-ending-backslash trick, which only exists inside a multi-line string.
        for src in [
            "k = \"a\\nb\"\n",
            "k = \"a\\n\\nb\"\n",
            "k = \"a\\n\\n\\nb\"\n",
            "k = \"a\\n\\r\\nb\"\n",
        ] {
            let ast = from_toml(src).unwrap_or_else(|e| panic!("fixture must parse {src:?}: {e}"));
            let text = to_toml(&ast).expect("serializes");
            let body = text
                .strip_prefix("k = \"")
                .and_then(|t| t.strip_suffix("\"\n"))
                .unwrap_or_else(|| panic!("expected a single-line basic string: {text:?}"));
            assert!(
                !body.contains('\n'),
                "raw newline inside a single-line string: {text:?}"
            );
            assert!(from_toml(&text).is_ok(), "output must re-parse: {text:?}");
        }
    }

    #[test]
    fn a_del_in_a_basic_string_is_escaped_because_toml_forbids_it_raw() {
        // toml-test's encoder corpus surfaced this: `to_toml` wrote `"x\u{7f}y"` with DEL raw, and
        // TOML forbids DEL in every string and comment form - the rule `parser::forbidden_control`
        // states and enforces. So the writer emitted text that this crate's own reader rejects,
        // let alone a reference one.
        let ast = from_toml("k = \"x\\u007Fy\"\n").expect("fixture parses");
        let text = to_toml(&ast).expect("serializes");
        assert!(!text.contains('\u{7f}'), "raw DEL emitted: {text:?}");
        assert_eq!(
            text, "k = \"x\\u007Fy\"\n",
            "the escaped spelling must be stable"
        );
        assert!(from_toml(&text).is_ok(), "output must re-parse: {text:?}");
    }

    #[test]
    fn a_del_in_a_multiline_string_is_escaped_too() {
        // Same rule, other form: a multi-line basic string may not hold a raw DEL either, so the
        // two escaping paths cannot be fixed independently.
        let ast = from_toml("k = \"\"\"a\\u007Fb\"\"\"\n").expect("fixture parses");
        let text = to_toml(&ast).expect("serializes");
        assert!(!text.contains('\u{7f}'), "raw DEL emitted: {text:?}");
        let again = from_toml(&text).expect("output re-parses");
        assert_eq!(
            to_toml(&again).expect("re-serializes"),
            text,
            "not a fixed point"
        );
    }

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
    fn to_toml_refuses_keys_that_become_one_key_without_the_tag() {
        // The shape `fmt_pbt::prop_toml_writer_is_fixed_point` drew at random on 2026-10-27 (seed
        // `16b4a97ca3115ecd9db10190e1011152ad03ac4adc071ec35866819a861a72dd`): `c` beside `!A c`,
        // which the reader keeps apart because a tag is part of a key's identity in YAML. TOML has
        // no spelling for that tag, so the pair used to go out as `c = 0` and `c = "!"` - text this
        // crate's own reader rejects, which is why the writer is asked to refuse instead.
        let mut tagged = CustomNode::plain_scalar("c");
        tagged.set_tag(pyrs_ast::ast::Tag::local("A"));
        let mut pairs = pyrs_ast::ast::NodeMap::default();
        pairs.insert(CustomNode::plain_scalar("c"), CustomNode::plain_scalar("0"));
        pairs.insert(tagged, CustomNode::plain_scalar("!"));
        assert_eq!(
            to_toml(&CustomNode::plain_mapping(pairs))
                .unwrap_err()
                .to_string(),
            "toml-keys-collide-without-metadata"
        );

        // Two clean keys that differ are not the writer's business: the gate that keeps the common
        // path cheap must not become a refusal of documents that were never at risk.
        let mut ok = pyrs_ast::ast::NodeMap::default();
        ok.insert(CustomNode::plain_scalar("c"), CustomNode::plain_scalar("0"));
        ok.insert(CustomNode::plain_scalar("d"), CustomNode::plain_scalar("1"));
        assert_eq!(
            to_toml(&CustomNode::plain_mapping(ok)).unwrap(),
            "c = 0\nd = 1\n"
        );

        // And a tagged key whose text is unique still writes: the refusal is about the collision,
        // not about tags, which the emitter elsewhere carries through the projection on purpose.
        let mut alone = pyrs_ast::ast::NodeMap::default();
        let mut keyed = CustomNode::plain_scalar("c");
        keyed.set_tag(pyrs_ast::ast::Tag::local("A"));
        alone.insert(keyed, CustomNode::plain_scalar("0"));
        alone.insert(CustomNode::plain_scalar("d"), CustomNode::plain_scalar("1"));
        assert_eq!(
            to_toml(&CustomNode::plain_mapping(alone)).unwrap(),
            "c = 0\nd = 1\n"
        );
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

    /// The tree's order outranks a section header's placement. TOML admits no root key/value after
    /// a root table header, so a table the tree lists ahead of a later pair cannot keep the header
    /// form without that pair's key moving ahead of it - which is what the round trip used to do:
    /// `0` holding a table and then `a` came back as `a`, `0`
    /// (`prop_toml_roundtrip_preserves_the_tree`, 20 000 generated cases). Such a table goes out
    /// inline, at its own position. Every order TOML can spell with headers - which is every order
    /// a parsed document has - still gets the form above.
    #[test]
    fn a_section_before_a_pair_keeps_the_order_by_going_inline() {
        let mut table = pyrs_ast::ast::NodeMap::default();
        table.insert(CustomNode::plain_scalar("A"), CustomNode::plain_scalar("a"));
        let mut pairs = pyrs_ast::ast::NodeMap::default();
        pairs.insert(
            CustomNode::plain_scalar("0"),
            CustomNode::Mapping {
                pairs: table,
                flow_style: false,
                meta: pyrs_ast::ast::NodeMeta::default(),
            },
        );
        pairs.insert(CustomNode::plain_scalar("a"), CustomNode::plain_scalar("b"));
        let node = CustomNode::Mapping {
            pairs,
            flow_style: false,
            meta: pyrs_ast::ast::NodeMeta::default(),
        };

        let text = to_toml(&node).expect("the tree writes");
        assert_eq!(text, "0 = {A = \"a\"}\na = \"b\"\n", "{text:?}");
        let back = from_toml(&text).expect("the text reads");
        let CustomNode::Mapping { pairs: keys, .. } = &back else {
            panic!("a table root reads back as a mapping");
        };
        let names: Vec<String> = keys
            .iter()
            .map(|(k, _)| match k {
                CustomNode::Scalar { value, .. } => value.to_string(),
                _ => panic!("a scalar key was expected"),
            })
            .collect();
        assert_eq!(names, ["0", "a"], "the key order survives: {text:?}");
        assert_eq!(
            to_toml(&back).expect("writes twice"),
            text,
            "not a fixed point"
        );
    }

    #[test]
    fn preserves_radix_integer_source_spelling() {
        // Fidelity #108: radix integers round-trip with their original prefix +
        // case intact, binary included — `0b` rides the hub under a `!!int` tag,
        // which every writer has honoured since (bu). Underscore separators are a
        // legal TOML spelling between digits and are kept too.
        for (src, expected) in [
            ("v = 0xDEADBEEF\n", "v = 0xDEADBEEF\n"),
            ("v = 0o755\n", "v = 0o755\n"),
            ("v = -31\n", "v = -31\n"),
            ("v = 0xFF_FF\n", "v = 0xFF_FF\n"),
            ("v = 0o1_7\n", "v = 0o1_7\n"),
            ("v = 0b101\n", "v = 0b101\n"),
            ("v = 0b1101_0110\n", "v = 0b1101_0110\n"),
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
    fn preserves_decimal_underscore_separators_and_plus_signs() {
        // Both are legal TOML spellings (twelve months of `boundaries.md`
        // claimed otherwise). The parser now records the token text, so the
        // writer's literal gate has to accept the same grammar it emits.
        for src in [
            "a = 1_000\n",
            "a = +42\n",
            "a = -1_000\n",
            "a = 5_349_221\n",
            "a = +9_223_372_036_854_775_807\n",
        ] {
            let ast = from_toml(src).unwrap();
            let text = to_toml(&ast).unwrap();
            assert_eq!(text, src, "decimal fidelity lost on {src:?}");
            assert_eq!(
                to_toml(&from_toml(&text).unwrap()).unwrap(),
                text,
                "not a fixed point on {src:?}"
            );
        }
    }

    #[test]
    fn a_quoted_integer_stays_a_string_unless_a_tag_says_otherwise() {
        let pair = |node: CustomNode| {
            CustomNode::plain_mapping(
                [(CustomNode::plain_scalar("a"), node)]
                    .into_iter()
                    .collect(),
            )
        };
        // Untagged quoted `42` is YAML's way of saying "text", so it stays one.
        assert_eq!(
            to_toml(&pair(CustomNode::quoted_scalar("42"))).unwrap(),
            "a = \"42\"\n"
        );
        // The projection tags `-1_000` because Core cannot read it, and the
        // leading `-` makes the YAML emitter quote the text. The tag is
        // authoritative: the TOML writer must still emit an integer.
        let mut tagged = CustomNode::quoted_scalar("-1_000");
        tagged.set_tag(pyrs_ast::ast::Tag {
            handle: "!!".to_string(),
            suffix: "int".to_string(),
        });
        assert_eq!(to_toml(&pair(tagged)).unwrap(), "a = -1_000\n");
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
    fn the_writer_refuses_the_time_ranges_the_reader_refuses() {
        // The reader policed the time-of-day ranges while the probe policed only their shape, so an
        // AST holding `24:00:00` or an offset of `+24:00` - text no TOML reader takes back - could
        // still be emitted. Both halves now carry the same bounds, which is what the corpus files
        // `invalid/datetime/offset-overflow-hour` and `offset-overflow-minute` describe. A legal
        // document is parsed first and only then re-written, because the reader would refuse these
        // spellings outright.
        for (seed, spelling) in [
            ("t = 17:45:00\n", "24:00:00"),
            ("t = 17:45:00\n", "17:60:00"),
            ("t = 17:45:00\n", "17:45:61"),
            (
                "d = 2020-01-02T03:04:05+09:00\n",
                "2020-01-02T03:04:05+24:00",
            ),
            (
                "d = 2020-01-02T03:04:05+09:00\n",
                "2020-01-02T03:04:05+09:60",
            ),
        ] {
            let mut doc = from_toml(seed).unwrap();
            let CustomNode::Mapping { pairs, .. } = &mut doc else {
                panic!("the seed document is not a table");
            };
            for (_, value) in pairs.iter_mut() {
                let CustomNode::Scalar { value: text, .. } = value else {
                    panic!("the seed value is not a scalar");
                };
                *text = alloc::sync::Arc::from(spelling);
            }
            assert!(to_toml(&doc).is_err(), "wrote {spelling}");
        }
    }

    #[test]
    fn a_root_level_inline_table_keeps_its_form_and_its_siblings_order() {
        // Measured before this was fixed: `[p] # t\na = 1\n` for the first case, and
        // `q = 2\n[p]\na = 1\n` for the second. The root classification looked only at "is this
        // value a non-empty mapping", so a source inline table was promoted to a section header -
        // which moves its note off the pair line and, because the grammar wants a table's plain
        // keys before its sub-table headers, pushes a sibling key ahead of it.
        let out = to_toml(&from_toml("p = { a = 1 } # t\n").unwrap()).unwrap();
        assert_eq!(
            out, "p = {a = 1} # t\n",
            "the note left the pair line: {out}"
        );

        // The padding inside the braces is normalised on the first emission (`{ a = 1 }` writes
        // `{a = 1}`), which is spelling, not shape; what used to be lost is the shape itself.
        let src = "p = { a = 1 }\nq = 2\n";
        let out = to_toml(&from_toml(src).unwrap()).unwrap();
        assert_eq!(out, "p = {a = 1}\nq = 2\n", "a sibling key moved: {out}");
        let again = to_toml(&from_toml(&out).unwrap()).unwrap();
        assert_eq!(out, again, "not a fixed point: {again}");

        // The control: a source that reads as a header still writes as one, so this is the reader's
        // `flow_style` bit doing the work and not the writer preferring one spelling.
        let src = "[p]\na = 1\n";
        assert_eq!(to_toml(&from_toml(src).unwrap()).unwrap(), src);
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
        // Emit preserves both notes, in the inline form the source used, and a
        // second parse + emit is byte-stable (idempotent normalisation).
        let out = to_toml(&ast).unwrap();
        assert!(out.contains("# lead a"), "leading a lost: {out}");
        assert!(out.contains("# lead b"), "leading b lost: {out}");
        let twice = to_toml(&from_toml(&out).unwrap()).unwrap();
        assert_eq!(out, twice, "not idempotent: {out} vs {twice}");
    }

    #[test]
    fn a_note_between_two_inline_members_keeps_its_own_line() {
        // What this pins is that a note line sitting between two members of a decorated top-level
        // inline table keeps its own line, and that the emission is a fixed point. It is not the
        // guard for the placement rule below - its input parses the note as the *following* key's
        // leading comment, so the trailing-note branch never runs here, and withdrawing that rule
        // leaves this test green. Measured, not assumed: the attribution run said so.
        //
        // It used to pin a `[a]` section as well, because the root classification ignored
        // `flow_style` and promoted every decorated top-level inline table. That rewrite is gone,
        // so the strongest form holds instead: the text is its own first emission.
        let src = "a = {\n  b = 1,\n  # n\n  c = 2\n}\n";
        let ast = from_toml(src).unwrap();
        let out = to_toml(&ast).unwrap();
        assert_eq!(out, src, "note moved off its own line: {out}");
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
        let inner = CustomNode::plain_mapping(
            [
                (CustomNode::plain_scalar("b"), b),
                (CustomNode::plain_scalar("c"), CustomNode::plain_scalar("2")),
            ]
            .into_iter()
            .collect(),
        );
        let root = CustomNode::plain_mapping(
            [(
                CustomNode::plain_scalar("t"),
                CustomNode::plain_sequence(vec![inner]),
            )]
            .into_iter()
            .collect(),
        );

        let out = to_toml(&root).unwrap();
        assert_eq!(
            out, "t = [{\n  b = 1,\n  # n\n  c = 2\n}]\n",
            "the note did not get its own line: {out}"
        );
        let again = to_toml(&from_toml(&out).unwrap()).unwrap();
        assert_eq!(out, again, "the emission is not a fixed point: {again}");
    }

    #[test]
    fn a_note_on_the_key_line_of_an_inline_member_is_written() {
        // The shape the hub hands over, measured: `arr = [{\n  # n\n  a = 1\n}]` goes to YAML as
        // `arr:\n  - {a: # n\n     1}` because a note that ends the key's line belongs to the key
        // there, and comes back with `n` on the KEY node's inline slot. That slot is the one this
        // writer never read - it looked at the key's leading list and the value's inline slot - so
        // the note vanished on the second hop: `arr = [{a = 1}]`.
        //
        // A note's own line above the member is where the TOML reader reports it from, so emitting
        // it there settles in one hop even though the slot it arrives in has no TOML counterpart.
        let mut key = CustomNode::plain_scalar("a");
        key.set_comment(Comment {
            text: "n".into(),
            standalone: false,
        });
        let inner =
            CustomNode::plain_mapping([(key, CustomNode::plain_scalar("1"))].into_iter().collect());
        let root = CustomNode::plain_mapping(
            [(
                CustomNode::plain_scalar("t"),
                CustomNode::plain_sequence(vec![inner]),
            )]
            .into_iter()
            .collect(),
        );

        let out = to_toml(&root).unwrap();
        assert_eq!(
            out, "t = [{\n  # n\n  a = 1\n}]\n",
            "the note did not survive: {out}"
        );
        let again = to_toml(&from_toml(&out).unwrap()).unwrap();
        assert_eq!(out, again, "the emission is not a fixed point: {again}");
    }

    #[test]
    fn a_note_on_the_key_line_of_a_section_header_is_written_on_the_header() {
        // The same slot #146 made the writer read for an inline member, one level up: the hub hands
        // over `sec: # note` with `n` on the KEY node's inline comment, and the section pass looked
        // only at the mapping's own slot - so `to_toml` returned `[sec]\nx = 1\n` and the note was
        // gone, not moved. A TOML-origin document keeps the note on the mapping instead (the shape
        // `cow_table_to_node` builds), which is why the header line already had a writer for it.
        let mut key = CustomNode::plain_scalar("sec");
        key.set_comment(Comment {
            text: "n".into(),
            standalone: false,
        });
        let section = CustomNode::plain_mapping(
            [(CustomNode::plain_scalar("x"), CustomNode::plain_scalar("1"))]
                .into_iter()
                .collect(),
        );
        let root = CustomNode::plain_mapping([(key, section)].into_iter().collect());

        let out = to_toml(&root).unwrap();
        assert_eq!(out, "[sec] # n\nx = 1\n", "the note was dropped: {out}");
        // One TOML round lands where the reader reports the note from, so the emission settles.
        let again = to_toml(&from_toml(&out).unwrap()).unwrap();
        assert_eq!(out, again, "the emission is not a fixed point: {again}");
    }

    #[test]
    fn a_note_on_the_key_line_of_a_section_body_entry_is_written() {
        // The header line got its second slot; the entry line inside the body reads the same node
        // shape and had the same blind read. The hub hands `a:\n  b: # n\n    x: 1` over with `n` on
        // b's KEY node, while `emit_pair` looked only at the value's slot - so to_toml returned
        // "[a]\nb = {x = 1}\n" and the note was dropped. A TOML-origin note on that line is reported
        // on the value (`b = {x = 1} # n`), which is also where the written text comes back from, so
        // the value's slot keeps priority and the key node is the fallback.
        let mut bkey = CustomNode::plain_scalar("b");
        bkey.set_comment(Comment {
            text: "n".into(),
            standalone: false,
        });
        let inner = CustomNode::plain_mapping(
            [(CustomNode::plain_scalar("x"), CustomNode::plain_scalar("1"))]
                .into_iter()
                .collect(),
        );
        let section = CustomNode::plain_mapping([(bkey, inner)].into_iter().collect());
        let root = CustomNode::plain_mapping(
            [(CustomNode::plain_scalar("a"), section)]
                .into_iter()
                .collect(),
        );

        let out = to_toml(&root).unwrap();
        assert_eq!(out, "[a]\nb = {x = 1} # n\n", "the note was dropped: {out}");
        let again = to_toml(&from_toml(&out).unwrap()).unwrap();
        assert_eq!(out, again, "the emission is not a fixed point: {again}");
    }

    #[test]
    fn an_array_element_note_takes_its_own_line_and_settles() {
        // The three places a note can sit on a multi-line array, measured: before an element, at
        // the end of the last element's line, and between an element's comma and the next element.
        // The middle case is the one the reader reports as the *following* element's leading note,
        // which is where the writer puts it, so the second emission is the same text.
        for (src, want) in [
            ("arr = [\n  # lead\n  1\n]\n", "arr = [\n  # lead\n  1\n]\n"),
            ("arr = [\n  1 # trail\n]\n", "arr = [\n  1 # trail\n]\n"),
            (
                "arr = [\n  1, # between\n  2\n]\n",
                "arr = [\n  1,\n  # between\n  2\n]\n",
            ),
        ] {
            let out = to_toml(&from_toml(src).unwrap()).unwrap();
            assert_eq!(out, want, "the note did not come back for {src:?}");
            let again = to_toml(&from_toml(&out).unwrap()).unwrap();
            assert_eq!(out, again, "not a fixed point: {out:?} -> {again:?}");
        }
    }

    #[test]
    fn an_array_without_notes_keeps_the_compact_form() {
        // The control for the rule above: nothing decorated, nothing reformatted.
        let out = to_toml(&from_toml("arr = [1, 2]\n").unwrap()).unwrap();
        assert_eq!(out, "arr = [1, 2]\n");
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
        // the source. The assertion used to be two `contains` checks, which the
        // old root-level promotion to `[tbl]` satisfied; the shape is what this
        // test is about, so the shape is what it pins.
        let src = "tbl = {\n  a = 1,\n  b = 2,\n}\n";
        let ast = from_toml(src).unwrap();
        let out = to_toml(&ast).unwrap();
        assert_eq!(out, "tbl = {a = 1, b = 2}\n", "not the inline form: {out}");
    }

    /// The same structural question the JSON family asks (`writer.rs::
    /// every_dialect_reads_back_what_it_writes_for_the_notation_corpus`), for the other
    /// source-fidelity writer in the matrix: a plain scalar carrying a notation TOML cannot spell
    /// must be normalised or refused, never emitted as text `from_toml` rejects.
    /// The prose promise both writer branches make — an undecorated inline table stays inside the
    /// 1.0 grammar, and a note is the only thing that takes it out — checked rather than asserted,
    /// because `fuzz/fuzz_targets/parse_toml.rs` now splits its oracle on exactly this.
    #[test]
    fn the_undecorated_1_0_promise_holds_and_a_note_is_what_leaves_it() {
        let plain = crate::from_toml("inli.inli.0bt = \"n\"\n").unwrap();
        let out = to_toml(&plain).unwrap();
        assert_eq!(out, "[inli]\ninli = {0bt = \"n\"}\n");
        crate::from_toml_v1_0(&out)
            .expect("an undecorated inline table must stay 1.0-loadable: {out}");

        // The same document with a note on the member: `#` runs to end of line, so the table has
        // to break, and 1.0 forbids the break — the trade #119 made for comment survival.
        let noted = crate::from_toml("inli.inli.0bt = \"n\" # e\n").unwrap();
        let out = to_toml(&noted).unwrap();
        assert!(
            out.contains("{\n"),
            "the note needs a line of its own: {out}"
        );
        crate::from_toml(&out).expect("the 1.1 reader is the gate a noted inline table answers to");
        assert!(
            crate::from_toml_v1_0(&out).is_err(),
            "1.1 grammar is what the note buys: {out}"
        );
    }

    #[test]
    fn to_toml_reads_back_what_it_writes_for_the_notation_corpus() {
        for spelling in [
            "0",
            "-0",
            "01",
            "001",
            "+7",
            "1_000",
            "0o17",
            "0x1F",
            "0b10",
            "1e3",
            "1.2e+3",
            ".5",
            "5.",
            "2024-01-01",
            "1:2",
            "~",
            "TRUE",
            "",
        ] {
            let root = CustomNode::plain_mapping(
                [(
                    CustomNode::plain_scalar("a"),
                    CustomNode::plain_scalar(spelling),
                )]
                .into_iter()
                .collect(),
            );
            let Ok(text) = to_toml(&root) else { continue };
            assert!(
                crate::from_toml(&text).is_ok(),
                "TOML wrote {text:?} for the spelling {spelling:?}, which its own reader refuses"
            );
        }
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

    /// The vector scan decides where the escaping loop starts, so a stop set differing by one byte
    /// would change the emitted text rather than crash. Every byte value is asked for at every
    /// offset of a tail that crosses the sixteen-byte boundary, against an inline restatement of
    /// `forbidden_single_line`.
    #[test]
    fn the_body_scan_stops_where_the_predicate_stops_at_every_offset() {
        let mut corpus: Vec<u8> = (0..=255u8).collect();
        corpus.extend([0xE2, 0x82, 0xAC]);
        for start in 0..corpus.len() {
            let tail = &corpus[start..];
            let expected = tail
                .iter()
                .position(|b| forbidden_single_line(*b))
                .unwrap_or(tail.len());
            assert_eq!(forbidden_start(tail, 0), expected, "offset {start}");
        }
    }

    /// The bulk prefix copy must write exactly what the whole-string char loop wrote before it, at
    /// the lengths where a vector boundary falls inside the value - including DEL, which the C0
    /// range alone would have missed, and a run of newlines, which the loop collapses.
    #[test]
    fn a_bulk_copied_basic_body_writes_what_the_char_loop_would_have_written() {
        fn body_as_loop_would(s: &str) -> String {
            let mut out = String::new();
            let mut it = s.chars().peekable();
            while let Some(c) = it.next() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\u{8}' => out.push_str("\\b"),
                    '\t' => out.push_str("\\t"),
                    '\n' => {
                        out.push_str("\\n");
                        while it.peek() == Some(&'\n') {
                            it.next();
                            out.push_str("\\n");
                        }
                    }
                    '\r' => out.push_str("\\r"),
                    '\u{c}' => out.push_str("\\f"),
                    c if (c as u32) < 0x20 || c == '\u{7f}' => {
                        out.push_str(&format!("\\u{:04X}", c as u32));
                    }
                    c => out.push(c),
                }
            }
            out
        }
        for length in [0usize, 1, 15, 16, 17, 31, 33, 256] {
            for stop in ['"', '\\', '\n', '\r', '\t', '\u{0}', '\u{7f}', 'é'] {
                for at in 0..=length {
                    let text = format!("{}{}{}", "a".repeat(at), stop, "a".repeat(length - at));
                    let mut got = String::new();
                    write_basic_body(&text, &mut got);
                    assert_eq!(got, body_as_loop_would(&text), "length {length} at {at}");
                }
            }
            // A newline run straddling the boundary exercises the collapsing arm.
            for at in 14..18 {
                let text = format!("{}{}", "a".repeat(at), "\n\n\n");
                let mut got = String::new();
                write_basic_body(&text, &mut got);
                assert_eq!(got, body_as_loop_would(&text), "run at {at}");
            }
        }
    }

    /// The dispatch decides by `target_arch`, so on an x86 CI the scalar loop only ever sees the
    /// last partial vector - yet that loop is the whole answer the aarch64 wheels ship. Ask both the
    /// same questions over a body that crosses the boundary, with every byte value at every offset
    /// where a vector starts, ends and straddles it, and require the same index from each.
    #[test]
    #[cfg(target_arch = "x86_64")]
    fn the_scalar_body_scan_answers_what_the_vector_answers() {
        fn agree(bytes: &[u8], what: &str) {
            for pos in [0usize, 1, 7, 15, 16, 17, 31, 33, 63, 64, 79, 80] {
                let pos = pos.min(bytes.len());
                assert_eq!(
                    forbidden_start_scalar(bytes, pos),
                    forbidden_start_sse2(bytes, pos),
                    "{what} from {pos}: {bytes:?}"
                );
            }
        }
        // 'a' is content for a single-line basic string, so a stop is only ever caused by the byte
        // placed - the tab and the newline included, which the writer must not skip over.
        let mut body = vec![b'a'; 80];
        agree(&body, "unbroken run");
        for offset in [0usize, 1, 7, 14, 15, 16, 17, 31, 32, 47, 63, 64, 79] {
            for value in 0u8..=255 {
                body[offset] = value;
                agree(&body, &format!("byte {value:#04x} at {offset}"));
                body[offset] = b'a';
            }
        }
        for offset in 0..=77 {
            body[offset] = 0xE2;
            body[offset + 1] = 0x82;
            body[offset + 2] = 0xAC;
            agree(&body, &format!("multi-byte at {offset}"));
            body[offset..offset + 3].fill(b'a');
        }
    }
}
