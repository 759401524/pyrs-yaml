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

use crate::{CowTable, KVAnnotations, Note, TomlTable, TomlValue, cow_table_to_node};
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use pyrs_ast::ast::CustomNode;
use pyrs_ast::ast::NodeMap;
use pyrs_ast::error::{DepthError, ParseError};

/// Default nesting limit for arrays / inline tables, mirroring the JSON
/// pipeline's `DEFAULT_MAX_DEPTH` (`pyrs-json`).
/// TOML's `parse_value` is the single recursion funnel (arrays and inline
/// tables both re-enter through it), so bounding it here caps the whole
/// descent. Without it a deeply nested `[[[[…]]]]` / `{a={b={c=…}}}` input
/// recursed until the native stack overflowed and aborted the process
/// (verified: 5000-deep array → `0xC00000FD`), the TOML analogue of the
/// #166 YAML merge stack overflow.
pub const DEFAULT_MAX_DEPTH: usize = 1000;

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
    let (root, leftover) = parse_ir(src, dialect)?;
    let mut node = cow_table_to_node(root);
    // A comment-only document has no key/table to consume the trailing
    // standalone note, so `pending_leading` would be dropped and the writer
    // fixed point (`# …\n` → re-parse → `` → ``) drifts to empty. When the
    // root is otherwise empty, attach the leftover note to it so the sole
    // `# …` line survives the round trip. Non-empty documents keep their
    // notes on keys and leave document-trailing comments unattributed (the
    // writer never emits those, so their fixed point is unaffected).
    if matches!(&node, CustomNode::Mapping { pairs, .. } if pairs.is_empty()) {
        for note in leftover {
            node.push_leading_comment(note.into_comment(true));
        }
    }
    Ok(node)
}

/// Run the scanner to the engine's own intermediate representation.
///
/// Both readers of a document start here: the AST projection above, and the event stream in
/// `events`, which reports the same IR with the positions it carries. That is why the two routes
/// cannot disagree about what is valid - one scanner decides, twice it is described - and it is the
/// reason TOML's producer is not the second walk `pyrs-json` needed: the table rules below
/// (the reopen marker, the array-of-tables tails) are heavy enough that re-deriving them in a
/// second pass would be two authored policies, which is what `docs/dev/event-stream.md` refuses.
///
/// The second value is every own-line note no key or header claimed; the AST has no slot for those
/// in a non-empty document, the event stream reports them where they sit.
pub(crate) fn parse_ir(
    src: &str,
    dialect: TomlDialect,
) -> Result<(CowTable, Vec<Note>), ParseError> {
    let mut p = Parser {
        s: src.as_bytes(),
        text: src,
        pos: 0,
        depth: 0,
        root: CowTable::new(0, 0),
        current: Vec::new(),
        pending_leading: Vec::new(),
        pending_blank: false,
        dialect,
        pending_inline_comment: Vec::new(),
        pending_comment_err: None,
    };
    // TOML forbids a bare carriage return: every 0x0D must be part of a 0x0D0A
    // pair, in every context (line endings, comments, and inside strings - an
    // escaped `\r` is the two bytes 5C 72, never a 0x0D). The per-path scanners
    // handled CR inconsistently, so toml-test's invalid/control corpus
    // (bare-cr / comment-cr / multi-cr / rawmulti-cr) leaked lone CRs. One tight
    // central scan catches all of them; CR is rare so this is a cheap byte walk.
    {
        let s = p.s;
        let mut i = 0usize;
        let mut bad: Option<usize> = None;
        while i < s.len() {
            if s[i] == b'\r' {
                if i + 1 >= s.len() || s[i + 1] != b'\n' {
                    bad = Some(i);
                    break;
                }
                i += 2;
            } else {
                i += 1;
            }
        }
        if let Some(off) = bad {
            return Err(p.err_at("bare carriage return (must be part of a CRLF pair)", off));
        }
    }
    p.parse_document()?;
    // Comments are scanned by the infallible `take_comment` (several callers live
    // in non-Result helpers), so a forbidden control byte is accumulated as a
    // sticky error and surfaced here once the rest of the document parses.
    if let Some(e) = p.pending_comment_err.take() {
        return Err(e);
    }
    Ok((p.root, core::mem::take(&mut p.pending_leading)))
}

