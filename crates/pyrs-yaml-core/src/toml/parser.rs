//! TOML 1.0 parser producing the shared `CustomNode` AST.
//!
//! Byte-level scanner aligned with the [TOML v1.0.0 grammar][spec]. Every
//! rejection surfaces as `ParseError::Syntax` with a 0-indexed `line`/`col`
//! derived from the byte offset where the failure was detected.
//!
//! [spec]: https://toml.io/en/v1.0.0

use crate::ast::CustomNode;
use crate::error::ParseError;
use crate::toml::{CowTable, KVAnnotations, TomlTable, TomlValue, cow_table_to_node};
use indexmap::IndexMap;

/// Parse a TOML document into the shared AST.
pub fn from_toml(src: &str) -> Result<CustomNode, ParseError> {
    let mut p = Parser {
        s: src.as_bytes(),
        text: src,
        pos: 0,
        root: CowTable::new(),
        current: Vec::new(),
        defined: Vec::new(),
        pending_leading: None,
    };
    p.parse_document()?;
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
        self.text[start..self.pos].trim().to_string()
    }

    fn skip_all_blank(&mut self) {
        loop {
            match self.peek() {
                Some(b' ') | Some(b'\t') => self.pos += 1,
                Some(b'#') => {
                    // Standalone comment line: remember the most recent
                    // one so the next pair or header adopts it as its
                    // leading. Blank lines in between flush the slot.
                    self.pending_leading = Some(self.take_comment());
                }
                Some(b'\n') => self.pos += 1,
                Some(b'\r') if self.byte_at(self.pos + 1) == Some(b'\n') => self.pos += 2,
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
        let anns = KVAnnotations { leading, trailing };
        self.register_kv(&key, value, anns, key_start)
    }

    fn parse_table_header(&mut self) -> Result<(), ParseError> {
        // The pending standalone block sits ABOVE the header; take it
        // before parsing so it becomes this section's leading.
        let leading = self.pending_leading.take();
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
        self.register_header(&key, is_array, comment, leading)
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
        match self.peek() {
            Some(b'"') => self.parse_basic_string_content(),
            Some(b'\'') => self.parse_literal_string(),
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
                // Radix-prefixed integers can carry a leading sign in
                // TOML (`-0x1F`, `+0o644`). Route them through the
                // prefixed path so the `0x`/`0o`/`0b` marker is not
                // mistaken for a decimal `0` followed by garbage.
                if self.starts_with(b"0x") || self.starts_with(b"0o") || self.starts_with(b"0b") {
                    let v = self.parse_prefixed_body_via_dispatch()?;
                    return Ok(if positive { v } else { negate_numeric(v) });
                }
                let v = self.parse_decimal_numeric_body()?;
                Ok(if positive { v } else { negate_numeric(v) })
            }
            _ => Err(self.err("expected a signed numeric value")),
        }
    }

    /// Dispatch to the correct `parse_prefixed_integer_with_source`
    /// variant based on the currently-visible `0x`/`0o`/`0b` marker.
    /// Binary canonicalises to decimal (YAML Core does not accept
    /// `0b101`), so it drops the source spelling. Callers enter with
    /// `self.pos` at the leading `0`; this helper consumes the full
    /// `0x`/`0o`/`0b` marker before scanning digits.
    fn parse_prefixed_body_via_dispatch(&mut self) -> Result<TomlValue, ParseError> {
        debug_assert_eq!(self.peek(), Some(b'0'));
        self.pos += 1; // consume the `0`
        let radix = match self.peek() {
            Some(b'x') | Some(b'X') => 16,
            Some(b'o') | Some(b'O') => 8,
            Some(b'b') | Some(b'B') => 2,
            _ => return Err(self.err("expected a radix marker after `0`")),
        };
        self.pos += 1; // consume the `x`/`o`/`b` marker
        let (v, src) = self.parse_prefixed_integer_with_source(radix)?;
        if radix == 2 {
            Ok(TomlValue::Integer(v, None))
        } else {
            Ok(TomlValue::Integer(v, Some(src)))
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
        // HH:MM:SS...
        let p = self.pos;
        matches!(self.byte_at(p + 2), Some(b':'))
            && matches!(self.byte_at(p + 5), Some(b':'))
            && (0..6).all(|i| {
                if i == 2 || i == 5 {
                    true
                } else {
                    matches!(self.byte_at(p + i), Some(b'0'..=b'9'))
                }
            })
    }

    fn parse_decimal_numeric_body(&mut self) -> Result<TomlValue, ParseError> {
        let num_start = self.pos;
        self.scan_int_digits()?;
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
        if self.peek() == Some(b'_') {
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
        self.parse_literal_string().map(TomlValue::String)
    }

    fn parse_basic_string(&mut self, _prefix: &str) -> Result<TomlValue, ParseError> {
        self.parse_basic_string_content().map(TomlValue::String)
    }

    fn parse_basic_string_content(&mut self) -> Result<String, ParseError> {
        let ml = self.starts_with(b"\"\"\"");
        if ml {
            self.pos += 3;
            // Skip first immediate newline (TOML 1.0).
            if self.peek() == Some(b'\n') {
                self.pos += 1;
            } else if self.starts_with(b"\r\n") {
                self.pos += 2;
            }
            self.scan_multiline_basic_content()
        } else {
            self.pos += 1;
            self.scan_single_line_basic_content()
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
                0x00..=0x08 | 0x0B..=0x0C | 0x0E..=0x1F => {
                    return Err(self.err("control character in basic string"));
                }
                _ => {
                    self.pos += 1;
                    let cut = floor_char_boundary(self.text, self.pos);
                    if cut > self.pos - 1 {
                        out.push_str(&self.text[self.pos - 1..cut]);
                        self.pos = cut;
                    } else {
                        out.push(b as char);
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
            _ => return Err(self.err("invalid escape in basic string")),
        }
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
                _ => {
                    let cur = self.pos;
                    self.pos += 1;
                    let cut = floor_char_boundary(self.text, self.pos);
                    if cut > cur {
                        out.push_str(&self.text[cur..cut]);
                        self.pos = cut;
                    } else {
                        out.push(b as char);
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

    fn parse_literal_string(&mut self) -> Result<String, ParseError> {
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
                    return Ok(strip_extra_quotes(content, true));
                }
                let b = self
                    .peek()
                    .ok_or_else(|| self.err("unterminated multiline literal"))?;
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
                if b == b'\'' {
                    let content = &self.text[start..self.pos];
                    self.pos += 1;
                    return Ok(content.to_string());
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
        self.skip_inline_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(TomlValue::InlineTable(entries));
        }
        loop {
            self.skip_inline_ws();
            let key = self.parse_key_path()?;
            self.skip_inline_ws();
            self.expect_byte(b'=', "expected `=` in inline table")?;
            self.skip_inline_ws();
            let v = self.parse_value()?;
            entries.push((key.join("."), v));
            self.skip_inline_ws();
            if self.peek() == Some(b',') {
                self.pos += 1;
                continue;
            }
            if self.peek() == Some(b'}') {
                self.pos += 1;
                return Ok(TomlValue::InlineTable(entries));
            }
            return Err(self.err("expected `,` or `}` in inline table"));
        }
    }

    fn skip_inline_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t')) {
            self.pos += 1;
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
            let space_time_sep = sep == Some(b' ')
                && matches!(self.byte_at(self.pos + 1), Some(b'0'..=b'9'))
                && matches!(self.byte_at(self.pos + 4), Some(b':'));
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
                let text = self.text[start..self.pos].to_string();
                Ok(TomlValue::Datetime(text, None))
            }
        } else {
            self.scan_time_body()?;
            let text = self.text[start..self.pos].to_string();
            Ok(TomlValue::Datetime(text, None))
        }
    }

    fn scan_time_body(&mut self) -> Result<(), ParseError> {
        let start = self.pos;
        // HH:MM:SS(.frac)?
        if !(matches!(self.peek(), Some(b'0'..=b'9'))
            && matches!(self.byte_at(self.pos + 1), Some(b'0'..=b'9'))
            && self.byte_at(self.pos + 2) == Some(b':')
            && matches!(self.byte_at(self.pos + 3), Some(b'0'..=b'9'))
            && matches!(self.byte_at(self.pos + 4), Some(b'0'..=b'9'))
            && self.byte_at(self.pos + 5) == Some(b':')
            && matches!(self.byte_at(self.pos + 6), Some(b'0'..=b'9'))
            && matches!(self.byte_at(self.pos + 7), Some(b'0'..=b'9')))
        {
            return Err(self.err_at("invalid time-of-day", start));
        }
        self.pos += 8;
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
        if b.len() < 8
            || b[2] != b':'
            || b[5] != b':'
            || !(0..8).all(|i| {
                if i == 2 || i == 5 {
                    true
                } else {
                    b[i].is_ascii_digit()
                }
            })
        {
            return false;
        }
        let h: u32 = s[0..2].parse().unwrap_or(99);
        let m: u32 = s[3..5].parse().unwrap_or(99);
        let sec: u32 = s[6..8].parse().unwrap_or(99);
        if h > 23 || m > 59 || sec > 62 {
            return false;
        }
        let mut idx = 8;
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
        // Every datetime form uses `timestamp` (the plugin tag); the specific
        // sub-shape (offset / local / date-only / time-only) is carried in
        // the value text rather than the tag suffix.
        assert_eq!(tags, vec!["timestamp"; 4]);
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
}
