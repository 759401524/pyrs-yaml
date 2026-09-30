//! TOML parser producing the shared `CustomNode` AST.
//!
//! Byte-level scanner aligned with the [TOML v1.1.0 grammar][spec].
//! Every rejection surfaces as `ParseError::Syntax` with a 0-indexed
//! `line`/`col` derived from the byte offset where the failure was
//! detected. PR #116 added the 1.1 dialect axis (`TomlDialect`);
//! `from_toml` defaults to [`TomlDialect::V1_1`] and every 1.0-only
//! input parses identically.
//!
//! [spec]: https://toml.io/en/v1.1.0

use crate::ast::CustomNode;
use crate::error::ParseError;
use crate::toml::{CowTable, KVAnnotations, TomlTable, TomlValue, cow_table_to_node};
use indexmap::IndexMap;

/// TOML grammar dialect to accept at parse time.
///
/// PR #116 grew the engine from 1.0 to 1.1. Existing callers of
/// [`from_toml`] get the 1.1 superset for free; strict-1.0 consumers
/// (CI schema validators pinning cargo-style TOML semantics, for
/// example) can call [`from_toml_v1_0`] to reject every 1.1-only
/// input with a positional parse error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TomlDialect {
    /// TOML 1.0.0 (2021-01-11) — inline tables stay on one line with
    /// no trailing comma; basic strings use only the 1.0 escape set;
    /// time / date-time values must include seconds.
    V1_0,
    /// TOML 1.1.0 (2025-12-18) — adds A1 multi-line inline tables +
    /// trailing commas inside them, A2 `\xHH`, A3 `\e`, and A4
    /// optional seconds in time-of-day.
    #[default]
    V1_1,
}

/// Parse a TOML document with the default V1_1 dialect.
pub fn from_toml(src: &str) -> Result<CustomNode, ParseError> {
    from_toml_with_options(src, TomlDialect::default())
}

/// Parse a TOML document pinned to the strict TOML 1.0.0 grammar.
pub fn from_toml_v1_0(src: &str) -> Result<CustomNode, ParseError> {
    from_toml_with_options(src, TomlDialect::V1_0)
}

/// Parse a TOML document selecting an explicit dialect.
pub fn from_toml_with_options(src: &str, dialect: TomlDialect) -> Result<CustomNode, ParseError> {
    let mut p = Parser {
        s: src.as_bytes(),
        text: src,
        pos: 0,
        root: CowTable::new(),
        current: Vec::new(),
        defined: Vec::new(),
        pending_leading: None,
        pending_blank: false,
        dialect,
        pending_inline_comment: None,
        pending_comment_err: None,
    };
    p.parse_document()?;
    // Comments are scanned by the infallible `take_comment` (several callers live
    // in non-Result helpers), so a forbidden control byte is accumulated as a
    // sticky error and surfaced here once the rest of the document parses.
    if let Some(e) = p.pending_comment_err.take() {
        return Err(e);
    }
    Ok(cow_table_to_node(p.root))
}

struct Parser<'a> {
    s: &'a [u8],
    text: &'a str,
    pos: usize,
    root: CowTable,
    /// Path of the active table (set by `[a.b]` or `[[a.b]]` headers).
    current: Vec<String>,
    /// Explicit-header paths already opened. Reopening is a duplicate error.
    defined: Vec<String>,
    /// The most-recent standalone `# ...` comment on a line of its own.
    /// Consumed by the next KV pair or table header as its `leading`;
    /// earlier blocks separated by blank lines are dropped so only the
    /// last block survives (matches the YAML receiver's model).
    pending_leading: Option<String>,
    /// True once `skip_all_blank` has crossed at least one blank line
    /// since the last real pair / header / EOF. Claimed by the next pair
    /// or header as its `blank_before` hint so the writer can reproduce
    /// the visual grouping.
    pending_blank: bool,
    /// Grammar dialect gate for A1 / A2 / A3 / A4 (PR #116).
    dialect: TomlDialect,
    /// Most-recent `# ...` comment seen inside a multi-line inline
    /// table on a line of its own. Claimed by the next member as its
    /// `leading` annotation (PR #119 interior-comment fidelity).
    pending_inline_comment: Option<String>,
    /// First forbidden control-character found inside a comment body, held as a
    /// deferred error because `take_comment` is infallible (toml-test
    /// `invalid/control/comment-*`). Surfaced by the document entry point.
    pending_comment_err: Option<ParseError>,
}

