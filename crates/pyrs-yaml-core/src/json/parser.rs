//! Native JSON engine - an RFC 8259-conformant parser and serializer built
//! in the `granit-parser` house style: a byte-level scanner with exact
//! positions, errors carrying line/column diagnostics, and direct
//! `CustomNode` construction (no intermediate value tree).
//!
//! Strictness choices (documented divergence or improvement over the
//! previous `serde_json`-based path):
//! - numbers keep their source spelling (`1e3`, `1.0`, `-0`) instead of
//!   f64 round-tripping, so `from-json | to-json` is byte-stable and no
//!   precision is lost on large doubles;
//! - lone UTF-16 surrogates are rejected (RFC 8259 "may reject"; a Rust
//!   `String` cannot represent them losslessly);
//! - duplicate object keys follow the JSON guidance: last value wins,
//!   first insertion order position kept (`IndexMap` semantics);
//! - top-level scalars are valid (RFC 8259), trailing commas are not,
//!   and a document must contain exactly one value.

use crate::ast::CustomNode;
use crate::error::{DepthError, ParseError};
use crate::parser::yaml::schema::needs_quotes;
use indexmap::IndexMap;

/// Default nesting limit, matching the YAML pipeline's `parse` default.
pub const DEFAULT_MAX_DEPTH: usize = 1000;

/// Dialect knobs for [`from_json_with_options`].
///
/// The default [`JsonParseOptions::STRICT`] implements RFC 8259 exactly:
/// no comments, no trailing commas, one top-level value. [`allow_comments`]
/// upgrades the parser to JSONC (the dialect popularised by TypeScript's
/// `tsconfig.json` and VS Code's `settings.json`), accepting `// line`
/// and `/* block */` comments wherever whitespace is legal. Trailing
/// commas remain rejected so the accepted language is still a strict
/// superset of JSON, not JSON5.
///
/// [`allow_comments`]: JsonParseOptions::allow_comments
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonParseOptions {
    pub allow_comments: bool,
    pub max_depth: usize,
}

impl JsonParseOptions {
    /// RFC 8259 with the default depth budget. `from_json` maps to this.
    pub const STRICT: JsonParseOptions = JsonParseOptions {
        allow_comments: false,
        max_depth: DEFAULT_MAX_DEPTH,
    };
    /// JSONC: allows `//` and `/* ... */` comments. Everything else strict.
    pub const JSONC: JsonParseOptions = JsonParseOptions {
        allow_comments: true,
        max_depth: DEFAULT_MAX_DEPTH,
    };
}

/// Parse one JSON document into the shared AST.
pub fn from_json(text: &str) -> Result<CustomNode, ParseError> {
    from_json_with_options(text, JsonParseOptions::STRICT)
}

/// Parse one JSONC document (JSON with comments) into the shared AST.
/// Comments are stripped, not preserved on the AST.
pub fn from_jsonc(text: &str) -> Result<CustomNode, ParseError> {
    from_json_with_options(text, JsonParseOptions::JSONC)
}

/// Parse with an explicit nesting limit (`MaxDepthExceeded` beyond it).
///
/// Kept as a distinct entry point because it predates the options struct;
/// new code should prefer [`from_json_with_options`].
pub fn from_json_with_max_depth(text: &str, max_depth: usize) -> Result<CustomNode, ParseError> {
    from_json_with_options(
        text,
        JsonParseOptions {
            allow_comments: false,
            max_depth,
        },
    )
}

/// Parse with a chosen dialect. See [`JsonParseOptions`] for the axes.
pub fn from_json_with_options(
    text: &str,
    opts: JsonParseOptions,
) -> Result<CustomNode, ParseError> {
    let mut p = Parser {
        s: text.as_bytes(),
        text,
        pos: 0,
        depth: 0,
        max_depth: opts.max_depth,
        allow_comments: opts.allow_comments,
        pending_comment: None,
    };
    p.ws();
    let value = p.value()?;
    p.ws();
    if p.pos != p.s.len() {
        return Err(p.err("trailing characters after the JSON value"));
    }
    Ok(value)
}