struct Parser<'a> {
    s: &'a [u8],
    text: &'a str,
    pos: usize,
    /// Current `parse_value` recursion depth, gated against
    /// [`DEFAULT_MAX_DEPTH`] to reject (not crash) over-nested input.
    depth: usize,
    root: CowTable,
    /// Path of the active table (set by `[a.b]` or `[[a.b]]` headers).
    current: Vec<String>,
    /// The standalone `# ...` comments seen on lines of their own since the last
    /// pair or header, in source order. All of them are claimed by the next KV
    /// pair or table header as its leading notes: a stack of comment lines above a
    /// key is ordinary input, and the `Option<String>` this replaces overwrote all
    /// but the last — a silent loss the writer could never show, because the
    /// shortened output round-tripped perfectly.
    pending_leading: Vec<Note>,
    /// True once `skip_all_blank` has crossed at least one blank line
    /// since the last real pair / header / EOF. Claimed by the next pair
    /// or header as its `blank_before` hint so the writer can reproduce
    /// the visual grouping.
    pending_blank: bool,
    /// Grammar dialect gate for A1 / A2 / A3 / A4 (PR #116).
    dialect: TomlDialect,
    /// `# ...` comments seen inside a multi-line inline table on a line of their
    /// own, in source order. All of them are claimed by the next member as its
    /// leading notes (PR #119 interior-comment fidelity, list-ified with the
    /// stacked-comment fix).
    pending_inline_comment: Vec<Note>,
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

    fn take_comment(&mut self) -> Note {
        let hash = self.pos;
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
        Note {
            text: self.text[start..self.pos].trim().to_string(),
            at: hash,
        }
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
                    // Standalone comment line: append it to the stack the next
                    // pair or header adopts as its leading notes. A comment runs to
                    // end-of-line, so the
                    // newline that terminates it is the comment's own
                    // break, NOT a blank separator. Eat it here and
                    // reset the tally so only genuinely empty lines
                    // after the comment count toward `pending_blank`.
                    // (PR #119 caught the latent #114 bug that omitted
                    // this: promoting an inline table to a `[section]`
                    // feeds `key = v\n# lead\nkey = v` back through this
                    // parser, and the comment's terminator newline was
                    // being miscounted as a blank line.)
                    let note = self.take_comment();
                    self.pending_leading.push(note);
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
    fn finish_line(&mut self) -> Result<Option<Note>, ParseError> {
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
        // Take ownership of the pending standalone comment stack (if any)
        // so it becomes THIS pair's leading rather than the next one's.
        let leading = core::mem::take(&mut self.pending_leading);
        let blank = core::mem::take(&mut self.pending_blank);
        let key_start = self.pos;
        let key = self.parse_key_path()?;
        self.skip_spaces();
        if self.peek() != Some(b'=') {
            return Err(self.err_at("expected `=` after key", self.pos));
        }
        self.pos += 1;
        self.skip_spaces();
        let value_at = self.pos;
        let value = self.parse_value()?;
        let trailing = self.finish_line()?;
        let anns = KVAnnotations {
            leading,
            trailing,
            blank_before: blank,
            key_at: key_start,
            value_at,
        };
        self.register_kv(&key, value, anns, key_start)
    }

    fn parse_table_header(&mut self) -> Result<(), ParseError> {
        // The pending standalone stack sits ABOVE the header; take it
        // before parsing so it becomes this section's leading. `pending_blank`
        // travels through the same route to mark the section's blank separator.
        let leading = core::mem::take(&mut self.pending_leading);
        let blank = core::mem::take(&mut self.pending_blank);
        let header_at = self.pos;
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
        self.register_header(&key, is_array, comment, leading, blank, header_at)
    }

    // ------ keys -------------------------------------------------------------

    /// One key path as the document wrote it: every segment paired with the offset of its first
    /// byte, because a header line names a whole path and a stream that reports only where the line
    /// begins cannot put its events in source order.
    fn parse_key_path(&mut self) -> Result<Vec<(String, usize)>, ParseError> {
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

    fn parse_key_segment(&mut self) -> Result<(String, usize), ParseError> {
        // Keys are never multi-line; the `multiline` flag from the string
        // scanners is discarded here.
        let at = self.pos;
        let name = match self.peek() {
            Some(b'"') => self.parse_basic_string_content().map(|(s, _)| s)?,
            Some(b'\'') => self.parse_literal_string().map(|(s, _)| s)?,
            _ => self.parse_bare_key()?,
        };
        Ok((name, at))
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
        // Single recursion funnel: arrays (`parse_string_or_array`) and
        // inline tables (`parse_inline_table`) both re-enter `parse_value`
        // for their elements, so bounding it here caps every nesting path.
        self.depth += 1;
        if self.depth > DEFAULT_MAX_DEPTH {
            self.depth -= 1;
            return Err(ParseError::MaxDepthExceeded(DepthError(DEFAULT_MAX_DEPTH)));
        }
        let out = self.parse_value_inner();
        self.depth -= 1;
        out
    }

    fn parse_value_inner(&mut self) -> Result<TomlValue, ParseError> {
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
                let v = self.parse_decimal_numeric_body(!positive)?;
                Ok(v)
            }
            _ => Err(self.err("expected a signed numeric value")),
        }
    }

    fn parse_unsigned_numeric(&mut self) -> Result<TomlValue, ParseError> {
        // Prefixed integers: 0x / 0o / 0b (TOML 1.0 disallows 0x0x-style
        // leading zero after the prefix). Radix-prefixed spellings are kept
        // verbatim; the two YAML Core also reads back as integers need no tag,
        // and the one it does not (`0b`) is tagged on projection.
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
            // The spelling is kept, and `toml_value_to_node` tags it `!!int`:
            // YAML Core does not recognise a binary literal, so an untagged
            // plain scalar would re-resolve to a string. Canonicalising to
            // decimal was the alternative the spoke took while its writers read
            // only text; since (bu) a standard tag decides the type in every
            // format, so the spelling can stay where it was written.
            let (v, src) = self.parse_prefixed_integer_with_source(2)?;
            return Ok(TomlValue::Integer(v, Some(src)));
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
        self.parse_decimal_numeric_body(false)
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

    fn parse_decimal_numeric_body(&mut self, negative: bool) -> Result<TomlValue, ParseError> {
        // The sign travels with the digits on purpose: TOML's i64 range is
        // asymmetric (`-2^63 ..= 2^63-1`), so a magnitude-then-negate flow
        // rejects or overflows the legal minimum `-9223372036854775808`
        // (magnitude 2^63 does not fit `i64`). `i64::from_str` accumulates
        // negatively and accepts the signed spelling directly.
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
            return self.emit_float(num_start, negative);
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.scan_optional_exponent(true, num_start)?;
            return self.emit_float(num_start, negative);
        }
        // Emit integer if no float marker. The source text carries the
        // digits (plus any leading `-`/`+` handled upstream).
        let text = &self.text[num_start..self.pos];
        let cleaned = text.replace('_', "");
        let spelling = if negative {
            format!("-{cleaned}")
        } else {
            cleaned
        };
        let value: i64 = spelling
            .parse()
            .map_err(|_| self.err_at("integer out of range or malformed", num_start))?;
        // Preserve `-0` verbatim because YAML Core reads `-0` back as
        // `Int(0)` and we can round-trip the sign; every other decimal
        // form either matches `i64::to_string()` (no work needed) or
        // contains `_`/`+` which we canonicalize away.
        let source: Option<alloc::sync::Arc<str>> = if text == "-0" {
            Some(alloc::sync::Arc::from(text))
        } else {
            None
        };
        Ok(TomlValue::Integer(value, source))
    }

    fn emit_float(&mut self, num_start: usize, negative: bool) -> Result<TomlValue, ParseError> {
        let text = &self.text[num_start..self.pos];
        let cleaned = text.replace('_', "");
        let magnitude: f64 = cleaned
            .parse()
            .map_err(|_| self.err_at("malformed float", num_start))?;
        let value = if negative { -magnitude } else { magnitude };
        // Preserve exponent spelling (`1e10`, `-3.14e-2`) verbatim: YAML
        // Core reads them back as Float, so the projected plain scalar
        // carries no tag and stays fully interoperable with the YAML
        // pipeline. Canonical decimal forms (`1.5`, `-0.0`) already
        // match the parsed value's rendering, so no source is recorded.
        let has_exponent = text.bytes().any(|b| b == b'e' || b == b'E');
        let source: Option<alloc::sync::Arc<str>> = if has_exponent && !text.contains('_') {
            Some(if negative {
                alloc::sync::Arc::from(format!("-{text}"))
            } else {
                alloc::sync::Arc::from(text)
            })
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
    ) -> Result<(i64, alloc::sync::Arc<str>), ParseError> {
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
                // separators: neither the hub's plain scalar nor the writer's
                // TOML-literal gate accepts them, so an underscored spelling
                // could only ever canonicalise — strip it here and keep the
                // `to_yaml -> load_toml -> to_toml` text continuous.
                let digits_no_underscores = raw.replace('_', "");
                let mut src = String::with_capacity(prefix_str.len() + digits_no_underscores.len());
                src.push_str(prefix_str);
                src.push_str(&digits_no_underscores);
                (v, alloc::sync::Arc::from(src))
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
            // Bulk-copy the source run up to the next byte that needs a decision. Every byte in
            // that set is ASCII, and UTF-8 never hides an ASCII byte inside a multi-byte sequence,
            // so the run begins and ends on a char boundary and `push_str` of a slice replaces a
            // loop that decoded and pushed one code point at a time. Stepping by whole bytes, or
            // stopping on a non-ASCII one, would put `self.pos` inside a character and make the
            // next slice panic.
            let start = self.pos;
            while self.pos < self.s.len() {
                let b = self.s[self.pos];
                if b == b'"' || b == b'\\' || b == b'\n' || b == b'\r' || Self::forbidden_control(b)
                {
                    break;
                }
                self.pos += 1;
            }
            if self.pos > start {
                out.push_str(&self.text[start..self.pos]);
            }
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
                // The scan above stops on exactly these four cases, so a byte that is none of
                // them cannot be what `peek` just returned.
                _ => unreachable!(),
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
        // Slice the byte buffer, not `self.text` (see json `hex4`): a `\x`
        // escape followed by a multibyte char would make
        // `&self.text[pos..pos+2]` land on a non-char boundary and panic.
        let digits = core::str::from_utf8(&self.s[self.pos..self.pos + 2])
            .map_err(|_| self.err("invalid hex in \\xHH escape"))?;
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
        // Byte-slice + UTF-8 validate (see json `hex4`): a `\uXXXX` / `\UXXXXXXXX`
        // escape followed by a multibyte char would otherwise slice
        // `&self.text[pos..pos+width]` across a char boundary and panic.
        let digits = core::str::from_utf8(&self.s[self.pos..self.pos + width])
            .map_err(|_| self.err("invalid hex in unicode escape"))?;
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
                    // Line-ending backslash: trim trailing ws to a newline. An escaped one is a
                    // content character and ends no line, so the test must not look past it:
                    // `"""a \\` + break + `b"""` is `a \` + break + `b`, and reading the
                    // continuation here swallowed the break the string had asked to keep.
                    let save = self.pos;
                    self.pos += 1;
                    if self.peek() != Some(b'\\')
                        && (matches!(self.peek(), Some(b'\n') | Some(b'\r'))
                            || self.is_at_line_end_ws())
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
                    // The closing delimiter is the LAST three quotes of the run. A literal
                    // multi-line string whose content ends in `'` is written with one or two
                    // extra quotes ahead of it — `''''one quote''''` is `'one quote'` — and those
                    // belong to the content. Slicing before the first match and then stepping over
                    // the rest dropped exactly that tail, which is the gap `strip_extra_quotes` was
                    // left documenting as "intentionally a no-op today".
                    let mut end = self.pos + 3;
                    while self.byte_at(end) == Some(b'\'') {
                        end += 1;
                    }
                    self.pos = end;
                    return Ok((self.text[start..end - 3].to_string(), true));
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
        let mut items: Vec<(TomlValue, KVAnnotations)> = Vec::new();
        loop {
            self.skip_array_ws();
            if self.peek() == Some(b']') {
                self.pos += 1;
                return Ok(TomlValue::Array(items));
            }
            if self.eof() {
                return Err(self.err("unterminated array"));
            }
            // Own-line comments collected before the element are its leading notes - the same
            // attribution `parse_inline_table` gives a member, because `#` runs to end of line and
            // the reader can only report a comment from the line that ends it.
            let leading = core::mem::take(&mut self.pending_inline_comment);
            let value_at = self.pos;
            let v = self.parse_value()?;
            let trailing = self.same_line_comment();
            items.push((
                v,
                KVAnnotations {
                    leading,
                    trailing,
                    blank_before: false,
                    key_at: value_at,
                    value_at,
                },
            ));
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

    /// A `#` comment still on the current line, without crossing a newline: the one that belongs to
    /// the value just parsed rather than to the element that follows it.
    fn same_line_comment(&mut self) -> Option<Note> {
        let mut at = self.pos;
        while matches!(self.byte_at(at), Some(b' ') | Some(b'\t')) {
            at += 1;
        }
        if self.byte_at(at) != Some(b'#') {
            return None;
        }
        self.pos = at;
        Some(self.take_comment())
    }

    fn skip_array_ws(&mut self) {
        loop {
            match self.peek() {
                Some(b' ') | Some(b'\t') | Some(b'\n') => self.pos += 1,
                Some(b'\r') if self.byte_at(self.pos + 1) == Some(b'\n') => self.pos += 2,
                Some(b'#') => {
                    // Collected rather than discarded: an array element's note has to reach the AST
                    // for `to_toml` to put it back, which is what #119 did for inline-table members
                    // and #434 for a member's key line. A comment left over when `]` arrives still
                    // has no owner - see the pinned boundary in `writer::tests`.
                    let note = self.take_comment();
                    self.pending_inline_comment.push(note);
                }
                _ => return,
            }
        }
    }

    fn parse_inline_table(&mut self) -> Result<TomlValue, ParseError> {
        self.expect_byte(b'{', "expected `{`")?;
        // Members are held as the paths that wrote them; `nest_inline` lays them into the tree
        // those paths name once the whole `{ ... }` is read.
        let mut entries: Vec<InlineMember> = Vec::new();
        // Segment paths already defined in this inline table, to detect dotted
        // key collisions (`a` vs `a.b`, `a.b` vs `a.b.c`) per TOML rules.
        let mut defined_paths: Vec<Vec<String>> = Vec::new();
        self.skip_inline_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(TomlValue::InlineTable(Vec::new()));
        }
        loop {
            self.skip_inline_ws();
            // A `# ...` on its own line above this member (captured by
            // `skip_inline_ws` into `pending_inline_comment`) is the
            // member's leading note. PR #119 plumbs it through the IR
            // so `to_toml` can reproduce interior comments.
            let leading = core::mem::take(&mut self.pending_inline_comment);
            let key_at = self.pos;
            let key = self.parse_key_path()?;
            self.skip_inline_ws();
            self.expect_byte(b'=', "expected `=` in inline table")?;
            self.skip_inline_ws();
            let value_at = self.pos;
            let v = self.parse_value()?;
            let trailing = self.take_inline_trailing_comment();
            let anns = KVAnnotations {
                leading,
                trailing,
                blank_before: false,
                key_at,
                value_at,
            };
            // toml-test strictness: inline tables reject a key that collides
            // with any already-defined path - equal, or one a prefix of the
            // other at segment boundaries (`a` then `a.b`, or `a.b` then
            // `a.b.c`). Valid sibling dotted keys (`{ a.b = 1, a.c = 2 }`) do
            // not collide because neither path is a prefix of the other.
            let named = names(&key);
            let conflict = defined_paths
                .iter()
                .any(|q| q[..q.len().min(named.len())] == named[..q.len().min(named.len())]);
            if conflict {
                return Err(self.err(&format!(
                    "duplicate key `{}` in inline table",
                    named.join(".")
                )));
            }
            defined_paths.push(named);
            entries.push((key, v, anns));
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
                        return Ok(TomlValue::InlineTable(nest_inline(entries)?));
                    }
                }
                continue;
            }
            if self.peek() == Some(b'}') {
                self.pos += 1;
                return Ok(TomlValue::InlineTable(nest_inline(entries)?));
            }
            return Err(self.err("expected `,` or `}` in inline table"));
        }
    }

    /// Capture a `# ...` comment that sits on the same line as the
    /// member value (i.e. only spaces/tabs between the value and the
    /// `#`). Returns `None` once a newline or the `,` / `}` separator
    /// is reached, so an own-line comment stays pending for the next
    /// member's leading slot rather than being misfiled as trailing.
    fn take_inline_trailing_comment(&mut self) -> Option<Note> {
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
                    // PR #119: collect the own-line comments so the next member
                    // adopts all of them as its leading notes.
                    let note = self.take_comment();
                    self.pending_inline_comment.push(note);
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
                        // Shape is valid; now range-check. TOML numeric offset
                        // hours are 00..=23 and minutes 00..=59 (toml-test
                        // invalid/datetime/offset-overflow-minute: `+12:60`).
                        let off_h = (self.s[self.pos] - b'0') * 10 + (self.s[self.pos + 1] - b'0');
                        let off_m =
                            (self.s[self.pos + 3] - b'0') * 10 + (self.s[self.pos + 4] - b'0');
                        if off_h > 23 || off_m > 59 {
                            return Err(self.err("offset out of range in date-time"));
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
        key: &[(String, usize)],
        value: TomlValue,
        mut anns: KVAnnotations,
        key_start: usize,
    ) -> Result<(), ParseError> {
        // A dotted path names one key at its last segment, and that segment's offset is where the
        // name the AST stores actually begins.
        if let Some((_, at)) = key.last() {
            anns.key_at = *at;
        }
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
        for (i, (seg, seg_at)) in key.iter().enumerate() {
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
                        // A dotted key creates intermediate tables that are CLOSED: TOML forbids
                        // re-opening them with a later `[header]`. The marker sits on the table, so
                        // no path text is registered anywhere - which is what keeps a dotted name
                        // (`a."b.c"`) apart from the deeper path `a.b.c`, and one array element's
                        // tables apart from its sibling's.
                        tbl.entries.insert(
                            seg.clone(),
                            TomlTable::Implicit(CowTable::new_closed(key_start, *seg_at)),
                        );
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
        key: &[(String, usize)],
        is_array: bool,
        comment: Option<Note>,
        leading: Vec<Note>,
        blank_before: bool,
        at: usize,
    ) -> Result<(), ParseError> {
        // Both rules this function enforces - a header that reopens a table another header defined,
        // and one that reopens a table a dotted key created - are read off the tree below, where each
        // `CowTable` is one table and so already carries its own identity. Keying them by the
        // dot-joined path text instead made `[a."b.c"]` a redefinition of `[a.b.c]`, and gave every
        // element of `[[a.b]]` the first element's identity, which is what refused four valid
        // documents of the official corpus. The text is built only where a rejection quotes it.
        let joined = || key_text(key);
        let mut root = &mut self.root as *mut CowTable;
        for (i, (seg, seg_at)) in key.iter().enumerate() {
            let last = i + 1 == key.len();
            unsafe {
                let tbl = &mut *root;
                match tbl.entries.get_mut(seg.as_str()) {
                    Some(TomlTable::Explicit(inner)) => {
                        if last {
                            if is_array {
                                return Err(
                                    self.err("cannot redefine a table as an array of tables")
                                );
                            }
                            return Err(
                                self.err(&format!("duplicate table header `[{}]`", joined()))
                            );
                        }
                        root = inner as *mut CowTable;
                    }
                    Some(TomlTable::Implicit(inner)) => {
                        if last {
                            if is_array {
                                return Err(self.err("cannot redefine a table as array"));
                            }
                            // A dotted key's table stays closed to a header that ends on it
                            // (toml-test `invalid/table/duplicate-key-*`, `redefine-*`). Two
                            // neighbouring cases are not this one and stay legal: passing through
                            // such a table to define a deeper one, which is
                            // `[fruit.apple.texture]` after `apple.color`, and opening a super-table
                            // a header only implied, which is `valid/table/without-super.toml`.
                            if inner.closed {
                                return Err(self.err(&format!(
                                    "cannot redefine table `[{}]` created by a dotted key",
                                    joined()
                                )));
                            }
                            // Snapshot the inner entries so we do not hold
                            // two mutable borrows of `tbl.entries` at once.
                            let taken = core::mem::take(&mut inner.entries);
                            tbl.entries.insert(
                                seg.clone(),
                                TomlTable::Explicit(CowTable {
                                    entries: taken,
                                    comment: comment.clone(),
                                    leading: leading.clone(),
                                    blank_before,
                                    at,
                                    key_at: *seg_at,
                                    closed: false,
                                }),
                            );
                            self.current = names(key);
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
                                entries: NodeMap::default(),
                                comment: comment.clone(),
                                leading: leading.clone(),
                                blank_before,
                                at,
                                key_at: *seg_at,
                                closed: false,
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
                                    entries: NodeMap::default(),
                                    comment: comment.clone(),
                                    leading: leading.clone(),
                                    blank_before,
                                    at,
                                    key_at: *seg_at,
                                    closed: false,
                                }];
                                tbl.entries
                                    .insert(seg.clone(), TomlTable::ArrayOfTables(list));
                                // current points to the newly pushed tail.
                                self.current = names(key);
                                return Ok(());
                            } else {
                                tbl.entries.insert(
                                    seg.clone(),
                                    TomlTable::Explicit(CowTable {
                                        entries: NodeMap::default(),
                                        comment: comment.clone(),
                                        leading: leading.clone(),
                                        blank_before,
                                        at,
                                        key_at: *seg_at,
                                        closed: false,
                                    }),
                                );
                                if let Some(TomlTable::Explicit(inner)) =
                                    tbl.entries.get_mut(seg.as_str())
                                {
                                    root = inner as *mut CowTable;
                                }
                            }
                        } else {
                            tbl.entries.insert(
                                seg.clone(),
                                TomlTable::Implicit(CowTable::new(at, *seg_at)),
                            );
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
        self.current = names(key);
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

/// The names of a parsed key path, for the messages and the table bookkeeping that address a
/// table by its dotted text rather than by where each segment sits.
fn names(key: &[(String, usize)]) -> Vec<String> {
    key.iter().map(|(name, _)| name.clone()).collect()
}

/// The same names as the dot-joined text a duplicate-key rejection quotes back.
fn key_text(key: &[(String, usize)]) -> String {
    names(key).join(".")
}

/// One member of an inline table exactly as the source wrote it: the key path it named (every
/// segment with its own offset), its value, and the notes and offsets that belong to the member.
type InlineMember = (Vec<(String, usize)>, TomlValue, KVAnnotations);
/// The entries an inline table holds after its members' paths are laid down: a name, a value, and
/// the annotations of the member that wrote it. This is the shape `TomlValue::InlineTable` owns.
type InlineEntries = Vec<(String, TomlValue, KVAnnotations)>;

/// Lay an inline table's members into the tree their key paths name.
///
/// A dotted key inside `{ ... }` is a path, not a name that happens to contain a dot: the spec's own
/// `animal = { type.name = "pug" }` means `type` → `name`, and two members that share a prefix
/// (`{ a.b = 1, a.c = 2 }`) share the one table that prefix names. Storing the dot-joined text as a
/// single key instead collapsed `a = { x.y = 1 }` and `a = { "x.y" = 1 }` — documents with different
/// data — onto one shape, and that is what cost the toml-test `inline-table/key-dotted-*` group. The
/// traversal mirrors `register_kv`, which already nests a dotted key at table level.
///
/// An intermediate level exists only because a path created it, so it reports the offset of the
/// segment that names it and begins its value at the next segment: the first byte the source really
/// wrote inside it. `parse_inline_table` has already rejected a member that extends a path an earlier
/// member ended on, so a name found here that is not a table is reported rather than overwritten.
fn nest_inline(members: Vec<InlineMember>) -> Result<InlineEntries, ParseError> {
    let mut out: InlineEntries = Vec::new();
    for (path, value, mut anns) in members {
        let (leaf, leaf_at) = match path.last() {
            Some((name, at)) => (name.clone(), *at),
            None => {
                return Err(Parser::static_err(
                    "empty key path in inline table",
                    anns.key_at,
                ));
            }
        };
        // The name the AST stores is the last segment, and its offset is where that name begins —
        // the same rule `register_kv` applies to a dotted key outside an inline table.
        anns.key_at = leaf_at;
        // Mutable-pointer traversal, for the reason `register_kv` names: a `&mut` reborrow of the
        // nested `Vec` cannot be carried across the next level's search.
        let mut level = &mut out as *mut InlineEntries;
        for i in 0..path.len() - 1 {
            let (seg, seg_at) = (path[i].0.as_str(), path[i].1);
            unsafe {
                let entries = &mut *level;
                match entries.iter_mut().find(|(n, _, _)| n.as_str() == seg) {
                    Some((_, TomlValue::InlineTable(children), _)) => {
                        level = children as *mut InlineEntries;
                    }
                    Some(_) => {
                        return Err(Parser::static_err(
                            "dotted key path traverses a non-table value",
                            seg_at,
                        ));
                    }
                    None => {
                        entries.push((
                            seg.to_string(),
                            TomlValue::InlineTable(Vec::new()),
                            KVAnnotations {
                                key_at: seg_at,
                                value_at: path[i + 1].1,
                                ..Default::default()
                            },
                        ));
                        if let Some((_, TomlValue::InlineTable(children), _)) = entries.last_mut() {
                            level = children as *mut InlineEntries;
                        }
                    }
                }
            }
        }
        unsafe {
            (*level).push((leaf, value, anns));
        }
    }
    Ok(out)
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

#[cfg(test)]
mod tests {
    use super::*;
    use pyrs_ast::ast::{CustomNode, ScalarStyle};

    #[test]
    fn unicode_escape_followed_by_multibyte_char_errors_not_panics() {
        // The TOML sibling of the json `hex4` fix: `\uXXXX` / `\UXXXXXXXX` and
        // `\xHH` escape readers sliced `&self.text[pos..pos+width]`, which
        // panics on a non-char boundary when the escape is followed by a
        // multibyte char. Must reject cleanly instead of aborting. Two U+FEFF
        // after `\u` put the 4-byte read window inside the second FEFF.
        let uni = "a = \"\\u\u{feff}\u{feff}\"";
        assert!(matches!(from_toml(uni), Err(ParseError::Syntax { .. })));
        // `\x` (width 2): one FEFF after the escape straddles the 2-byte window.
        let byte = "a = \"\\x\u{feff}\"";
        assert!(matches!(from_toml(byte), Err(ParseError::Syntax { .. })));
    }

    #[test]
    fn a_multibyte_run_is_copied_whole_and_the_bytes_that_ended_it_still_do() {
        // The basic-string scanner bulk-copies up to the next ASCII byte that needs a decision, so
        // a run of multi-byte characters arrives as one slice - as the whole value, as the tail
        // after an escape, and as the run that resumes right after one. The stop set is unchanged:
        // a control character or DEL directly after multi-byte text, a line terminator, and the
        // end of input are still clean rejections rather than content or a panic.
        let src = "a = \"héllo — 名前\"\nb = \"\\u00e9尾\"\nc = \"é\\t\\ttab\"\n";
        let ast = from_toml(src).unwrap();
        let CustomNode::Mapping { pairs, .. } = &ast else {
            panic!("{ast:?}");
        };
        let got: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| match (k, v) {
                (CustomNode::Scalar { value: k, .. }, CustomNode::Scalar { value: v, .. }) => {
                    (k.to_string(), v.to_string())
                }
                _ => panic!("expected scalar pair"),
            })
            .collect();
        assert_eq!(
            got,
            vec![
                ("a".to_string(), "héllo — 名前".to_string()),
                ("b".to_string(), "é尾".to_string()),
                ("c".to_string(), "é\t\ttab".to_string()),
            ]
        );

        for bad in [
            "a = \"é\u{1}\"",
            "a = \"é\u{7f}\"",
            "a = \"é\n\"",
            "a = \"é",
        ] {
            assert!(
                matches!(from_toml(bad), Err(ParseError::Syntax { .. })),
                "{bad:?} parsed instead of rejecting"
            );
        }
    }

    /// Run `f` on a thread with a large stack so the recursion-limit tests
    /// exercise the guard, not the runner's default 2 MiB test-thread stack.
    /// Debug builds carry fat frames, so reaching the 1000-deep trip point
    /// overflows the small default stack before the guard can fire; production
    /// callers (main thread / the maturin extension) have an 8 MiB stack, so
    /// the guard is the real backstop there. The oversized thread mirrors that.
    ///
    /// A thread is a `std` object, so these two cases - and only these - exist on
    /// one side of the feature split. The depth guard itself is `alloc`-only logic
    /// and the same code runs under `default-features = false`; what the `no_std`
    /// side does not get here is a *stack* to overflow, which is why it cannot be
    /// the thing under test.
    #[cfg(feature = "std")]
    fn on_big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(f)
            .expect("spawn big-stack thread")
            .join()
            .expect("big-stack thread panicked")
    }

    #[cfg(feature = "std")]
    #[test]
    fn deep_nesting_errors_instead_of_overflowing_the_stack() {
        // Regression for the TOML analogue of the #166 YAML merge stack
        // overflow: before the depth guard, a 5000-deep array aborted the
        // process (native stack overflow, exit 0xC00000FD). With the guard
        // it must return `MaxDepthExceeded` cleanly, not overflow.
        on_big_stack(|| {
            let src = format!("a = {}{}", "[".repeat(5000), "]".repeat(5000));
            let err = from_toml(&src).expect_err("over-nested array must be rejected");
            assert!(matches!(err, ParseError::MaxDepthExceeded(_)), "{err:?}");

            let inline = format!("a = {}1{}", "{b = ".repeat(5000), "}".repeat(5000));
            let err = from_toml(&inline).expect_err("over-nested inline table must be rejected");
            assert!(matches!(err, ParseError::MaxDepthExceeded(_)), "{err:?}");
        });
    }

    #[cfg(feature = "std")]
    #[test]
    fn within_depth_budget_parses() {
        // Comfortably within the 1000 budget: a 500-deep array parses on a
        // production-sized stack.
        on_big_stack(|| {
            let src = format!("a = {}{}", "[".repeat(500), "]".repeat(500));
            assert!(from_toml(&src).is_ok());
        });
    }

    #[test]
    fn comment_only_document_keeps_its_note_on_the_root() {
        // A `# …` line with no following key used to drop on re-parse, so
        // the writer fixed point drifted (`# note` → parse → `` ). The
        // leftover standalone note now lands on the otherwise-empty root
        // table and survives a second emit unchanged.
        let node = from_toml("# only note\n").unwrap();
        let text = crate::to_toml(&node).unwrap();
        assert_eq!(text, "# only note\n");
        let again = crate::to_toml(&from_toml(&text).unwrap()).unwrap();
        assert_eq!(again, text);
    }

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

    /// The value one key names inside a mapping. A dotted key's whole point is that its table sits
    /// below the name, so these tests ask for a child by name and read as a failure whatever the
    /// answer is not — a missing `type` under `animal` is the defect, not a `None` to branch on.
    fn child<'a>(node: &'a CustomNode, name: &str) -> &'a CustomNode {
        let CustomNode::Mapping { pairs, .. } = node else {
            panic!("asked for `{name}` on a node that is not a mapping");
        };
        pairs
            .iter()
            .find(|(k, _)| matches!(k, CustomNode::Scalar { value, .. } if value.as_ref() == name))
            .map(|(_k, v)| v)
            .unwrap_or_else(|| panic!("no key `{name}` among {:?}", parse_keys(node)))
    }

    /// The text a scalar node carries, for the string-form tests that ask what a scanner kept.
    fn scalar(node: &CustomNode) -> String {
        let CustomNode::Scalar { value, .. } = node else {
            panic!("asked for a scalar where the tree has something else");
        };
        value.to_string()
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
    fn an_inline_table_dotted_key_names_a_path_not_a_key_with_a_dot() {
        // `animal = { type.name = "pug" }` is the TOML spec's own example of the form, and it is the
        // shape `valid/spec-1.0.0/inline-table-0.toml` asserts. Keeping `type.name` as one key text
        // instead of a table both disagreed with the spec and collapsed this document onto the shape
        // of `animal = { "type.name" = "pug" }`, which says something different.
        let ast = from_toml("animal = { type.name = \"pug\" }\n").unwrap();
        let animal = child(&ast, "animal");
        assert_eq!(parse_keys(animal), vec!["type"]);
        assert_eq!(parse_keys(child(animal, "type")), vec!["name"]);
    }

    #[test]
    fn sibling_dotted_members_of_an_inline_table_share_the_one_table_they_name() {
        let ast = from_toml("t = { a.b = 1, x = 2, a.c = 3 }\n").unwrap();
        let t = child(&ast, "t");
        assert_eq!(parse_keys(t), vec!["a", "x"]);
        assert_eq!(parse_keys(child(t, "a")), vec!["b", "c"]);
    }

    #[test]
    fn a_quoted_dotted_key_stays_one_key() {
        // The other half of the distinction the fix opens: quotes put the dot inside the name, so
        // this document must stay one key and must not be read as the path it now spells.
        let ast = from_toml("t = { \"a.b\" = 1 }\n").unwrap();
        assert_eq!(parse_keys(child(&ast, "t")), vec!["a.b"]);
    }

    #[test]
    fn a_deep_inline_dotted_key_nests_every_level_it_names() {
        let ast = from_toml("t = { a.b.c.d = 1 }\n").unwrap();
        let mut node = child(&ast, "t");
        for name in ["a", "b", "c"] {
            node = child(node, name);
        }
        assert_eq!(parse_keys(node), vec!["d"]);
    }

    #[test]
    fn the_last_three_quotes_of_a_run_close_a_literal_multiline_string() {
        // A literal multi-line string that ends in `'` is written with extra quotes ahead of the
        // closing delimiter, and only the last three of the run close it: `''''one quote''''` is
        // `'one quote'`, and the five-quote tail below is `''`. Slicing at the FIRST `'''` match
        // and stepping over the rest ate that tail — the corpus files
        // `valid/string/multiline-quotes` (`lit_one`) and `valid/string/raw-multiline`
        // (`this-str-has-apostrophes`) were the evidence.
        let one = from_toml("lit = ''''one quote''''\n").unwrap();
        assert_eq!(scalar(child(&one, "lit")), "'one quote'");
        let two = from_toml("lit = '''a'''''\n").unwrap();
        assert_eq!(scalar(child(&two, "lit")), "a''");
        // The control: a run of exactly three closes, and nothing is left to hand back.
        let plain = from_toml("lit = '''a'''\n").unwrap();
        assert_eq!(scalar(child(&plain, "lit")), "a");
    }

    #[test]
    fn an_escaped_backslash_at_the_end_of_a_line_is_content_not_a_continuation() {
        // `\\` is an escaped backslash, so the break that follows it belongs to the string; only an
        // unescaped backslash at line end swallows the break and the indent after it. The old test
        // looked one byte past the first backslash, saw the break behind the second one, and ate the
        // newline — `valid/string/multiline`'s `escape-bs-1`.
        let escaped = from_toml("a = \"\"\"x \\\\\nb\"\"\"\n").unwrap();
        assert_eq!(scalar(child(&escaped, "a")), "x \\\nb");
        // The control that must keep working: a lone backslash at line end is a continuation.
        let continued = from_toml("a = \"\"\"x \\\n   b\"\"\"\n").unwrap();
        assert_eq!(scalar(child(&continued, "a")), "x b");
    }

    #[test]
    fn a_deep_header_leaves_the_super_table_it_passed_through_open() {
        // A header's walk only closes the table it names, never the ones it passes through, so the
        // super-table `[x.y.z.w]` implied stays open: `[x]` below it is TOML 1.1's legal
        // super-table re-opening, which is the corpus file `valid/table/without-super.toml`.
        let ast = from_toml("[x.y.z.w]\nk = 1\n[x]\nj = 2\n").unwrap();
        assert_eq!(parse_keys(&ast), vec!["x"]);
        let x = child(&ast, "x");
        assert_eq!(parse_keys(x), vec!["y", "j"]);
    }

    #[test]
    fn a_key_whose_name_contains_a_dot_is_not_the_deeper_path() {
        // `[a."b.c"]` and `[a.b.c]` join to the same text, which is how the header bookkeeping came
        // to refuse one of them as a duplicate of the other — the same conflation the inline-table
        // fix removed, in `[header]` form. `valid/table/names.toml` is the corpus case.
        let ast = from_toml("[a.b.c]\nx = 1\n[a.\"b.c\"]\ny = 2\n").unwrap();
        let a = child(&ast, "a");
        assert_eq!(parse_keys(a), vec!["b", "b.c"]);
        assert_eq!(parse_keys(child(a, "b")), vec!["c"]);
        assert_eq!(scalar(child(child(a, "b.c"), "y")), "2");
    }

    #[test]
    fn a_header_under_an_array_of_tables_belongs_to_the_element_it_was_written_in() {
        // `[[a.b]]` / `[a.b.c]` twice is two tables, one per array element
        // (`valid/table/array-table-array.toml`): the header walks into the element it was written
        // under, so the sub-table it names belongs to that element and its sibling's of the same
        // name is a different table. Writing it twice inside one element is the other half, and the
        // duplicate TOML forbids.
        let ast =
            from_toml("[[a.b]]\n[a.b.c]\nd = \"val0\"\n[[a.b]]\n[a.b.c]\nd = \"val1\"\n").unwrap();
        let a = child(&ast, "a");
        let CustomNode::Sequence { items, .. } = child(a, "b") else {
            panic!("`a.b` is not an array of tables");
        };
        assert_eq!(items.len(), 2);
        for (i, item) in items.iter().enumerate() {
            let c = child(item, "c");
            assert_eq!(scalar(child(c, "d")), format!("val{i}"));
        }
        assert!(
            matches!(
                from_toml("[[a.b]]\n[a.b.c]\nd = 1\n[a.b.c]\ne = 2\n"),
                Err(ParseError::Syntax { .. })
            ),
            "a repeated header inside one array element was accepted"
        );
    }

    #[test]
    fn a_table_a_dotted_key_created_stays_closed_to_a_later_header() {
        // The first two are the bit on the table: `[fruit.apple.taste]` ends on a table a dotted key
        // created, and the corpus insists it stays refused
        // (`invalid/table/duplicate-key-05.toml`, `redefine-03.toml`). The third is the plain
        // duplicate over a header's own table, refused by the walk reaching an `Explicit`.
        for src in [
            "[fruit]\napple.taste.sweet = true\n[fruit.apple.taste]\n",
            "[t1]\nt2.t3.v = 0\n[t1.t2.t3]\n",
            "[a.b.c]\nx = 1\n[a.b.c]\ny = 2\n",
        ] {
            assert!(
                matches!(from_toml(src), Err(ParseError::Syntax { .. })),
                "accepted: {src}"
            );
        }
    }

    #[test]
    fn an_inline_table_still_refuses_a_member_that_extends_a_finished_path() {
        // The nesting is built on top of this rejection: a level walked here is always one a dotted
        // member opened, never an existing value the path would have to overwrite.
        for src in [
            "a = { x = 1, x.y = 2 }",
            "a = { x.y = 1, x = 2 }",
            "a = { x.y = 1, x.y.z = 2 }",
            "a = { b = { c = 1 }, b.d = 2 }",
        ] {
            let outcome = from_toml(src);
            assert!(
                matches!(outcome, Err(ParseError::Syntax { .. })),
                "{src} was accepted"
            );
        }
    }

    #[test]
    fn a_nested_inline_table_writes_back_and_reads_as_the_same_data() {
        // The writer's own shape for the path it was handed (`a = {b = {c = 1}}`) differs from the
        // source spelling, which is what `a.b.c = 1` at table level already does; the claim pinned
        // here is that the data survives, because that is the claim the flat key used to break.
        let src = "animal = { type.name = \"pug\" }\n";
        let ast = from_toml(src).unwrap();
        let text = crate::writer::to_toml(&ast).unwrap();
        let back = from_toml(&text).unwrap();
        assert_eq!(back, ast);
        assert_eq!(parse_keys(child(&back, "animal")), vec!["type"]);
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
    fn i64_lower_bound_negative_integer_is_accepted() {
        // TOML's i64 range is asymmetric: -2^63 is legal. A magnitude-then-
        // negate flow overflows because 9223372036854775808 does not fit i64
        // (found by the Python-side Hypothesis dialect fuzz, PR #172 follow-up).
        let ast = from_toml("lo = -9223372036854775808\nhi = 9223372036854775807\n").unwrap();
        let CustomNode::Mapping { pairs, .. } = &ast else {
            unreachable!()
        };
        let vals: Vec<&str> = pairs
            .values()
            .map(|v| match v {
                CustomNode::Scalar { value, .. } => value.as_ref(),
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(vals, vec!["-9223372036854775808", "9223372036854775807"]);
        // One past the lower bound stays a clean rejection, not a panic.
        assert!(from_toml("lo = -9223372036854775809\n").is_err());
        // ...and so does one past the upper bound.
        assert!(from_toml("hi = 9223372036854775808\n").is_err());
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
    fn binary_spelling_reaches_the_hub_under_an_int_tag() {
        // YAML Core has no `0b` lexeme, so an untagged `0b1010` plain scalar
        // re-resolves to a string. The projection keeps the spelling and states
        // the type instead; hex and octal need no such fence because Core reads
        // them back as integers and a tag would only change the hub text.
        let observed = |src: &str| -> (String, Option<(String, String)>) {
            let ast = from_toml(src).unwrap();
            let CustomNode::Mapping { pairs, .. } = &ast else {
                unreachable!()
            };
            let (_, v) = pairs.iter().next().unwrap();
            let CustomNode::Scalar { value, .. } = v else {
                unreachable!()
            };
            (
                value.to_string(),
                v.tag().map(|t| (t.handle.clone(), t.suffix.clone())),
            )
        };
        assert_eq!(
            observed("b = 0b1010\n"),
            (
                "0b1010".to_string(),
                Some(("!!".to_string(), "int".to_string()))
            )
        );
        assert_eq!(
            observed("h = 0xDEADBEEF\n"),
            ("0xDEADBEEF".to_string(), None)
        );
        assert_eq!(observed("o = 0o755\n"), ("0o755".to_string(), None));
        assert_eq!(observed("d = 42\n"), ("42".to_string(), None));
    }

    #[test]
    fn the_fence_reaches_values_inside_a_container() {
        // The projection is one function, so an array or inline-table member
        // gets the same answer as a top-level pair. The inline table keeps the
        // inline form: a reader marks `inline = { .. }` flow and only a `[inline]`
        // header block, so the first emission is already the fixed point rather
        // than the second one after a rewrite to a section.
        let src = "arr = [0b10, 0x1F]\ninline = { b = 0b11, d = 7 }\n";
        let once = crate::to_toml(&from_toml(src).unwrap()).unwrap();
        assert_eq!(once, "arr = [0b10, 0x1F]\ninline = {b = 0b11, d = 7}\n");
        assert_eq!(once, crate::to_toml(&from_toml(&once).unwrap()).unwrap());
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