impl<'a> Parser<'a> {
    fn eof(&self) -> bool {
        self.pos >= self.s.len()
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.pos).copied()
    }

    fn starts_with(&self, lit: &[u8]) -> bool {
        self.s[self.pos..].starts_with(lit)
    }

    fn byte_at(&self, i: usize) -> Option<u8> {
        self.s.get(i).copied()
    }

    fn err(&self, msg: &str) -> ParseError {
        self.err_at(msg, self.pos)
    }

    fn err_at(&self, msg: &str, byte_pos: usize) -> ParseError {
        let cut = floor_char_boundary(self.text, byte_pos.min(self.text.len()));
        let consumed = &self.text[..cut];
        let line = consumed.matches('\n').count();
        let col = match consumed.rfind('\n') {
            Some(nl) => consumed[nl + 1..].chars().count(),
            None => consumed.chars().count(),
        };
        ParseError::Syntax {
            message: format!(
                "TOML parse error: {msg} at line {} column {}",
                line + 1,
                col + 1
            ),
            line,
            col,
        }
    }

    /// Skip spaces and tabs (no comment consumption).
    fn skip_spaces(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t')) {
            self.pos += 1;
        }
    }

    fn take_comment(&mut self) -> String {
        self.pos += 1; // consume '#'
        let start = self.pos;
        while !self.eof() && self.peek() != Some(b'\n') && self.peek() != Some(b'\r') {
            self.pos += 1;
        }
        // TOML comments may not contain raw control codes (U+0000..U+001F except
        // tab, plus DEL U+007F). Record the first offense as a sticky error; the
        // multi-byte-safe byte test never flags a UTF-8 continuation byte.
        if self.pending_comment_err.is_none() {
            let mut bad: Option<usize> = None;
            for i in start..self.pos {
                if Self::forbidden_control(self.s[i]) {
                    bad = Some(i);
                    break;
                }
            }
            if let Some(off) = bad {
                self.pending_comment_err = Some(self.err_at("control character in comment", off));
            }
        }
        self.text[start..self.pos].trim().to_string()
    }

    fn skip_all_blank(&mut self) {
        // A single `\n` here counts as a blank line: `finish_line` already
        // consumed the terminating newline of the previous content line,
        // so any additional newline seen during `skip_all_blank` came
        // from an empty line between the previous and next pair. Two or
        // more such newlines set the flag once; a comment line resets the
        // tally so a `# note\nkey = 1` sequence does NOT imply a blank
        // between them.
        let mut newlines_here = 0usize;
        loop {
            match self.peek() {
                Some(b' ') | Some(b'\t') => self.pos += 1,
                Some(b'#') => {
                    // Standalone comment line: remember the most recent
                    // one so the next pair or header adopts it as its
                    // leading. A comment runs to end-of-line, so the
                    // newline that terminates it is the comment's own
                    // break, NOT a blank separator. Eat it here and
                    // reset the tally so only genuinely empty lines
                    // after the comment count toward `pending_blank`.
                    // (PR #119 caught the latent #114 bug that omitted
                    // this: promoting an inline table to a `[section]`
                    // feeds `key = v\n# lead\nkey = v` back through this
                    // parser, and the comment's terminator newline was
                    // being miscounted as a blank line.)
                    self.pending_leading = Some(self.take_comment());
                    if self.peek() == Some(b'\n') {
                        self.pos += 1;
                    } else if self.peek() == Some(b'\r')
                        && self.byte_at(self.pos + 1) == Some(b'\n')
                    {
                        self.pos += 2;
                    }
                    newlines_here = 0;
                }
                Some(b'\n') => {
                    newlines_here += 1;
                    if newlines_here >= 1 {
                        self.pending_blank = true;
                    }
                    self.pos += 1;
                }
                Some(b'\r') if self.byte_at(self.pos + 1) == Some(b'\n') => {
                    newlines_here += 1;
                    if newlines_here >= 1 {
                        self.pending_blank = true;
                    }
                    self.pos += 2;
                }
                _ => return,
            }
        }
    }

    /// After the semantic content of a line: allow trailing ws, one comment,
    /// and one line ending. Errors on any other trailing byte. The inline
    /// comment (if any) is returned so `parse_line` can attach it as the
    /// pair's trailing annotation.
    fn finish_line(&mut self) -> Result<Option<String>, ParseError> {
        self.skip_spaces();
        let inline = if self.peek() == Some(b'#') {
            Some(self.take_comment())
        } else {
            None
        };
        self.consume_line_ending()?;
        Ok(inline)
    }

    fn consume_line_ending(&mut self) -> Result<(), ParseError> {
        if self.pos < self.s.len() {
            if self.s[self.pos] == b'\n' {
                self.pos += 1;
                return Ok(());
            }
            if self.s[self.pos] == b'\r'
                && self.pos + 1 < self.s.len()
                && self.s[self.pos + 1] == b'\n'
            {
                self.pos += 2;
                return Ok(());
            }
            let ch = self.s[self.pos] as char;
            return Err(self.err(&format!("expected newline, found {ch:?}")));
        }
        Ok(())
    }

    fn at_line_end_or_eof(&self) -> bool {
        if self.eof() {
            return true;
        }
        matches!(self.peek(), Some(b'\n' | b'\r' | b'#'))
    }

    fn expect_byte(&mut self, b: u8, ctx: &str) -> Result<(), ParseError> {
        if self.peek() == Some(b) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.err(ctx))
        }
    }

    // ------ top-level loop --------------------------------------------------

    fn parse_document(&mut self) -> Result<(), ParseError> {
        loop {
            self.skip_all_blank();
            if self.eof() {
                return Ok(());
            }
            self.parse_line()?;
        }
    }

    fn parse_line(&mut self) -> Result<(), ParseError> {
        if self.peek() == Some(b'[') {
            return self.parse_table_header();
        }
        if self.at_line_end_or_eof() {
            // stray comment/CR-only line, already handled by skip_all_blank
            self.pos += 1;
            return Ok(());
        }
        // Take ownership of the pending standalone comment block (if any)
        // so it becomes THIS pair's leading rather than the next one's.
        let leading = self.pending_leading.take();
        let blank = std::mem::take(&mut self.pending_blank);
        let key_start = self.pos;
        let key = self.parse_key_path()?;
        self.skip_spaces();
        if self.peek() != Some(b'=') {
            return Err(self.err_at("expected `=` after key", self.pos));
        }
        self.pos += 1;
        self.skip_spaces();
        let value = self.parse_value()?;
        let trailing = self.finish_line()?;
        let anns = KVAnnotations {
            leading,
            trailing,
            blank_before: blank,
        };
        self.register_kv(&key, value, anns, key_start)
    }

    fn parse_table_header(&mut self) -> Result<(), ParseError> {
        // The pending standalone block sits ABOVE the header; take it
        // before parsing so it becomes this section's leading. `pending_blank`
        // travels through the same route to mark the section's blank separator.
        let leading = self.pending_leading.take();
        let blank = std::mem::take(&mut self.pending_blank);
        self.expect_byte(b'[', "expected `[`")?;
        let is_array = self.peek() == Some(b'[');
        if is_array {
            self.pos += 1;
        }
        self.skip_spaces();
        let key = self.parse_key_path()?;
        self.skip_spaces();
        if is_array {
            self.expect_byte(b']', "expected `]]`")?;
            self.expect_byte(b']', "expected `]]`")?;
        } else {
            self.expect_byte(b']', "expected `]`")?;
        }
        self.skip_spaces();
        let comment = if self.peek() == Some(b'#') {
            Some(self.take_comment())
        } else {
            None
        };
        self.consume_line_ending()?;
        self.register_header(&key, is_array, comment, leading, blank)
    }

    // ------ keys -------------------------------------------------------------

    fn parse_key_path(&mut self) -> Result<Vec<String>, ParseError> {
        let mut parts = vec![self.parse_key_segment()?];
        loop {
            self.skip_spaces();
            if self.peek() == Some(b'.') {
                self.pos += 1;
                self.skip_spaces();
                parts.push(self.parse_key_segment()?);
            } else {
                return Ok(parts);
            }
        }
    }

    fn parse_key_segment(&mut self) -> Result<String, ParseError> {
        // Keys are never multi-line; the `multiline` flag from the string
        // scanners is discarded here.
        match self.peek() {
            Some(b'"') => self.parse_basic_string_content().map(|(s, _)| s),
            Some(b'\'') => self.parse_literal_string().map(|(s, _)| s),
            _ => self.parse_bare_key(),
        }
    }

    fn parse_bare_key(&mut self) -> Result<String, ParseError> {
        let start = self.pos;
        while matches!(
            self.peek(),
            Some(b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-')
        ) {
            self.pos += 1;
        }
        if self.pos == start {
            return Err(self.err("expected a bare, quoted, or dotted key"));
        }
        Ok(self.text[start..self.pos].to_string())
    }

    // ------ values -----------------------------------------------------------

    fn parse_value(&mut self) -> Result<TomlValue, ParseError> {
        match self.peek() {
            Some(b'"') | Some(b'[') => self.parse_string_or_array(),
            Some(b'\'') => self.parse_literal_or_start_date(None),
            Some(b'{') => self.parse_inline_table(),
            Some(b't') => self.parse_true(),
            Some(b'f') => self.parse_false(),
            Some(b'i') => self.parse_inf(true),
            Some(b'n') => self.parse_nan(true),
            Some(b'+') => self.parse_signed_numeric(true),
            Some(b'-') => self.parse_signed_numeric(false),
            Some(b'0'..=b'9') => self.parse_unsigned_numeric(),
            _ => Err(self.err("expected a value")),
        }
    }

    fn parse_true(&mut self) -> Result<TomlValue, ParseError> {
        if self.starts_with(b"true") {
            self.pos += 4;
            Ok(TomlValue::Boolean(true))
        } else {
            Err(self.err("expected literal `true`"))
        }
    }

    fn parse_false(&mut self) -> Result<TomlValue, ParseError> {
        if self.starts_with(b"false") {
            self.pos += 5;
            Ok(TomlValue::Boolean(false))
        } else {
            Err(self.err("expected literal `false`"))
        }
    }

    fn parse_inf(&mut self, positive: bool) -> Result<TomlValue, ParseError> {
        let _ = positive;
        if self.starts_with(b"inf") {
            self.pos += 3;
            Ok(TomlValue::Float(f64::INFINITY, None))
        } else {
            Err(self.err("expected `inf`"))
        }
    }

    fn parse_nan(&mut self, positive: bool) -> Result<TomlValue, ParseError> {
        let _ = positive;
        if self.starts_with(b"nan") {
            self.pos += 3;
            Ok(TomlValue::Float(f64::NAN, None))
        } else {
            Err(self.err("expected `nan`"))
        }
    }

    fn parse_signed_numeric(&mut self, positive: bool) -> Result<TomlValue, ParseError> {
        self.pos += 1;
        match self.peek() {
            Some(b'i') if !positive => {
                if self.starts_with(b"inf") {
                    self.pos += 3;
                    return Ok(TomlValue::Float(f64::NEG_INFINITY, None));
                }
                Err(self.err("expected `inf`"))
            }
            Some(b'i') if positive => {
                if self.starts_with(b"inf") {
                    self.pos += 3;
                    return Ok(TomlValue::Float(f64::INFINITY, None));
                }
                Err(self.err("expected `inf`"))
            }
            Some(b'n') => {
                if self.starts_with(b"nan") {
                    self.pos += 3;
                    return Ok(TomlValue::Float(f64::NAN, None));
                }
                Err(self.err("expected `nan`"))
            }
            Some(b'0'..=b'9') => {
                // TOML forbids a sign on radix-prefixed integers: `signed-int`
                // only ever wraps a *decimal* integer, so `-0x1F` / `+0b1` /
                // `+0o644` are invalid and must be rejected, not folded away.
                if self.starts_with(b"0x") || self.starts_with(b"0o") || self.starts_with(b"0b") {
                    return Err(self.err("radix-prefixed integer cannot carry a sign"));
                }
                let v = self.parse_decimal_numeric_body()?;
                Ok(if positive { v } else { negate_numeric(v) })
            }
            _ => Err(self.err("expected a signed numeric value")),
        }
    }

    fn parse_unsigned_numeric(&mut self) -> Result<TomlValue, ParseError> {
        // Prefixed integers: 0x / 0o / 0b (TOML 1.0 disallows 0x0x-style
        // leading zero after the prefix). Radix-prefixed spellings are
        // preserved verbatim because YAML Core resolves them back to the
        // same integer without needing a tag fence.
        if self.starts_with(b"0x") {
            self.pos += 2;
            let (v, src) = self.parse_prefixed_integer_with_source(16)?;
            return Ok(TomlValue::Integer(v, Some(src)));
        }
        if self.starts_with(b"0o") {
            self.pos += 2;
            let (v, src) = self.parse_prefixed_integer_with_source(8)?;
            return Ok(TomlValue::Integer(v, Some(src)));
        }
        if self.starts_with(b"0b") {
            self.pos += 2;
            // YAML Core schema does not recognise binary literals, so
            // the projected plain scalar would resolve back to Str on
            // the round trip. We canonicalise binary integers to decimal
            // rather than introduce a `!!int` tag fence (see design doc
            // D1: only spellings YAML Core also accepts get preserved).
            let (v, _src) = self.parse_prefixed_integer_with_source(2)?;
            return Ok(TomlValue::Integer(v, None));
        }
        // A leading digit could start a date/time or a decimal number.
        // Peek 10 bytes for a date-time prefix (YYYY-MM-DD) or 8+1 for a
        // bare local time (HH:MM:SS).
        if self.looks_like_datetime_prefix() {
            return self.parse_datetime();
        }
        if self.looks_like_time_prefix() {
            return self.parse_datetime();
        }
        self.parse_decimal_numeric_body()
    }

    fn looks_like_datetime_prefix(&self) -> bool {
        // YYYY-MM-DD...
        let p = self.pos;
        matches!(self.byte_at(p + 4), Some(b'-'))
            && matches!(self.byte_at(p + 7), Some(b'-'))
            && (0..4).all(|i| matches!(self.byte_at(p + i), Some(b'0'..=b'9')))
            && (matches!(self.byte_at(p + 5), Some(b'0'..=b'9'))
                && matches!(self.byte_at(p + 6), Some(b'0'..=b'9'))
                && matches!(self.byte_at(p + 8), Some(b'0'..=b'9'))
                && matches!(self.byte_at(p + 9), Some(b'0'..=b'9')))
    }

    fn looks_like_time_prefix(&self) -> bool {
        // HH:MM:SS... (all dialects) or HH:MM (V1_1 only, PR #116 A4).
        let p = self.pos;
        let hh = matches!(self.byte_at(p), Some(b'0'..=b'9'))
            && matches!(self.byte_at(p + 1), Some(b'0'..=b'9'))
            && self.byte_at(p + 2) == Some(b':')
            && matches!(self.byte_at(p + 3), Some(b'0'..=b'9'))
            && matches!(self.byte_at(p + 4), Some(b'0'..=b'9'));
        if !hh {
            return false;
        }
        if self.byte_at(p + 5) == Some(b':') {
            // HH:MM:SS form: seconds pair must be digits too.
            return matches!(self.byte_at(p + 6), Some(b'0'..=b'9'))
                && matches!(self.byte_at(p + 7), Some(b'0'..=b'9'));
        }
        // No seconds component: only valid under V1_1.
        self.dialect == TomlDialect::V1_1
    }

    fn parse_decimal_numeric_body(&mut self) -> Result<TomlValue, ParseError> {
        let num_start = self.pos;
        self.scan_int_digits()?;
        // TOML forbids leading zeros in the decimal integer part (`01`, `007`,
        // `-01`, `01.5`); only a bare `0` (optionally followed by `.`/`e`) is
        // legal. Checked on the integer run before the float markers below.
        {
            let digits = self.text[num_start..self.pos].replace('_', "");
            if digits.len() > 1 && digits.starts_with('0') {
                return Err(self.err_at("leading zeros in decimal integer", num_start));
            }
        }
        if self.peek() == Some(b'.') {
            self.pos += 1;
            self.scan_int_digits()?;
            let frac_end = self.pos;
            if frac_end == num_start {
                return Err(self.err("empty fraction"));
            }
            let is_float = true;
            self.scan_optional_exponent(is_float, num_start)?;
            return self.emit_float(num_start);
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.scan_optional_exponent(true, num_start)?;
            return self.emit_float(num_start);
        }
        // Emit integer if no float marker. The source text carries the
        // digits (plus any leading `-`/`+` handled upstream).
        let text = &self.text[num_start..self.pos];
        let cleaned = text.replace('_', "");
        let value: i64 = cleaned
            .parse()
            .map_err(|_| self.err_at("integer out of range or malformed", num_start))?;
        // Preserve `-0` verbatim because YAML Core reads `-0` back as
        // `Int(0)` and we can round-trip the sign; every other decimal
        // form either matches `i64::to_string()` (no work needed) or
        // contains `_`/`+` which we canonicalize away.
        let source: Option<std::sync::Arc<str>> = if text == "-0" {
            Some(std::sync::Arc::from(text))
        } else {
            None
        };
        Ok(TomlValue::Integer(value, source))
    }

    fn emit_float(&mut self, num_start: usize) -> Result<TomlValue, ParseError> {
        let text = &self.text[num_start..self.pos];
        let cleaned = text.replace('_', "");
        let value: f64 = cleaned
            .parse()
            .map_err(|_| self.err_at("malformed float", num_start))?;
        // Preserve exponent spelling (`1e10`, `-3.14e-2`) verbatim: YAML
        // Core reads them back as Float, so the projected plain scalar
        // carries no tag and stays fully interoperable with the YAML
        // pipeline. Canonical decimal forms (`1.5`, `-0.0`) already
        // match the parsed value's rendering, so no source is recorded.
        let has_exponent = text.bytes().any(|b| b == b'e' || b == b'E');
        let source: Option<std::sync::Arc<str>> = if has_exponent && !text.contains('_') {
            Some(std::sync::Arc::from(text))
        } else {
            None
        };
        Ok(TomlValue::Float(value, source))
    }

    fn scan_int_digits(&mut self) -> Result<(), ParseError> {
        let start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9') | Some(b'_')) {
            if self.peek() == Some(b'_') {
                if self.pos == start {
                    return Err(self.err("leading underscore in numeric"));
                }
                let prev = self.byte_at(self.pos - 1).unwrap_or(b'0');
                if prev == b'_' {
                    return Err(self.err("double underscore in numeric"));
                }
            }
            self.pos += 1;
        }
        if self.pos == start {
            return Err(self.err("expected digits"));
        }
        if self.byte_at(self.pos - 1) == Some(b'_') {
            return Err(self.err("trailing underscore in numeric"));
        }
        Ok(())
    }

    fn scan_optional_exponent(
        &mut self,
        is_float: bool,
        num_start: usize,
    ) -> Result<(), ParseError> {
        if !matches!(self.peek(), Some(b'e' | b'E')) {
            return Ok(());
        }
        if !is_float {
            // Already handled upstream; this branch is defensive only.
            return Ok(());
        }
        self.pos += 1;
        if matches!(self.peek(), Some(b'+' | b'-')) {
            self.pos += 1;
        }
        let exp_start = self.pos;
        self.scan_int_digits()?;
        if self.pos == exp_start {
            return Err(self.err_at("expected digits in exponent", num_start));
        }
        Ok(())
    }

    /// Parse a `0x`/`0o`/`0b` prefixed integer and return both the decoded
    /// value and the exact source spelling (with the prefix), so the
    /// writer can round-trip radix notation verbatim.
    fn parse_prefixed_integer_with_source(
        &mut self,
        radix: u32,
    ) -> Result<(i64, std::sync::Arc<str>), ParseError> {
        let prefix_str = match radix {
            16 => "0x",
            8 => "0o",
            _ => "0b",
        };
        let start = self.pos;
        let digit_test: fn(u8) -> bool = match radix {
            16 => |b: u8| b.is_ascii_hexdigit(),
            8 => |b| matches!(b, b'0'..=b'7'),
            _ => |b| matches!(b, b'0'..=b'1'),
        };
        while self.peek().is_some_and(digit_test) || self.peek() == Some(b'_') {
            self.pos += 1;
        }
        if self.pos == start {
            return Err(self.err("expected digits after prefix"));
        }
        let raw = &self.text[start..self.pos];
        let cleaned = raw.replace('_', "");
        if raw.starts_with('_') || raw.ends_with('_') || raw.contains("__") {
            return Err(self.err_at("malformed underscore placement in integer", start));
        }
        i64::from_str_radix(&cleaned, radix)
            .map(|v| {
                // Preserve the prefix + digits spelling, but strip `_`
                // separators: YAML Core's numeric grammar rejects them, so
                // the projected plain scalar must contain only the radix
                // prefix and continuous digit run to survive the round trip
                // through `to_yaml -> load_toml -> to_toml`.
                let digits_no_underscores = raw.replace('_', "");
                let mut src = String::with_capacity(prefix_str.len() + digits_no_underscores.len());
                src.push_str(prefix_str);
                src.push_str(&digits_no_underscores);
                (v, std::sync::Arc::from(src))
            })
            .map_err(|_| self.err_at("prefixed integer out of range", start))
    }

    fn parse_string_or_array(&mut self) -> Result<TomlValue, ParseError> {
        if self.peek() == Some(b'[') {
            return self.parse_array();
        }
        // "..." or """..."""
        self.parse_basic_string("")
    }

    fn parse_literal_or_start_date(&mut self, _hint: Option<()>) -> Result<TomlValue, ParseError> {
        self.parse_literal_string()
            .map(|(s, ml)| TomlValue::String(s, ml))
    }

    fn parse_basic_string(&mut self, _prefix: &str) -> Result<TomlValue, ParseError> {
        self.parse_basic_string_content()
            .map(|(s, ml)| TomlValue::String(s, ml))
    }

    fn parse_basic_string_content(&mut self) -> Result<(String, bool), ParseError> {
        let ml = self.starts_with(b"\"\"\"");
        if ml {
            self.pos += 3;
            // Skip first immediate newline (TOML 1.0).
            if self.peek() == Some(b'\n') {
                self.pos += 1;
            } else if self.starts_with(b"\r\n") {
                self.pos += 2;
            }
            self.scan_multiline_basic_content().map(|s| (s, true))
        } else {
            self.pos += 1;
            self.scan_single_line_basic_content().map(|s| (s, false))
        }
    }

    fn scan_single_line_basic_content(&mut self) -> Result<String, ParseError> {
        let mut out = String::new();
        loop {
            let b = self
                .peek()
                .ok_or_else(|| self.err("unterminated basic string"))?;
            match b {
                b'"' => {
                    self.pos += 1;
                    return Ok(out);
                }
                b'\\' => {
                    self.pos += 1;
                    self.push_basic_escape(&mut out)?;
                }
                b'\n' | b'\r' => {
                    return Err(self.err("newline in single-line basic string"));
                }
                c if Self::forbidden_control(c) => {
                    return Err(self.err("control character in basic string"));
                }
                _ => {
                    // Copy one whole character and advance by its UTF-8 length so
                    // `self.pos` always lands on a char boundary. The previous
                    // byte-wise `floor_char_boundary` path could slice
                    // `text[pos-1..cut]` from a mid-character offset and panic on
                    // certain (lossy-decoded) inputs.
                    match self.text[self.pos..].chars().next() {
                        Some(ch) => {
                            out.push(ch);
                            self.pos += ch.len_utf8();
                        }
                        None => return Err(self.err("unterminated basic string")),
                    }
                }
            }
        }
    }

    fn push_basic_escape(&mut self, out: &mut String) -> Result<(), ParseError> {
        let e = self.peek().ok_or_else(|| self.err("unterminated escape"))?;
        self.pos += 1;
        match e {
            b'n' => out.push('\n'),
            b't' => out.push('\t'),
            b'r' => out.push('\r'),
            b'\\' => out.push('\\'),
            b'"' => out.push('"'),
            b'b' => out.push('\u{8}'),
            b'f' => out.push('\u{c}'),
            b'u' => self.push_unicode_escape(out, 4)?,
            b'U' => self.push_unicode_escape(out, 8)?,
            // TOML 1.1.0 additions (PR #116 A2/A3), gated by dialect.
            b'x' if self.dialect == TomlDialect::V1_1 => self.push_byte_escape(out)?,
            b'e' if self.dialect == TomlDialect::V1_1 => out.push('\u{1b}'),
            _ => return Err(self.err("invalid escape in basic string")),
        }
        Ok(())
    }

    /// `\xHH` — TOML 1.1.0 byte escape for codepoints 0x00..=0xFF.
    fn push_byte_escape(&mut self, out: &mut String) -> Result<(), ParseError> {
        if self.pos + 2 > self.s.len() {
            return Err(self.err("truncated \\xHH escape"));
        }
        let digits = &self.text[self.pos..self.pos + 2];
        let value =
            u32::from_str_radix(digits, 16).map_err(|_| self.err("invalid hex in \\xHH escape"))?;
        self.pos += 2;
        let c = char::from_u32(value).ok_or_else(|| self.err("invalid \\xHH byte"))?;
        out.push(c);
        Ok(())
    }

    fn push_unicode_escape(&mut self, out: &mut String, width: usize) -> Result<(), ParseError> {
        if self.pos + width > self.s.len() {
            return Err(self.err("truncated unicode escape"));
        }
        let digits = &self.text[self.pos..self.pos + width];
        let value = u32::from_str_radix(digits, 16)
            .map_err(|_| self.err("invalid hex in unicode escape"))?;
        self.pos += width;
        let c = char::from_u32(value).ok_or_else(|| self.err("invalid unicode code point"))?;
        out.push(c);
        Ok(())
    }

    fn scan_multiline_basic_content(&mut self) -> Result<String, ParseError> {
        let mut out = String::new();
        loop {
            if self.starts_with(b"\"\"\"") {
                self.pos += 3;
                // Allow up to 2 more "s as content (greedy match rules).
                while self.peek() == Some(b'"') {
                    out.push('"');
                    self.pos += 1;
                }
                return Ok(out);
            }
            let b = self
                .peek()
                .ok_or_else(|| self.err("unterminated multiline string"))?;
            match b {
                b'\\' => {
                    // Line-ending backslash: trim trailing ws to a newline.
                    let save = self.pos;
                    self.pos += 1;
                    if matches!(self.peek(), Some(b'\n') | Some(b'\r')) || self.is_at_line_end_ws()
                    {
                        // Skip spaces/tabs before newline
                        self.pos = save + 1;
                        while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
                            self.pos += 1;
                        }
                    } else {
                        self.pos = save;
                        self.pos += 1;
                        self.push_basic_escape(&mut out)?;
                    }
                }
                b'\r' if self.byte_at(self.pos + 1) != Some(b'\n') => {
                    return Err(self.err("bare carriage return in multiline string"));
                }
                c if Self::forbidden_control(c) => {
                    return Err(self.err("control character in multiline string"));
                }
                _ => {
                    // Copy one whole character and advance by its UTF-8 length so
                    // `self.pos` stays on a char boundary; the byte-wise
                    // `floor_char_boundary` path could slice `text[cur..cut]`
                    // from a mid-character offset and panic on non-ASCII content.
                    match self.text[self.pos..].chars().next() {
                        Some(ch) => {
                            out.push(ch);
                            self.pos += ch.len_utf8();
                        }
                        None => return Err(self.err("unterminated multi-line string")),
                    }
                }
            }
        }
    }

    fn is_at_line_end_ws(&self) -> bool {
        let mut i = self.pos + 1;
        while matches!(self.byte_at(i), Some(b' ' | b'\t')) {
            i += 1;
        }
        matches!(self.byte_at(i), Some(b'\n' | b'\r'))
    }

    /// TOML forbids raw C0 control codes (everything below U+0020 except tab,
    /// and newline/CR which the callers treat as line breaks) plus DEL (U+007F)
    /// in every string and comment form. A multi-byte UTF-8 sequence never has
    /// a byte in this range, so a byte-wise test never false-flags a character.
    fn forbidden_control(b: u8) -> bool {
        matches!(b, 0x00..=0x08 | 0x0B | 0x0C | 0x0E..=0x1F | 0x7F)
    }

    fn parse_literal_string(&mut self) -> Result<(String, bool), ParseError> {
        let ml = self.starts_with(b"'''");
        if ml {
            self.pos += 3;
            if self.peek() == Some(b'\n') {
                self.pos += 1;
            } else if self.starts_with(b"\r\n") {
                self.pos += 2;
            }
            let start = self.pos;
            loop {
                if self.starts_with(b"'''") {
                    let content = &self.text[start..self.pos];
                    self.pos += 3;
                    while self.peek() == Some(b'\'') {
                        self.pos += 1;
                    }
                    return Ok((strip_extra_quotes(content, true), true));
                }
                let b = self
                    .peek()
                    .ok_or_else(|| self.err("unterminated multiline literal"))?;
                if b == b'\r' && self.byte_at(self.pos + 1) != Some(b'\n') {
                    return Err(self.err("bare carriage return in multiline literal"));
                }
                if Self::forbidden_control(b) {
                    return Err(self.err("control character in multiline literal"));
                }
                self.pos += 1;
                let _ = b;
            }
        } else {
            self.pos += 1;
            let start = self.pos;
            loop {
                let b = self
                    .peek()
                    .ok_or_else(|| self.err("unterminated literal string"))?;
                if b == b'\n' || b == b'\r' {
                    return Err(self.err("newline in single-line literal string"));
                }
                if Self::forbidden_control(b) {
                    return Err(self.err("control character in literal string"));
                }
                if b == b'\'' {
                    let content = &self.text[start..self.pos];
                    self.pos += 1;
                    return Ok((content.to_string(), false));
                }
                self.pos += 1;
            }
        }
    }

    fn parse_array(&mut self) -> Result<TomlValue, ParseError> {
        self.expect_byte(b'[', "expected `[`")?;
        let mut items = Vec::new();
        loop {
            self.skip_array_ws();
            if self.peek() == Some(b']') {
                self.pos += 1;
                return Ok(TomlValue::Array(items));
            }
            if self.eof() {
                return Err(self.err("unterminated array"));
            }
            let v = self.parse_value()?;
            items.push(v);
            self.skip_array_ws();
            if self.peek() == Some(b',') {
                self.pos += 1;
                continue;
            }
            if self.peek() == Some(b']') {
                self.pos += 1;
                return Ok(TomlValue::Array(items));
            }
            return Err(self.err("expected `,` or `]` in array"));
        }
    }

    fn skip_array_ws(&mut self) {
        loop {
            match self.peek() {
                Some(b' ') | Some(b'\t') | Some(b'\n') => self.pos += 1,
                Some(b'\r') if self.byte_at(self.pos + 1) == Some(b'\n') => self.pos += 2,
                Some(b'#') => {
                    self.take_comment();
                }
                _ => return,
            }
        }
    }

    fn parse_inline_table(&mut self) -> Result<TomlValue, ParseError> {
        self.expect_byte(b'{', "expected `{`")?;
        let mut entries = Vec::new();
        // Segment paths already defined in this inline table, to detect dotted
        // key collisions (`a` vs `a.b`, `a.b` vs `a.b.c`) per TOML rules.
        let mut defined_paths: Vec<Vec<String>> = Vec::new();
        self.skip_inline_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(TomlValue::InlineTable(entries));
        }
        loop {
            self.skip_inline_ws();
            // A `# ...` on its own line above this member (captured by
            // `skip_inline_ws` into `pending_inline_comment`) is the
            // member's leading note. PR #119 plumbs it through the IR
            // so `to_toml` can reproduce interior comments.
            let leading = self.pending_inline_comment.take();
            let key = self.parse_key_path()?;
            self.skip_inline_ws();
            self.expect_byte(b'=', "expected `=` in inline table")?;
            self.skip_inline_ws();
            let v = self.parse_value()?;
            let trailing = self.take_inline_trailing_comment();
            let anns = KVAnnotations {
                leading,
                trailing,
                blank_before: false,
            };
            // toml-test strictness: inline tables reject a key that collides
            // with any already-defined path - equal, or one a prefix of the
            // other at segment boundaries (`a` then `a.b`, or `a.b` then
            // `a.b.c`). Valid sibling dotted keys (`{ a.b = 1, a.c = 2 }`) do
            // not collide because neither path is a prefix of the other.
            let conflict = defined_paths
                .iter()
                .any(|q| q[..q.len().min(key.len())] == key[..q.len().min(key.len())]);
            if conflict {
                return Err(self.err(&format!(
                    "duplicate key `{}` in inline table",
                    key.join(".")
                )));
            }
            let joined_key = key.join(".");
            defined_paths.push(key.clone());
            entries.push((joined_key, v, anns));
            self.skip_inline_ws();
            if self.peek() == Some(b',') {
                self.pos += 1;
                // TOML 1.1.0 (PR #116 A1): allow a trailing comma before
                // `}`. V1_0 continues into the next loop iteration which
                // then rejects the missing key with the same
                // "expected `=` in inline table" error users already
                // saw before this PR landed.
                if self.dialect == TomlDialect::V1_1 {
                    self.skip_inline_ws();
                    if self.peek() == Some(b'}') {
                        self.pos += 1;
                        return Ok(TomlValue::InlineTable(entries));
                    }
                }
                continue;
            }
            if self.peek() == Some(b'}') {
                self.pos += 1;
                return Ok(TomlValue::InlineTable(entries));
            }
            return Err(self.err("expected `,` or `}` in inline table"));
        }
    }

    /// Capture a `# ...` comment that sits on the same line as the
    /// member value (i.e. only spaces/tabs between the value and the
    /// `#`). Returns `None` once a newline or the `,` / `}` separator
    /// is reached, so an own-line comment stays pending for the next
    /// member's leading slot rather than being misfiled as trailing.
    fn take_inline_trailing_comment(&mut self) -> Option<String> {
        if self.dialect != TomlDialect::V1_1 {
            return None;
        }
        let mut i = self.pos;
        while matches!(self.byte_at(i), Some(b' ' | b'\t')) {
            i += 1;
        }
        if self.byte_at(i) == Some(b'#') {
            self.pos = i;
            Some(self.take_comment())
        } else {
            None
        }
    }

    fn skip_inline_ws(&mut self) {
        // TOML 1.0.0 confined inline tables to a single physical line so
        // only spaces and tabs separated members. TOML 1.1.0 (PR #116
        // A1) additionally allows newlines and comment lines between
        // members; the dialect gate keeps V1_0 rejecting them.
        loop {
            match self.peek() {
                Some(b' ' | b'\t') => self.pos += 1,
                Some(b'\n') if self.dialect == TomlDialect::V1_1 => self.pos += 1,
                Some(b'\r')
                    if self.dialect == TomlDialect::V1_1
                        && self.byte_at(self.pos + 1) == Some(b'\n') =>
                {
                    self.pos += 2;
                }
                Some(b'#') if self.dialect == TomlDialect::V1_1 => {
                    // PR #119: remember the most recent own-line comment
                    // so the next member adopts it as its leading note.
                    self.pending_inline_comment = Some(self.take_comment());
                }
                _ => return,
            }
        }
    }

    fn parse_datetime(&mut self) -> Result<TomlValue, ParseError> {
        // Assumed YYYY-MM-DD or HH:MM:SS pattern already checked upstream.
        let start = self.pos;
        let has_date = !matches!(self.byte_at(start + 2), Some(b':'));
        if has_date {
            self.pos += 10; // YYYY-MM-DD
            if !validate_date(&self.text[start..self.pos]) {
                return Err(self.err_at("invalid calendar date", start));
            }
            // Optional T / t / space + time follows.
            let sep = self.peek();
            // Space-separated date-time: the character at `pos + 3`
            // (relative to the space) is the `:` in `HH:MM`. A
            // pre-#116 off-by-one used `pos + 4` here, which meant
            // space-separated date-times silently fell through the
            // `has_time` check and were rejected later. The new A4
            // optional-seconds test exercises the path and caught
            // the bug.
            let space_time_sep = sep == Some(b' ')
                && matches!(self.byte_at(self.pos + 1), Some(b'0'..=b'9'))
                && matches!(self.byte_at(self.pos + 2), Some(b'0'..=b'9'))
                && self.byte_at(self.pos + 3) == Some(b':')
                && matches!(self.byte_at(self.pos + 4), Some(b'0'..=b'9'))
                && matches!(self.byte_at(self.pos + 5), Some(b'0'..=b'9'));
            let has_time = matches!(sep, Some(b'T') | Some(b't')) || space_time_sep;
            if has_time {
                self.pos += 1;
            }
            if has_time {
                self.scan_time_body()?;
                // Optional offset (Z / z / ±HH:MM). Consumed but not
                // surfaced separately: the plugin tag is uniform `timestamp`
                // across every datetime shape, and the exact text (including
                // any offset suffix) is preserved in the scalar value.
                match self.peek() {
                    Some(b'Z') | Some(b'z') => {
                        self.pos += 1;
                    }
                    Some(b'+') | Some(b'-') => {
                        self.pos += 1;
                        // HH:MM
                        if !(matches!(self.peek(), Some(b'0'..=b'9'))
                            && matches!(self.byte_at(self.pos + 1), Some(b'0'..=b'9'))
                            && self.byte_at(self.pos + 2) == Some(b':')
                            && matches!(self.byte_at(self.pos + 3), Some(b'0'..=b'9'))
                            && matches!(self.byte_at(self.pos + 4), Some(b'0'..=b'9')))
                        {
                            return Err(self.err("invalid offset in date-time"));
                        }
                        self.pos += 5;
                    }
                    _ => {}
                }
                let text = self.text[start..self.pos].to_string();
                Ok(TomlValue::Datetime(text, None))
            } else {
                // Date-only (no time component): tag `!date` so the load path
                // uses `date.fromisoformat` and round-trips as a local date
                // rather than a midnight `datetime`.
                let text = self.text[start..self.pos].to_string();
                Ok(TomlValue::Datetime(text, Some("date")))
            }
        } else {
            // Time-only (no date component): tag `!time` so the load path
            // uses `time.fromisoformat`; `datetime.fromisoformat` rejects a
            // bare `HH:MM:SS` with `Invalid isoformat string`.
            self.scan_time_body()?;
            let text = self.text[start..self.pos].to_string();
            Ok(TomlValue::Datetime(text, Some("time")))
        }
    }

    fn scan_time_body(&mut self) -> Result<(), ParseError> {
        let start = self.pos;
        // HH:MM is required in every dialect. TOML 1.1.0 (PR #116 A4)
        // additionally allows HH:MM without seconds; V1_0 continues to
        // require the full HH:MM:SS triple.
        if !(matches!(self.peek(), Some(b'0'..=b'9'))
            && matches!(self.byte_at(self.pos + 1), Some(b'0'..=b'9'))
            && self.byte_at(self.pos + 2) == Some(b':')
            && matches!(self.byte_at(self.pos + 3), Some(b'0'..=b'9'))
            && matches!(self.byte_at(self.pos + 4), Some(b'0'..=b'9')))
        {
            return Err(self.err_at("invalid time-of-day", start));
        }
        self.pos += 5;
        // Optional `:SS(.frac)?`. Under V1_0 its absence is an error.
        if self.peek() == Some(b':') {
            self.pos += 1;
            if !(matches!(self.peek(), Some(b'0'..=b'9'))
                && matches!(self.byte_at(self.pos + 1), Some(b'0'..=b'9')))
            {
                return Err(self.err_at("invalid seconds in time-of-day", start));
            }
            self.pos += 2;
            if self.peek() == Some(b'.') {
                self.pos += 1;
                let fs = self.pos;
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.pos += 1;
                }
                if self.pos == fs {
                    return Err(self.err_at("empty fractional seconds", start));
                }
            }
        } else if self.dialect == TomlDialect::V1_0 {
            return Err(self.err_at("invalid time-of-day", start));
        }
        Ok(())
    }

    // ------ AST assembly -----------------------------------------------------

    fn register_kv(
        &mut self,
        key: &[String],
        value: TomlValue,
        anns: KVAnnotations,
        key_start: usize,
    ) -> Result<(), ParseError> {
        if key.is_empty() {
            return Err(self.err_at("empty key path", key_start));
        }
        // Walk from the root through the current section path, then descend
        // along the dotted key. Mutable-pointer traversal mirrors
        // `register_header` because Rust's borrow-checker rejects the naive
        // `cur = inner` reborrow inside a loop.
        let mut root_ptr = &mut self.root as *mut CowTable;
        for seg in &self.current {
            unsafe {
                let tbl = &mut *root_ptr;
                match tbl.entries.get_mut(seg.as_str()) {
                    Some(TomlTable::Explicit(inner)) | Some(TomlTable::Implicit(inner)) => {
                        root_ptr = inner as *mut CowTable;
                    }
                    Some(TomlTable::ArrayOfTables(list)) => {
                        let Some(last) = list.last_mut() else {
                            return Err(Parser::static_err(
                                "array-of-tables missing tail while descending",
                                key_start,
                            ));
                        };
                        root_ptr = last as *mut CowTable;
                    }
                    _ => {
                        return Err(Parser::static_err(
                            "path descends into a non-table value",
                            key_start,
                        ));
                    }
                }
            }
        }
        for (i, seg) in key.iter().enumerate() {
            let last = i + 1 == key.len();
            unsafe {
                let tbl = &mut *root_ptr;
                if last {
                    if tbl.entries.contains_key(seg.as_str()) {
                        return Err(Parser::static_err(
                            &format!("duplicate key `{seg}`"),
                            key_start,
                        ));
                    }
                    tbl.entries
                        .insert(seg.clone(), TomlTable::Value(value, anns));
                    return Ok(());
                }
                match tbl.entries.get_mut(seg.as_str()) {
                    Some(TomlTable::Implicit(inner)) => {
                        root_ptr = inner as *mut CowTable;
                    }
                    Some(TomlTable::Explicit(_)) => {
                        return Err(Parser::static_err(
                            "cannot extend a table already defined by a header",
                            key_start,
                        ));
                    }
                    Some(TomlTable::Value(_, _)) => {
                        return Err(Parser::static_err(
                            "dotted key path traverses a non-table value",
                            key_start,
                        ));
                    }
                    Some(TomlTable::ArrayOfTables(_)) => {
                        return Err(Parser::static_err(
                            "cannot extend an array of tables with a dotted key",
                            key_start,
                        ));
                    }
                    None => {
                        tbl.entries
                            .insert(seg.clone(), TomlTable::Implicit(CowTable::new()));
                        if let Some(TomlTable::Implicit(inner)) = tbl.entries.get_mut(seg.as_str())
                        {
                            root_ptr = inner as *mut CowTable;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn register_header(
        &mut self,
        key: &[String],
        is_array: bool,
        comment: Option<String>,
        leading: Option<String>,
        blank_before: bool,
    ) -> Result<(), ParseError> {
        let joined = key.join(".");
        if !is_array && self.defined.contains(&joined) {
            return Err(self.err(&format!("duplicate table header `[{joined}]`")));
        }
        let mut root = &mut self.root as *mut CowTable;
        for (i, seg) in key.iter().enumerate() {
            let last = i + 1 == key.len();
            unsafe {
                let tbl = &mut *root;
                match tbl.entries.get_mut(seg.as_str()) {
                    Some(TomlTable::Explicit(inner)) => {
                        if last && !is_array {
                            return Err(self.err(&format!("duplicate key `{seg}`")));
                        }
                        root = inner as *mut CowTable;
                    }
                    Some(TomlTable::Implicit(inner)) => {
                        if last {
                            if is_array {
                                return Err(self.err("cannot redefine a table as array"));
                            }
                            // Snapshot the inner entries so we do not hold
                            // two mutable borrows of `tbl.entries` at once.
                            let taken = std::mem::take(&mut inner.entries);
                            tbl.entries.insert(
                                seg.clone(),
                                TomlTable::Explicit(CowTable {
                                    entries: taken,
                                    comment: comment.clone(),
                                    leading: leading.clone(),
                                    blank_before,
                                }),
                            );
                            self.current = key.to_vec();
                            self.defined.push(joined);
                            return Ok(());
                        }
                        root = inner as *mut CowTable;
                    }
                    Some(TomlTable::Value(..)) => {
                        return Err(self.err("cannot redefine a value as a table"));
                    }
                    Some(TomlTable::ArrayOfTables(list)) => {
                        if last {
                            if !is_array {
                                return Err(self.err("cannot redefine an array of tables"));
                            }
                            list.push(CowTable {
                                entries: IndexMap::new(),
                                comment: comment.clone(),
                                leading: leading.clone(),
                                blank_before,
                            });
                            let tail = list.last_mut().unwrap() as *mut CowTable;
                            root = tail;
                        } else {
                            let tail = list.last_mut().ok_or_else(|| {
                                Parser::static_err("empty array-of-tables path", 0)
                            })?;
                            root = tail as *mut CowTable;
                        }
                    }
                    None => {
                        if last {
                            if is_array {
                                let list = vec![CowTable {
                                    entries: IndexMap::new(),
                                    comment: comment.clone(),
                                    leading: leading.clone(),
                                    blank_before,
                                }];
                                tbl.entries
                                    .insert(seg.clone(), TomlTable::ArrayOfTables(list));
                                // current points to the newly pushed tail
                                self.current = key.to_vec();
                                self.defined.push(joined);
                                return Ok(());
                            } else {
                                tbl.entries.insert(
                                    seg.clone(),
                                    TomlTable::Explicit(CowTable {
                                        entries: IndexMap::new(),
                                        comment: comment.clone(),
                                        leading: leading.clone(),
                                        blank_before,
                                    }),
                                );
                                if let Some(TomlTable::Explicit(inner)) =
                                    tbl.entries.get_mut(seg.as_str())
                                {
                                    root = inner as *mut CowTable;
                                }
                            }
                        } else {
                            tbl.entries
                                .insert(seg.clone(), TomlTable::Implicit(CowTable::new()));
                            if let Some(TomlTable::Implicit(inner)) =
                                tbl.entries.get_mut(seg.as_str())
                            {
                                root = inner as *mut CowTable;
                            }
                        }
                    }
                }
            }
        }
        if !is_array {
            self.defined.push(joined.clone());
        }
        self.current = key.to_vec();
        Ok(())
    }

    fn static_err(msg: &str, byte_pos: usize) -> ParseError {
        ParseError::Syntax {
            message: format!("TOML parse error: {msg} at offset {byte_pos}"),
            line: 0,
            col: byte_pos,
        }
    }
}

fn strip_extra_quotes(content: &str, _multiline: bool) -> String {
    // A `'''` terminator may be preceded by up to 2 `'` that belong to
    // content; the caller advanced past them without adding to the string,
    // so this hook is intentionally a no-op today but kept for symmetry.
    content.to_string()
}

/// Lightweight validator the writer uses to guard tagged timestamps without
/// pulling in the full parser context. Grammar follows TOML 1.0:
/// `localDate`, `localTime`, `localDatetime` and offset `datetime`.
pub(crate) struct DateTimeProbe;

impl DateTimeProbe {
    pub(crate) fn check_all(&mut self, s: &str) -> bool {
        let bytes = s.as_bytes();
        if bytes.len() >= 10
            && bytes[4] == b'-'
            && bytes[7] == b'-'
            && (0..10).all(|i| {
                if i == 4 || i == 7 {
                    true
                } else {
                    bytes[i].is_ascii_digit()
                }
            })
        {
            // Date or date-time.
            if !validate_date(&s[0..10]) {
                return false;
            }
            if bytes.len() == 10 {
                return true;
            }
            match bytes[10] {
                b'T' | b't' | b' ' => {}
                _ => return false,
            }
            Self::check_time_and_offset(&s[11..])
        } else {
            // Local time only.
            Self::check_time_and_offset(s)
        }
    }

    fn check_time_and_offset(s: &str) -> bool {
        let b = s.as_bytes();
        // HH:MM is required in every dialect (TOML 1.0 and 1.1).
        if b.len() < 5
            || b[2] != b':'
            || !(0..5).all(|i| if i == 2 { true } else { b[i].is_ascii_digit() })
        {
            return false;
        }
        let h: u32 = s[0..2].parse().unwrap_or(99);
        let m: u32 = s[3..5].parse().unwrap_or(99);
        if h > 23 || m > 59 {
            return false;
        }
        // Seconds are optional under TOML 1.1 (PR #116 A4). If a `:`
        // follows the minute, we require the two-digit seconds pair.
        let mut idx = 5;
        if b.get(idx) == Some(&b':') {
            idx += 1;
            if !(b.len() > idx
                && b[idx].is_ascii_digit()
                && b.get(idx + 1).is_some_and(|c| c.is_ascii_digit()))
            {
                return false;
            }
            let sec: u32 = s[idx..idx + 2].parse().unwrap_or(99);
            if sec > 62 {
                return false;
            }
            idx += 2;
            if b.get(idx) == Some(&b'.') {
                idx += 1;
                let fs = idx;
                while b.get(idx).is_some_and(|c| c.is_ascii_digit()) {
                    idx += 1;
                }
                if idx == fs {
                    return false;
                }
            }
        } else if b.len() > idx {
            // No seconds component but extra trailing content (e.g.
            // `14:15:00` with an offset suffix but no seconds would
            // never be legal anyway).
            return false;
        }
        match b.get(idx) {
            None => true,
            Some(b'Z') | Some(b'z') => idx + 1 == b.len(),
            Some(b'+') | Some(b'-') => {
                b.len() == idx + 6
                    && b[idx + 1].is_ascii_digit()
                    && b[idx + 2].is_ascii_digit()
                    && b[idx + 3] == b':'
                    && b[idx + 4].is_ascii_digit()
                    && b[idx + 5].is_ascii_digit()
            }
            _ => false,
        }
    }
}

fn validate_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() < 10
        || b[4] != b'-'
        || b[7] != b'-'
        || !(0..10).all(|i| {
            if i == 4 || i == 7 {
                true
            } else {
                b[i].is_ascii_digit()
            }
        })
    {
        return false;
    }
    let year: i32 = s[0..4].parse().unwrap_or(0);
    let month: u32 = s[5..7].parse().unwrap_or(0);
    let day: u32 = s[8..10].parse().unwrap_or(0);
    if !(1..=12).contains(&month) {
        return false;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    day >= 1 && day <= max_day
}

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

fn negate_numeric(v: TomlValue) -> TomlValue {
    use std::sync::Arc;
    match v {
        TomlValue::Integer(i, Some(src)) => {
            // Radix-prefixed spellings (`0x1F`, `0o755`) do NOT accept
            // a leading `-` under YAML Core's numeric grammar; negating
            // them loses the fidelity fence, so we canonicalise to
            // decimal instead of emitting a Str-resolving plain scalar.
            let is_radix = src.starts_with("0x") || src.starts_with("0o") || src.starts_with("0b");
            if is_radix {
                return TomlValue::Integer(-i, None);
            }
            let flipped = if let Some(rest) = src.strip_prefix('-') {
                Arc::<str>::from(rest)
            } else if let Some(rest) = src.strip_prefix('+') {
                Arc::<str>::from(rest)
            } else {
                Arc::<str>::from(format!("-{src}"))
            };
            let new_source = if flipped.starts_with("-0") && flipped.len() == 2 {
                // negating literal `0` -> `-0` -> canonicalize to 0
                None
            } else {
                Some(flipped)
            };
            TomlValue::Integer(-i, new_source)
        }
        TomlValue::Integer(i, None) => TomlValue::Integer(-i, None),
        TomlValue::Float(f, Some(src)) => {
            let flipped = if let Some(rest) = src.strip_prefix('-') {
                Arc::<str>::from(rest)
            } else if let Some(rest) = src.strip_prefix('+') {
                Arc::<str>::from(rest)
            } else {
                Arc::<str>::from(format!("-{src}"))
            };
            TomlValue::Float(-f, Some(flipped))
        }
        TomlValue::Float(f, None) => TomlValue::Float(-f, None),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{CustomNode, ScalarStyle};

    fn parse_keys(node: &CustomNode) -> Vec<String> {
        let CustomNode::Mapping { pairs, .. } = node else {
            panic!("not a mapping");
        };
        pairs
            .iter()
            .map(|(k, _)| match k {
                CustomNode::Scalar { value, .. } => value.to_string(),
                _ => panic!("non-scalar key"),
            })
            .collect()
    }

    #[test]
    fn parses_basic_kv_and_tables() {
        let ast = from_toml("a = 1\nb = \"hi\"\n[tbl]\nc = true\nd = 3.14\n").unwrap();
        assert_eq!(parse_keys(&ast), vec!["a", "b", "tbl"]);
    }

    #[test]
    fn parses_array_of_tables() {
        let ast = from_toml("[[fruit]]\nname = \"apple\"\n[[fruit]]\nname = \"banana\"\n").unwrap();
        let CustomNode::Mapping { pairs, .. } = &ast else {
            unreachable!()
        };
        let fruit = pairs
            .iter()
            .find(|(k, _)| match k {
                CustomNode::Scalar { value, .. } => value.as_ref() == "fruit",
                _ => false,
            })
            .unwrap();
        let CustomNode::Sequence { items, .. } = &fruit.1 else {
            panic!("expected AOT sequence");
        };
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn parses_dotted_keys() {
        let ast = from_toml("a.b.c = 1\nx.y = 2\n").unwrap();
        assert_eq!(parse_keys(&ast), vec!["a", "x"]);
    }

    #[test]
    fn parses_inline_table() {
        let ast = from_toml("point = { x = 1, y = 2 }\n").unwrap();
        let CustomNode::Mapping { pairs, .. } = &ast else {
            unreachable!()
        };
        let v = pairs.iter().next().unwrap().1;
        assert!(matches!(
            v,
            CustomNode::Mapping {
                flow_style: true,
                ..
            }
        ));
    }

    #[test]
    fn parses_string_variants() {
        let src = "b = \"x\\ty\"\nl = 'x\\y'\nm = \"\"\"a\\\n b\"\"\"\nml = '''\nline1\nline2'''\n";
        let _ast = from_toml(src).unwrap();
    }

    #[test]
    fn rejects_duplicate_keys() {
        let e = from_toml("a = 1\na = 2\n").unwrap_err();
        let ParseError::Syntax { message, .. } = e else {
            panic!("expected Syntax");
        };
        assert!(message.contains("duplicate"), "{message}");
    }

    #[test]
    fn rejects_reopened_explicit_table() {
        let e = from_toml("[a]\n[a]\n").unwrap_err();
        assert!(format!("{e:?}").contains("duplicate table header"));
    }

    #[test]
    fn datetime_forms_carry_distinct_tags() {
        let ast = from_toml(
            "a = 1979-05-27T07:32:00Z\nb = 1979-05-27T07:32:00\nc = 1979-05-27\nd = 07:32:00\n",
        )
        .unwrap();
        let CustomNode::Mapping { pairs, .. } = &ast else {
            unreachable!()
        };
        let tags: Vec<String> = pairs
            .iter()
            .map(|(_, v)| v.tag().map(|t| t.suffix.clone()).unwrap_or_default())
            .collect();
        // Each temporal sub-shape carries the plugin tag it needs to decode:
        // offset / local date-times use `timestamp`, date-only uses `date`,
        // time-only uses `time` - routing them to `datetime`/`date`/`time`
        // `fromisoformat` respectively instead of crashing on a bare time.
        assert_eq!(tags, vec!["timestamp", "timestamp", "date", "time"]);
    }

    #[test]
    fn integers_in_every_radix() {
        let ast =
            from_toml("d = 42\nh = 0xDEADBEEF\no = 0o01234567\nb = 0b11010110\nu = 5_349_221\n")
                .unwrap();
        let CustomNode::Mapping { pairs, .. } = &ast else {
            unreachable!()
        };
        for (_, v) in pairs {
            assert!(matches!(
                v,
                CustomNode::Scalar {
                    style: ScalarStyle::Plain,
                    ..
                }
            ));
        }
    }

    #[test]
    fn floats_special_forms() {
        let _ast = from_toml("a = inf\nb = -inf\nc = nan\nd = 1e10\ne = -3.14e-2\n").unwrap();
    }

    // ------ TOML 1.1.0 (PR #116) ----------------------------------------------

    #[test]
    fn v1_1_accepts_multiline_inline_table() {
        // A1: inline tables may span lines when the parser runs under
        // V1_1. Members remain in insertion order and surface as a
        // nested Mapping just like their single-line siblings.
        let src = "tbl = {\n  a = 1,\n  b = 2,\n}\n";
        let ast = from_toml(src).unwrap();
        let CustomNode::Mapping { pairs, .. } = &ast else {
            unreachable!()
        };
        let (_, val) = pairs.iter().next().unwrap();
        let CustomNode::Mapping { pairs: inner, .. } = val else {
            panic!("expected inline mapping, got {val:?}")
        };
        assert_eq!(inner.len(), 2);
    }

    #[test]
    fn v1_1_accepts_trailing_comma_in_inline_table() {
        // A1: the trailing `,` before `}` is valid TOML 1.1.
        let src = "p = { x = 1, y = 2, }\n";
        let ast = from_toml(src).unwrap();
        let CustomNode::Mapping { pairs, .. } = &ast else {
            unreachable!()
        };
        assert_eq!(pairs.len(), 1);
    }

    #[test]
    fn v1_0_rejects_trailing_comma_in_inline_table() {
        // Dialect gate: the same source is a syntax error under V1_0.
        let src = "p = { x = 1, y = 2, }\n";
        let err = from_toml_v1_0(src).unwrap_err();
        assert!(
            matches!(&err, ParseError::Syntax { message, .. } if message.contains("TOML parse error")),
            "expected Syntax, got {err:?}"
        );
    }

    #[test]
    fn v1_1_accepts_byte_escape_x_hh() {
        // A2: `\xHH` covers codepoints 0x00..=0xFF. Two hex digits,
        // exactly one byte appended.
        let src = "null = \"nul:\\x00\"\nletter_a = \"a:\\x61\"\n";
        let ast = from_toml(src).unwrap();
        let CustomNode::Mapping { pairs, .. } = &ast else {
            unreachable!()
        };
        let mut seen_a = false;
        for (k, v) in pairs {
            if let CustomNode::Scalar { value, .. } = k
                && &**value == "letter_a"
            {
                let CustomNode::Scalar { value, .. } = v else {
                    unreachable!()
                };
                assert_eq!(&**value, "a:a");
                seen_a = true;
            }
        }
        assert!(seen_a, "letter_a pair not found");
    }

    #[test]
    fn v1_1_accepts_escape_e() {
        // A3: `\e` is U+001B (ESC). Commonly used with ANSI CSI
        // sequences (`"\e["`).
        let src = "csi = \"\\e[\"\n";
        let ast = from_toml(src).unwrap();
        let CustomNode::Mapping { pairs, .. } = &ast else {
            unreachable!()
        };
        let (_, v) = pairs.iter().next().unwrap();
        let CustomNode::Scalar { value, .. } = v else {
            unreachable!()
        };
        assert_eq!(&**value, "\u{1b}[");
    }

    #[test]
    fn v1_0_rejects_escape_x_and_e() {
        // Dialect gate: `\x41` and `\e` are unknown escapes in TOML
        // 1.0 and must fall through to the standard error path.
        for src in ["a = \"\\x41\"\n", "a = \"\\e\"\n"] {
            let err = from_toml_v1_0(src).unwrap_err();
            assert!(
                matches!(&err, ParseError::Syntax { message, .. } if message.contains("invalid escape")),
                "expected invalid-escape Syntax, got {err:?}"
            );
        }
    }

    #[test]
    fn v1_1_accepts_optional_seconds_in_time_and_datetime() {
        // A4: `HH:MM` is valid time / date-time under TOML 1.1.
        let src = "t = 14:15\ndt = 2010-02-03 14:15\n";
        let ast = from_toml(src).unwrap();
        let CustomNode::Mapping { pairs, .. } = &ast else {
            unreachable!()
        };
        assert_eq!(pairs.len(), 2);
        // Exact spellings ride through unchanged in the scalar text.
        let texts: Vec<String> = pairs
            .iter()
            .filter_map(|(_, v)| {
                if let CustomNode::Scalar { value, .. } = v {
                    Some(value.to_string())
                } else {
                    None
                }
            })
            .collect();
        assert!(texts.contains(&"14:15".to_string()), "{texts:?}");
        assert!(texts.contains(&"2010-02-03 14:15".to_string()), "{texts:?}");
    }

    #[test]
    fn v1_0_rejects_missing_seconds() {
        // Dialect gate: the same source without seconds is a syntax
        // error under V1_0.
        for src in ["t = 14:15\n", "dt = 2010-02-03 14:15\n"] {
            let err = from_toml_v1_0(src).unwrap_err();
            assert!(
                matches!(&err, ParseError::Syntax { .. }),
                "expected Syntax, got {err:?}"
            );
        }
    }
}