struct Parser<'a> {
    s: &'a [u8],
    text: &'a str,
    pos: usize,
    depth: usize,
    max_depth: usize,
    allow_comments: bool,
    /// The most recent JSONC comment consumed by `ws()`, waiting to
    /// be attached to the next constructed node. `own_line` records
    /// whether the comment started on a line of its own (i.e. no
    /// non-whitespace token on the current line before it), which
    /// maps directly onto `Comment::standalone`. Only populated when
    /// `allow_comments` is on.
    pending_comment: Option<PendingComment>,
}

/// Intermediate comment record the parser threads between `ws()`
/// (which consumes the comment bytes) and the value / key sites that
/// attach it to a `CustomNode`.
struct PendingComment {
    text: String,
    own_line: bool,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.s.get(self.pos).copied()
    }

    /// 0-indexed line/column fields (granit convention) plus a human
    /// 1-indexed suffix in the message body.
    fn err(&self, msg: &str) -> ParseError {
        let clamped = self.pos.min(self.text.len());
        // `self.pos` is a byte offset; snap to a char boundary so a mid-
        // multi-byte stop never panics on slicing or counting.
        let cut = floor_char_boundary(self.text, clamped);
        let consumed = &self.text[..cut];
        let line = consumed.matches('\n').count();
        let col = match consumed.rfind('\n') {
            Some(nl) => consumed[nl + 1..].chars().count(),
            None => consumed.chars().count(),
        };
        ParseError::Syntax {
            message: format!("{msg} at line {} column {}", line + 1, col + 1),
            line,
            col,
        }
    }

    fn ws(&mut self) {
        // A newline seen at the top of a `ws()` sequence marks any
        // following comment as `own_line` unless we cross another
        // non-whitespace token before reaching it. That is the JSONC
        // analogue of the TOML model's standalone vs trailing split.
        let mut own_line_seen = false;
        loop {
            match self.peek() {
                Some(b' ' | b'\t') => self.pos += 1,
                Some(b'\n' | b'\r') => {
                    own_line_seen = true;
                    self.pos += 1;
                }
                Some(b'/') if self.allow_comments => {
                    let line = self.s.get(self.pos + 1) == Some(&b'/');
                    let block = self.s.get(self.pos + 1) == Some(&b'*');
                    if !line && !block {
                        return;
                    }
                    let body_start = self.pos + 2;
                    if line {
                        self.pos = body_start;
                        while !self.eof_pos()
                            && self.peek() != Some(b'\n')
                            && self.peek() != Some(b'\r')
                        {
                            self.pos += 1;
                        }
                        let body_end = self.pos;
                        self.pending_comment = Some(PendingComment {
                            text: self.text[body_start..body_end].trim().to_string(),
                            own_line: own_line_seen,
                        });
                    } else {
                        self.pos = body_start;
                        loop {
                            if self.pos + 1 >= self.s.len() {
                                // Unterminated block comment: bail out so
                                // the outer parser falls through to the
                                // existing "trailing characters" error.
                                return;
                            }
                            if self.s[self.pos] == b'*' && self.s[self.pos + 1] == b'/' {
                                break;
                            }
                            self.pos += 1;
                        }
                        let body_end = self.pos;
                        self.pos += 2;
                        self.pending_comment = Some(PendingComment {
                            text: self.text[body_start..body_end].trim().to_string(),
                            own_line: own_line_seen,
                        });
                        // A block comment does not consume the trailing
                        // newline; the next `ws()` iteration will pick it
                        // up and reset `own_line_seen` accordingly.
                    }
                }
                _ => return,
            }
        }
    }

    /// Attach the pending comment (if any) to `node` and reset the slot.
    fn flush_pending(&mut self, node: &mut CustomNode) {
        if let Some(pc) = self.pending_comment.take() {
            node.set_comment(crate::ast::Comment {
                text: std::sync::Arc::from(pc.text),
                standalone: pc.own_line,
            });
        }
    }

    fn eof_pos(&self) -> bool {
        self.pos >= self.s.len()
    }

    fn expect(&mut self, lit: &str, ctx: &str) -> Result<(), ParseError> {
        if self.text[self.pos..].starts_with(lit) {
            self.pos += lit.len();
            Ok(())
        } else {
            Err(self.err(ctx))
        }
    }

    fn value(&mut self) -> Result<CustomNode, ParseError> {
        self.depth += 1;
        if self.depth > self.max_depth {
            self.depth -= 1;
            return Err(ParseError::MaxDepthExceeded(DepthError(self.max_depth)));
        }
        let out = self.value_inner();
        self.depth -= 1;
        out
    }

    fn value_inner(&mut self) -> Result<CustomNode, ParseError> {
        match self.peek() {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => {
                let s = self.string()?;
                Ok(quoted_or_plain(s))
            }
            Some(b't') => {
                self.expect("true", "expected literal `true`")?;
                Ok(CustomNode::plain_scalar("true"))
            }
            Some(b'f') => {
                self.expect("false", "expected literal `false`")?;
                Ok(CustomNode::plain_scalar("false"))
            }
            Some(b'n') => {
                self.expect("null", "expected literal `null`")?;
                Ok(CustomNode::plain_null())
            }
            Some(b'-') | Some(b'0'..=b'9') => self.number(),
            _ => Err(self.err("expected a JSON value")),
        }
    }

    fn object(&mut self) -> Result<CustomNode, ParseError> {
        self.pos += 1; // '{'
        let mut pairs: IndexMap<CustomNode, CustomNode> = IndexMap::new();
        self.ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(CustomNode::plain_mapping(pairs));
        }
        loop {
            self.ws();
            // Any comment consumed by the leading `ws()` is the key's
            // standalone note; claim the slot before parsing the key so
            // the value-side `ws()` below starts with a clean slate.
            let key_pending = self.pending_comment.take();
            if self.peek() != Some(b'"') {
                return Err(self.err("expected a quoted object key"));
            }
            let key = self.string()?;
            let mut key_node = quoted_or_plain(key);
            if let Some(pc) = key_pending {
                key_node.set_comment(crate::ast::Comment {
                    text: std::sync::Arc::from(pc.text),
                    standalone: pc.own_line,
                });
            }
            self.ws();
            self.expect(":", "expected `:` after the object key")?;
            self.ws();
            let mut value = self.value()?;
            self.ws();
            self.flush_pending(&mut value);
            // duplicate keys: last value wins, first position kept
            pairs.insert(key_node, value);
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(CustomNode::plain_mapping(pairs));
                }
                _ => return Err(self.err("expected `,` or `}` in object")),
            }
        }
    }

    fn array(&mut self) -> Result<CustomNode, ParseError> {
        self.pos += 1; // '['
        let mut items: Vec<CustomNode> = Vec::new();
        self.ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(CustomNode::plain_sequence(items));
        }
        loop {
            self.ws();
            // A standalone comment between `,` (or `[`) and the element
            // is the element's leading annotation; claim the slot so
            // the post-value flush below starts clean.
            let element_pending = self.pending_comment.take();
            let mut item = self.value()?;
            if let Some(pc) = element_pending {
                item.set_comment(crate::ast::Comment {
                    text: std::sync::Arc::from(pc.text),
                    standalone: pc.own_line,
                });
            }
            self.ws();
            self.flush_pending(&mut item);
            items.push(item);
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(CustomNode::plain_sequence(items));
                }
                _ => return Err(self.err("expected `,` or `]` in array")),
            }
        }
    }

    /// `-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?` - grammar-exact.
    /// The matched slice is kept verbatim as a plain scalar (no f64 round
    /// trip), which preserves arbitrary-precision integer and float text.
    fn number(&mut self) -> Result<CustomNode, ParseError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        match self.peek() {
            Some(b'0') => {
                self.pos += 1;
                if matches!(self.peek(), Some(b'0'..=b'9')) {
                    return Err(self.err("leading zero in number"));
                }
            }
            Some(b'1'..=b'9') => {
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.pos += 1;
                }
            }
            _ => return Err(self.err("expected integer digits in number")),
        }
        if self.peek() == Some(b'.') {
            self.pos += 1;
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.err("expected digits after `.` in number"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.err("expected digits in number exponent"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
        }
        Ok(CustomNode::plain_scalar(
            self.text[start..self.pos].to_string(),
        ))
    }

    /// Decode a quoted string; returns its content.
    fn string(&mut self) -> Result<String, ParseError> {
        debug_assert_eq!(self.peek(), Some(b'"'));
        self.pos += 1; // skip opening quote
        let mut out = String::new();
        loop {
            // Bulk-scan literal characters until we hit a special byte.
            let start = self.pos;
            while self.pos < self.s.len() {
                let b = self.s[self.pos];
                if b == b'"' || b == b'\\' || b < 0x20 {
                    break;
                }
                self.pos += 1;
            }
            if self.pos > start {
                // SAFETY: we only advanced over bytes that are not ASCII < 0x20,
                // not " or \ — so UTF-8 sequences are preserved intact.
                out.push_str(&self.text[start..self.pos]);
            }
            // Handle the stopping character.
            let Some(b) = self.peek() else {
                return Err(self.err("unterminated string"));
            };
            match b {
                b'"' => {
                    self.pos += 1;
                    return Ok(out);
                }
                b'\\' => {
                    self.pos += 1;
                    let e = self.peek().ok_or_else(|| self.err("unterminated escape"))?;
                    self.pos += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => self.unicode_escape(&mut out)?,
                        _ => return Err(self.err("invalid escape sequence")),
                    }
                }
                0x00..=0x1f => {
                    return Err(
                        self.err(&format!("unescaped control character U+{b:04X} in string"))
                    );
                }
                _ => unreachable!(),
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, ParseError> {
        if self.pos + 4 > self.s.len() {
            return Err(self.err("truncated \\u escape"));
        }
        let digits = &self.text[self.pos..self.pos + 4];
        let value = u32::from_str_radix(digits, 16)
            .map_err(|_| self.err("invalid hex digits in \\u escape"))?;
        self.pos += 4;
        Ok(value)
    }

    fn unicode_escape(&mut self, out: &mut String) -> Result<(), ParseError> {
        let code = self.hex4()?;
        match code {
            0xD800..=0xDBFF => {
                // high surrogate must pair with a following \uDC00-\uDFFF
                if self.peek() != Some(b'\\') || self.s.get(self.pos + 1) != Some(&b'u') {
                    return Err(self.err("lone high surrogate (missing low surrogate pair)"));
                }
                self.pos += 2;
                let low = self.hex4()?;
                if !(0xDC00..=0xDFFF).contains(&low) {
                    return Err(
                        self.err("invalid surrogate pair (second escape is not a low surrogate)")
                    );
                }
                let combined = 0x1_0000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                out.push(
                    char::from_u32(combined)
                        .ok_or_else(|| self.err("invalid combined surrogate value"))?,
                );
                Ok(())
            }
            0xDC00..=0xDFFF => Err(self.err("lone low surrogate")),
            other => {
                let c = char::from_u32(other)
                    .ok_or_else(|| self.err("invalid \\u escape (not a Unicode scalar value)"))?;
                out.push(c);
                Ok(())
            }
        }
    }
}

/// JSON strings land in the YAML-shaped AST without ever re-resolving:
/// text that a plain YAML scalar would reinterpret is quoted (the same
/// `needs_quotes` discipline the TOML spoke follows).
fn quoted_or_plain(value: String) -> CustomNode {
    if needs_quotes(&value) {
        CustomNode::double_quoted_scalar(value)
    } else {
        CustomNode::plain_scalar(value)
    }
}

/// Stable equivalent of `str::floor_char_boundary` (unstable on `str`).
fn floor_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    let mut i = index;
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::ScalarStyle;
    use crate::parser::yaml::Schema;

    fn scalar_text(node: &CustomNode) -> String {
        match node {
            CustomNode::Scalar { value, .. } => value.to_string(),
            other => panic!("expected scalar, got {other:?}"),
        }
    }

    #[test]
    fn parses_all_value_kinds() {
        let n = from_json(r#"{"a":[1, -2.5, true, false, null, "s"], "e":{}, "f":[]}"#).unwrap();
        let CustomNode::Mapping { pairs, .. } = &n else {
            panic!("object")
        };
        assert_eq!(pairs.len(), 3);
        let (_, root_v) = pairs.get_index(0).unwrap();
        let CustomNode::Sequence { items, .. } = root_v else {
            unreachable!()
        };
        assert_eq!(items.len(), 6);
        assert_eq!(scalar_text(&items[0]), "1");
        assert_eq!(scalar_text(&items[1]), "-2.5");
        assert_eq!(scalar_text(&items[2]), "true");
        assert!(matches!(items[4], CustomNode::Null { .. }));
        assert_eq!(scalar_text(&items[5]), "s");
    }

    #[test]
    fn string_styling_prevents_reinterpretation() {
        let n = from_json(r#"{"k": ["plain", "123", "true", ""]}"#).unwrap();
        let CustomNode::Mapping { pairs, .. } = &n else {
            unreachable!()
        };
        let (_, root_v) = pairs.get_index(0).unwrap();
        let CustomNode::Sequence { items, .. } = root_v else {
            unreachable!()
        };
        assert!(matches!(
            &items[0],
            CustomNode::Scalar {
                style: ScalarStyle::Plain,
                ..
            }
        ));
        // "123" and "true" must come back out as strings, never re-typed
        for item in &items[1..3] {
            assert!(matches!(
                item,
                CustomNode::Scalar {
                    style: ScalarStyle::DoubleQuoted,
                    ..
                }
            ));
        }
        assert!(matches!(
            &items[3],
            CustomNode::Scalar {
                style: ScalarStyle::DoubleQuoted,
                value,
                ..
            } if value.is_empty()
        ));
    }

    #[test]
    fn numbers_keep_source_spelling() {
        for text in [
            "1e3",
            "1.0",
            "-0",
            "0E0",
            "1234567890123456789012345678901234567890123456789",
        ] {
            let n = from_json(&format!("{{\"v\": {text}}}")).unwrap();
            let CustomNode::Mapping { pairs, .. } = &n else {
                unreachable!()
            };
            let (_, v) = pairs.get_index(0).unwrap();
            assert_eq!(scalar_text(v), text);
        }
    }

    #[test]
    fn number_grammar_is_exact() {
        for bad in [
            "01", "+1", ".5", "1.", "1e", "1e+", "-", "1.5e2.5", "00", "0x1", "1_000",
        ] {
            assert!(
                from_json(bad).is_err(),
                "must reject {bad:?} (with trailing-char or grammar error)"
            );
        }
        // single valid check to keep the failure message useful
        from_json("0").unwrap();
        from_json("-0.5e-3").unwrap();
    }

    #[test]
    fn surrogate_pairs_combine_and_lone_surrogates_reject() {
        let n = from_json(r#""😀""#).unwrap();
        assert_eq!(scalar_text(&n), "😀");
        let n = from_json(r#""\ud83d\ude00""#).unwrap();
        assert_eq!(scalar_text(&n), "😀");
        for bad in [r#""\ud83d""#, r#""\udc00""#, r#""\ud83dA""#] {
            assert!(
                from_json(bad).is_err(),
                "must reject lone surrogate {bad:?}"
            );
        }
    }

    #[test]
    fn structure_errors_carry_positions() {
        let e = from_json("{\n  \"a\": 1,\n  \"b\"\n}").unwrap_err();
        let ParseError::Syntax { message, line, col } = &e else {
            panic!("{e:?}")
        };
        // Error fires when `:` is expected: `ws()` has advanced past the
        // newline after `"b"`, so the failure is reported at `}` (line 4
        // 1-indexed, 0-indexed line=3, 0-indexed col=0 — `}` sits at the
        // start of its line with no leading spaces).
        assert_eq!(*line, 3);
        assert_eq!(*col, 0);
        assert!(message.contains("line 4 column 1"), "{message}");
        assert!(message.contains("`:`"), "{message}");
    }

    #[test]
    fn duplicate_keys_last_wins_first_position() {
        let n = from_json(r#"{"a": 1, "b": 2, "a": 3}"#).unwrap();
        let CustomNode::Mapping { pairs, .. } = &n else {
            unreachable!()
        };
        assert_eq!(pairs.len(), 2);
        let (_, v0) = pairs.get_index(0).unwrap();
        assert_eq!(scalar_text(v0), "3");
        let (k1, _) = pairs.get_index(1).unwrap();
        assert!(k1.source_range().is_none()); // keys carry no ranges
    }

    #[test]
    fn strict_rejections() {
        for bad in [
            "",         // empty document
            "1 2",      // two top-level values
            "[1,]",     // trailing comma
            "{'a': 1}", // single quotes
            "\"a\tb\"", // raw control char in string
            "NaN",
            "Infinity",
            "{\"a\" 1}", // missing colon
            "[1 2]",     // missing comma
            "\"unterminated",
            r#"{"k":"\x41"}"#, // invalid escape
        ] {
            assert!(from_json(bad).is_err(), "must reject {bad:?}");
        }
    }

    #[test]
    fn max_depth_is_enforced() {
        let deep = format!("{}{}", "[".repeat(10), "]".repeat(10));
        from_json_with_max_depth(&deep, 5).unwrap_err();
        from_json_with_max_depth(&deep, 20).unwrap();
    }

    #[test]
    fn jsonc_accepts_line_and_block_comments() {
        // Dialect opt-in lets `// ...` and `/* ... */` ride anywhere
        // whitespace is legal. Same AST as the equivalent strict JSON.
        let strict = r#"{"a": 1, "b": [2, 3]}"#;
        let jsonc = r#"{
            // leading note
            "a": 1, /* mid */
            "b": [
                // inside array
                2,
                3  // trailing on same line, no comma
            ]
            /* footer */
        }"#;
        let want = from_json(strict).unwrap();
        let got = from_jsonc(jsonc).unwrap();
        // `want` and `got` differ only in that `strict` never sees the
        // `,` before `]` — but the shape (values, order) is identical.
        // Re-serialize to compare semantically.
        let a = crate::json::to_json_text(&want).unwrap();
        let b = crate::json::to_json_text(&got).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn jsonc_still_rejects_trailing_commas_and_multi_root() {
        // Trailing commas remain a hard error even under JSONC — the
        // accepted language is JSON + comments, not JSON5.
        assert!(from_jsonc(r#"[1, 2,]"#).is_err());
        assert!(from_jsonc("1 2").is_err());
        assert!(from_jsonc("NaN").is_err());
    }

    #[test]
    fn strict_rejects_comments_but_jsonc_accepts_them() {
        let src = "{\"a\": 1 /* note */}";
        assert!(
            from_json(src).is_err(),
            "strict mode must reject block comments"
        );
        assert!(from_jsonc(src).is_ok());
    }

    #[test]
    fn unterminated_block_comment_is_treated_as_trailing_content() {
        // The JSONC scanner bails silently on an unterminated block, so
        // the outer parser falls through to the normal "trailing
        // characters after the JSON value" rejection with a stable
        // position instead of hanging on a slice-out-of-bounds.
        let err = from_jsonc("1 /* unterminated").unwrap_err();
        assert!(format!("{err:?}").contains("trailing"), "{err:?}");
    }

    #[test]
    fn resolved_types_match_json_semantics() {
        // Plain scalars produced by the parser resolve back through the
        // core schema to the same JSON types (round trip in the AST).
        // `null` is the exception: the parser emits a dedicated `Null` AST
        // variant rather than a plain scalar with the text "null".
        let n_null = from_json("null").unwrap();
        assert!(matches!(n_null, CustomNode::Null { .. }));
        let cases = [
            ("42", Schema::Core),
            ("-1.5", Schema::Core),
            ("1e3", Schema::Core),
            ("true", Schema::Core),
            ("false", Schema::Core),
        ];
        for (text, schema) in cases {
            let n = from_json(text).unwrap();
            let CustomNode::Scalar {
                value,
                style: ScalarStyle::Plain,
                ..
            } = &n
            else {
                panic!("{text} should be a plain scalar: {n:?}")
            };
            assert_eq!(value.as_ref(), text);
            // resolves to a non-string JSON primitive
            assert!(!matches!(
                schema.resolve(value),
                crate::parser::yaml::YamlType::Str(_)
            ));
        }
    }
}
