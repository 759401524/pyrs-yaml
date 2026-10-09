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

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use pyrs_ast::ast::{CustomNode, NodeMap};
use pyrs_ast::error::{DepthError, ParseError};
use pyrs_schema::schema::needs_quotes;
use pyrs_schema::types::{Schema, YamlType};

/// Default nesting limit, matching the YAML pipeline's `parse` default.
pub const DEFAULT_MAX_DEPTH: usize = 1000;

/// Dialect knobs for [`from_json_with_options`].
///
/// The default [`JsonParseOptions::STRICT`] implements RFC 8259 exactly:
/// no comments, no trailing commas, one top-level value. [`allow_comments`]
/// upgrades the parser toward JSONC / JSON5, unlocking each axis
/// individually. Trailing commas, single-quoted strings and unquoted
/// identifier keys are the three additional JSON5 features surfaced
/// here; combining all flags yields the JSON5 dialect while
/// [`JsonParseOptions::JSONC`] keeps the parser a strict superset of
/// RFC 8259 with only comments enabled.
///
/// [`allow_comments`]: JsonParseOptions::allow_comments
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonParseOptions {
    pub allow_comments: bool,
    pub allow_trailing_commas: bool,
    pub allow_single_quoted: bool,
    pub allow_unquoted_keys: bool,
    /// JSON5 numeric forms: hexadecimal integers (`0xDECAF`), leading `+`,
    /// leading / trailing decimal point (`.5`, `5.`), and the bare
    /// `Infinity` / `NaN` literals. Off for STRICT and JSONC so those
    /// dialects keep rejecting them exactly as before; on for JSON5.
    pub allow_json5_numbers: bool,
    pub max_depth: usize,
}

impl JsonParseOptions {
    /// RFC 8259 with the default depth budget. `from_json` maps to this.
    pub const STRICT: JsonParseOptions = JsonParseOptions {
        allow_comments: false,
        allow_trailing_commas: false,
        allow_single_quoted: false,
        allow_unquoted_keys: false,
        allow_json5_numbers: false,
        max_depth: DEFAULT_MAX_DEPTH,
    };
    /// JSONC: allows `//` and `/* ... */` comments. Everything else strict.
    pub const JSONC: JsonParseOptions = JsonParseOptions {
        allow_comments: true,
        allow_trailing_commas: false,
        allow_single_quoted: false,
        allow_unquoted_keys: false,
        allow_json5_numbers: false,
        max_depth: DEFAULT_MAX_DEPTH,
    };
    /// JSON5: all extensions on. Trailing commas, single-quoted
    /// strings, unquoted identifier keys, line/block comments and the
    /// hexadecimal / leading-sign / leading-dot / `Infinity` / `NaN`
    /// numeric forms.
    pub const JSON5: JsonParseOptions = JsonParseOptions {
        allow_comments: true,
        allow_trailing_commas: true,
        allow_single_quoted: true,
        allow_unquoted_keys: true,
        allow_json5_numbers: true,
        max_depth: DEFAULT_MAX_DEPTH,
    };
}

/// Parse one JSON document into the shared AST.
pub fn from_json(text: &str) -> Result<CustomNode, ParseError> {
    from_json_with_options(text, JsonParseOptions::STRICT)
}

/// Parse one JSONC document (JSON with comments) into the shared AST.
///
/// Comments are preserved on the AST (standalone notes ride the
/// `leading_comment` slot introduced by #114, inline notes the
/// `comment` slot) and round-trip through `to_jsonc_text` /
/// `to_jsonc_text_pretty`. `to_json_text` still drops them.
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
            max_depth,
            ..JsonParseOptions::STRICT
        },
    )
}

/// Parse one JSON5 document into the shared AST.
pub fn from_json5(text: &str) -> Result<CustomNode, ParseError> {
    from_json_with_options(text, JsonParseOptions::JSON5)
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
        allow_trailing_commas: opts.allow_trailing_commas,
        allow_single_quoted: opts.allow_single_quoted,
        allow_unquoted_keys: opts.allow_unquoted_keys,
        allow_json5_numbers: opts.allow_json5_numbers,
        pending_comment: Vec::new(),
    };
    p.ws();
    // A file-leading note belongs to the root container itself, mirroring
    // the writer's `emit_root_leading` placement. Claim it from the slot
    // before descending so the first member or element cannot steal it —
    // otherwise `// A\n{"k":…}` re-reads with the note on the key and the
    // JSONC/JSON5 writer fixed point drifts.
    let root_leading = core::mem::take(&mut p.pending_comment);
    let value = p.value()?;
    p.ws();
    if p.pos != p.s.len() {
        return Err(p.err("trailing characters after the JSON value"));
    }
    // File-trailing notes are claimed by the first member or element when
    // one exists, but an empty container (`{}` / `[]`) or a root scalar
    // leaves them dangling in `pending_comment` — they would silently
    // vanish from the AST and break the JSONC/JSON5 writer fixed point.
    // Attach any leftover note to the root node as well.
    let mut root = value;
    for pc in root_leading {
        root.push_leading_comment(pyrs_ast::ast::Comment {
            text: alloc::sync::Arc::from(pc.text),
            standalone: true,
        });
    }
    p.flush_pending(&mut root);
    Ok(root)
}

struct Parser<'a> {
    s: &'a [u8],
    text: &'a str,
    pos: usize,
    depth: usize,
    max_depth: usize,
    allow_comments: bool,
    allow_trailing_commas: bool,
    allow_single_quoted: bool,
    allow_unquoted_keys: bool,
    allow_json5_numbers: bool,
    /// The JSONC comments consumed by `ws()` that have not met a node yet, in
    /// source order. `own_line` records whether the comment started on a line of
    /// its own (i.e. no non-whitespace token on the current line before it), which
    /// maps directly onto `Comment::standalone`. Only populated when
    /// `allow_comments` is on.
    ///
    /// A list: a member or element may be introduced by any number of comment
    /// lines, and a single slot kept only the last of them.
    pending_comment: Vec<PendingComment>,
}

/// Record a stack of pending notes onto `node`: every standalone one appends to
/// the node's leading list in source order, while a same-line trailing note keeps
/// owning the single inline `comment` slot (unchanged from #112).
fn attach_pending(node: &mut CustomNode, pending: Vec<PendingComment>) {
    for pc in pending {
        let comment = pyrs_ast::ast::Comment {
            text: alloc::sync::Arc::from(pc.text),
            standalone: pc.own_line,
        };
        if pc.own_line {
            node.push_leading_comment(comment);
        } else {
            node.set_comment(comment);
        }
    }
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
        // A comment at input offset 0 (file-leading) is also on its own
        // line even though no newline precedes it in the buffer; seed the
        // flag so it is classified standalone rather than as a trailing
        // note the writer would drop on an empty/root container.
        let mut own_line_seen = self.pos == 0;
        loop {
            match self.peek() {
                Some(b' ' | b'\t') => self.pos += 1,
                Some(b'\n' | b'\r') => {
                    own_line_seen = true;
                    self.pos += 1;
                }
                // JSON5 structural whitespace beyond RFC 8259's four
                // (tab / space / LF / CR): vertical tab and form feed, then
                // NBSP, the Unicode Zs space separators, the LS/PS line
                // terminators and ZWNBSP (U+FEFF). Gated on the JSON5 flag
                // so STRICT / JSONC keep rejecting every one of them.
                Some(0x0b | 0x0c) if self.allow_json5_numbers => self.pos += 1,
                Some(b) if self.allow_json5_numbers && b >= 0x80 => {
                    // `self.pos` always sits on a char boundary (each
                    // advance in this loop moves by a whole code point), and
                    // a lead byte >= 0x80 means a multi-byte char starts
                    // here. Skip it when it is JSON5 whitespace.
                    if !self.text.is_char_boundary(self.pos) {
                        return;
                    }
                    if let Some(c) = self.text[self.pos..]
                        .chars()
                        .next()
                        .filter(|c| is_json5_ws(*c))
                    {
                        if matches!(c, '\u{2028}' | '\u{2029}') {
                            own_line_seen = true;
                        }
                        self.pos += c.len_utf8();
                    } else {
                        return;
                    }
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
                        while !self.eof_pos() {
                            // Advance a whole code point: `pos + 1` steps could
                            // stop inside a multi-byte char, leaving `pos` off
                            // the boundary for every later `&self.text[pos..]`
                            // (libFuzzer: `//` + trailing FEFF then `expect`
                            // panicking with "not a char boundary"). No
                            // multi-byte char is a line terminator, so this
                            // breaks exactly where the byte test did.
                            match self.text[self.pos..].chars().next() {
                                Some('\n') | Some('\r') => break,
                                Some(c) => self.pos += c.len_utf8(),
                                None => break,
                            }
                        }
                        let body_end = self.pos;
                        self.pending_comment.push(PendingComment {
                            text: self.text[body_start..body_end].trim().to_string(),
                            own_line: own_line_seen,
                        });
                    } else {
                        self.pos = body_start;
                        loop {
                            if self.pos + 1 >= self.s.len() {
                                // Unterminated block comment: rewind to the
                                // `/` so the outer parser rejects cleanly from
                                // a char boundary — leaving `pos` wherever the
                                // byte scan stopped can put it inside a
                                // multi-byte char, and `expect`/`key_start`
                                // slice `&self.text[pos..]` next.
                                self.pos = body_start - 2;
                                return;
                            }
                            if self.s[self.pos] == b'*' && self.s[self.pos + 1] == b'/' {
                                break;
                            }
                            self.pos += 1;
                        }
                        let body_end = self.pos;
                        self.pos += 2;
                        self.pending_comment.push(PendingComment {
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

    /// Attach every pending comment (if any) to `node` and reset the slot.
    ///
    /// PR #115 migrates JSONC's standalone-note capture onto the
    /// `leading_comments` slot introduced by #114, so an object member
    /// or array element can carry BOTH the notes on the lines above AND
    /// a trailing inline note on the same line as its value. Same-line
    /// trailing notes keep using `comment` (unchanged from #112).
    fn flush_pending(&mut self, node: &mut CustomNode) {
        attach_pending(node, core::mem::take(&mut self.pending_comment));
    }

    /// JSON5 single-quoted string. Same escape set as RFC 8259 basic
    /// strings but delimited by `'`; the parser additionally honours the
    /// JSON5 line-continuation backslash rule (a trailing `\` before a
    /// newline swallows both).
    fn single_quoted_string(&mut self) -> Result<String, ParseError> {
        debug_assert_eq!(self.peek(), Some(b'\''));
        self.pos += 1;
        let mut out = String::new();
        loop {
            let b = self.peek().ok_or_else(|| self.err("unterminated string"))?;
            match b {
                b'\'' => {
                    self.pos += 1;
                    return Ok(out);
                }
                b'\\' => {
                    self.pos += 1;
                    let e = self.peek().ok_or_else(|| self.err("unterminated escape"))?;
                    self.pos += 1;
                    match e {
                        b'\'' => out.push('\''),
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'\n' | b'\r' => {
                            // JSON5 line continuation: nothing appended.
                        }
                        b'u' => self.unicode_escape(&mut out)?,
                        // JSON5-only string escapes (allow_json5_numbers is
                        // true only for the JSON5 preset): vertical tab and
                        // NUL. Strict JSON / JSONC still reject them.
                        b'v' if self.allow_json5_numbers => out.push('\u{b}'),
                        b'0' if self.allow_json5_numbers => out.push('\0'),
                        _ => return Err(self.err("invalid escape sequence")),
                    }
                }
                b'\n' | b'\r' => {
                    return Err(self.err("newline in single-quoted string"));
                }
                _ => {
                    // Copy one whole character and advance by its UTF-8 length so
                    // `self.pos` stays on a char boundary; the byte-wise
                    // `floor_char_boundary` path could slice `text[pos-1..cut]`
                    // from a mid-character offset and panic on lossy-decoded input.
                    match self.text[self.pos..].chars().next() {
                        Some(ch) => {
                            out.push(ch);
                            self.pos += ch.len_utf8();
                        }
                        None => return Err(self.err("unterminated string")),
                    }
                }
            }
        }
    }

    /// True when the code point at `self.pos` can start a JSON5 identifier
    /// name: an ASCII `A-Z a-z _ $` or any Unicode `ID_Start` (PR #129). Used
    /// only as a cheap dispatch guard; `identifier_key` re-checks exactly.
    fn at_identifier_start(&self) -> bool {
        match self.peek() {
            Some(b'A'..=b'Z' | b'a'..=b'z' | b'_' | b'$') => true,
            // A lead byte means a multi-byte code point begins here; it is a
            // valid start only if it is Unicode `ID_Start`. A non-boundary
            // position falls through to `false`.
            Some(b) if b >= 0x80 && self.text.is_char_boundary(self.pos) => self.text[self.pos..]
                .chars()
                .next()
                .is_some_and(unicode_ident::is_xid_start),
            _ => false,
        }
    }

    /// JSON5 identifier key (unquoted). The first code point is an ASCII
    /// `A-Z a-z _ $` or any Unicode `ID_Start`; continuation adds digits,
    /// combining marks and connectors (Unicode `ID_Continue` plus `$`/`_`).
    /// PR #129 lifted the former ASCII-only limit to full Unicode via the
    /// `unicode-ident` tables, so e.g. `{ é: 1, 名: 2, हिन्दी: 3 }` parses.
    fn identifier_key(&mut self) -> Result<String, ParseError> {
        let start = self.pos;
        let first = self.text[start..]
            .chars()
            .next()
            .ok_or_else(|| self.err("expected an identifier key"))?;
        if !(first == '$' || first == '_' || unicode_ident::is_xid_start(first)) {
            return Err(self.err("expected an identifier key"));
        }
        self.pos += first.len_utf8();
        while self.pos < self.s.len() {
            match self.text[self.pos..].chars().next() {
                Some(ch) if ch == '$' || ch == '_' || unicode_ident::is_xid_continue(ch) => {
                    self.pos += ch.len_utf8();
                }
                _ => break,
            }
        }
        Ok(self.text[start..self.pos].to_string())
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
            Some(b'\'') if self.allow_single_quoted => {
                let s = self.single_quoted_string()?;
                // PR #121: keep the single-quote style on the AST (JSON5-
                // only path, so STRICT / JSONC never produce this) so
                // `to_json5_text` can reproduce `'…'` rather than forcing
                // every string double-quoted.
                Ok(CustomNode::Scalar {
                    value: s.into(),
                    style: pyrs_ast::ast::ScalarStyle::SingleQuoted,
                    chomping: pyrs_ast::ast::Chomping::Clip,
                    block_indent: None,
                    meta: pyrs_ast::ast::NodeMeta::default(),
                })
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
            // JSON5 numeric forms (PR #120): a leading `+` or `.` and the
            // bare `Infinity` / `NaN` literals. Gated so STRICT / JSONC
            // keep treating them as "expected a JSON value" errors.
            //
            // The two words are stored in the hub's own spelling (`.inf` / `.nan`), not verbatim.
            // `Infinity` is a YAML *string* under the Core schema, so keeping the source word would
            // make every consumer that re-reads the projection change the value's type (#312);
            // `.inf` is the spelling that resolves to `f64::INFINITY` there, and the JSON5 writer
            // restores the dialect's token from the value. The other JSON5-only forms (`0x…`, `.5`,
            // `5.`, `+7`) stay verbatim because YAML already reads them as numbers.
            Some(b'+') | Some(b'.') if self.allow_json5_numbers => self.number(),
            Some(b'I') if self.allow_json5_numbers => {
                self.expect("Infinity", "expected `Infinity`")?;
                Ok(CustomNode::plain_scalar(".inf"))
            }
            Some(b'N') if self.allow_json5_numbers => {
                self.expect("NaN", "expected `NaN`")?;
                Ok(CustomNode::plain_scalar(".nan"))
            }
            Some(b'-') | Some(b'0'..=b'9') => self.number(),
            _ => Err(self.err("expected a JSON value")),
        }
    }

    fn object(&mut self) -> Result<CustomNode, ParseError> {
        self.pos += 1; // '{'
        let mut pairs: NodeMap<CustomNode, CustomNode> = NodeMap::default();
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
            let key_pending = core::mem::take(&mut self.pending_comment);
            let key_str = if self.peek() == Some(b'"') {
                self.string()?
            } else if self.allow_single_quoted && self.peek() == Some(b'\'') {
                self.single_quoted_string()?
            } else if self.allow_unquoted_keys && self.at_identifier_start() {
                self.identifier_key()?
            } else {
                return Err(self.err("expected a quoted object key"));
            };
            let mut key_node = quoted_or_plain(key_str);
            attach_pending(&mut key_node, key_pending);
            self.ws();
            self.expect(":", "expected `:` after the object key")?;
            self.ws();
            let mut value = self.value()?;
            self.ws();
            self.flush_pending(&mut value);
            // duplicate keys: last value wins, first position kept
            pairs.insert(key_node, value);
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                    self.ws();
                    if self.allow_trailing_commas && self.peek() == Some(b'}') {
                        self.pos += 1;
                        return Ok(CustomNode::plain_mapping(pairs));
                    }
                }
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
            let element_pending = core::mem::take(&mut self.pending_comment);
            let mut item = self.value()?;
            attach_pending(&mut item, element_pending);
            self.ws();
            self.flush_pending(&mut item);
            items.push(item);
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                    self.ws();
                    if self.allow_trailing_commas && self.peek() == Some(b']') {
                        self.pos += 1;
                        return Ok(CustomNode::plain_sequence(items));
                    }
                }
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
        let json5 = self.allow_json5_numbers;
        // Sign. STRICT / JSONC allow only `-`; JSON5 also allows `+`.
        if self.peek() == Some(b'-') || (json5 && self.peek() == Some(b'+')) {
            self.pos += 1;
        }
        // JSON5 signed `Infinity` (`-Infinity` / `+Infinity`): the token
        // starts with a sign, so it reaches `number()` rather than the
        // bare `Infinity` dispatch in `value_inner`. `NaN` is unsigned,
        // so it is not handled here. Signed or not, the hub keeps YAML's
        // float spelling - see `value_inner` for why the word cannot stay.
        if json5 && self.peek() == Some(b'I') {
            self.expect("Infinity", "expected `Infinity` after sign")?;
            let negative = self.text[start..self.pos].starts_with('-');
            return Ok(CustomNode::plain_scalar(if negative {
                "-.inf"
            } else {
                ".inf"
            }));
        }
        // JSON5 hexadecimal integer: `0x` / `0X` followed by one or more
        // hex digits. The raw slice is preserved verbatim so #121's
        // writer can emit it unchanged (same source-spelling strategy as
        // TOML #108).
        if json5
            && self.peek() == Some(b'0')
            && matches!(self.s.get(self.pos + 1).copied(), Some(b'x' | b'X'))
        {
            self.pos += 2;
            let ds = self.pos;
            while matches!(self.peek(), Some(b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F')) {
                self.pos += 1;
            }
            if self.pos == ds {
                return Err(self.err("expected hexadecimal digits in number"));
            }
            return Ok(CustomNode::plain_scalar(
                self.text[start..self.pos].to_string(),
            ));
        }
        // JSON5 leading-dot form: `.5` (no integer part). Under STRICT the
        // `.` branch below requires a preceding integer, so handle it here.
        let mut saw_int = false;
        if json5 && self.peek() == Some(b'.') {
            self.pos += 1;
            let ds = self.pos;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.pos += 1;
            }
            if self.pos == ds {
                return Err(self.err("expected digits after `.` in number"));
            }
            saw_int = true; // fractional part already consumed
        }
        if !saw_int {
            match self.peek() {
                Some(b'0') => {
                    self.pos += 1;
                    if json5 {
                        // JSON5 tolerates a leading zero (`07` parses as
                        // decimal 7); keep consuming the digit run.
                        while matches!(self.peek(), Some(b'0'..=b'9')) {
                            self.pos += 1;
                        }
                    } else if matches!(self.peek(), Some(b'0'..=b'9')) {
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
        }
        // Fractional part. JSON5 allows a trailing `.` with no following
        // digits (`5.`); STRICT requires at least one digit after the dot.
        if self.peek() == Some(b'.') && !saw_int {
            self.pos += 1;
            if matches!(self.peek(), Some(b'0'..=b'9')) {
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.pos += 1;
                }
            } else if !json5 {
                return Err(self.err("expected digits after `.` in number"));
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
                        // JSON5-only escapes (see the single-quoted twin).
                        b'v' if self.allow_json5_numbers => out.push('\u{b}'),
                        b'0' if self.allow_json5_numbers => out.push('\0'),
                        b'\'' if self.allow_json5_numbers => out.push('\''),
                        b'\n' | b'\r' if self.allow_json5_numbers => {
                            // JSON5 line continuation: a backslash
                            // immediately before a line terminator removes
                            // both. Normalise CRLF so the trailing LF is
                            // not mistaken for an unescaped newline.
                            if e == b'\r' && self.peek() == Some(b'\n') {
                                self.pos += 1;
                            }
                        }
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
        // Slice the byte buffer, not `self.text`: a `\u` escape followed by a
        // multibyte char would make `&self.text[pos..pos+4]` land on a
        // non-char boundary and panic (the JSON sibling of the #153 slice
        // crashes). Four hex digits are ASCII, so a valid escape survives the
        // UTF-8 check below; a malformed one that straddles a multibyte char is
        // rejected cleanly instead of aborting.
        let digits = core::str::from_utf8(&self.s[self.pos..self.pos + 4])
            .map_err(|_| self.err("invalid hex digits in \\u escape"))?;
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

/// JSON5 structural whitespace: the Unicode `White_Space` property minus
/// NEL (U+0085, which JSON5 does not classify as whitespace), plus ZWNBSP
/// (U+FEFF). This is exactly the spec's `WhiteSpace ∪ LineTerminator` set
/// (tab, VT, FF, space, NBSP, every Zs separator, CR, LF, LS, PS) minus
/// the ASCII members, which `ws()` already handles on single bytes.
fn is_json5_ws(ch: char) -> bool {
    (ch.is_whitespace() && ch != '\u{85}') || ch == '\u{feff}'
}

/// JSON strings land in the YAML-shaped AST without ever re-resolving: text that a plain YAML scalar
/// would reinterpret is quoted (the same `needs_quotes` discipline the TOML spoke follows).
///
/// The question has to be asked against the widest reader the hub serves, not just the core schema.
/// `needs_quotes` resolves under YAML Core, and Core reads `Infinity` as a string while the JSON5
/// resolver reads it as a number - so a JSON string spelled `"Infinity"` stored plain came back a
/// float from `load_json5` while `load_jsonc` and `load_json` kept it a string, one document meaning
/// two things per reader (#312, the direction the projection *receives*). Quoting a string that did
/// not need it costs a pair of bytes in the hub and changes no value; leaving one that did unquoted is
/// a type change. The same reasoning as `plain_text_is_typed` for cross-format mapping keys.
///
/// Asking it of every string was itself a defect: measured by `.ci/ir-baseline.json`, the unconditional second
/// resolver cost `from_json_medium` +14.24% and `from_jsonc_medium` +13.22% of their instructions against a
/// 0.50% tolerance. The value is kept, the cost is not: `json5_could_be_typed` decides whether the resolver
/// could answer at all, and it is a sound filter rather than a guessed one - `resolve_json5_type` trims and
/// then dispatches on `-`, `+`, `.`, a digit, `I` or `N`, so anything else is a string by construction and the
/// resolver would have said so. Strict JSON and JSONC keep identical output; they simply stop paying a full
/// resolve for text that cannot be a number.
fn quoted_or_plain(value: String) -> CustomNode {
    if needs_quotes(&value)
        || (json5_could_be_typed(&value)
            && !matches!(Schema::Json5.resolve(&value), YamlType::Str(_)))
    {
        CustomNode::double_quoted_scalar(value)
    } else {
        CustomNode::plain_scalar(value)
    }
}

/// Whether `text` can resolve to anything other than a string under the JSON5 profile.
///
/// The direction that must hold is one-way and it is the safe one: *if the resolver would return a non-string,
/// this returns true*. Skipping a text the resolver would have called a string changes nothing, because the
/// caller only quotes on a non-string answer. `json5_guard_is_a_sound_superset` checks it against the resolver
/// itself over the spellings that made #312, so the filter cannot drift from the grammar it stands in for.
///
/// The shape is a measurement rather than a style preference. `text.trim()` cost `from_json_medium` +2.13%;
/// decoding one `char` cost +0.61%; a chain of byte compares still landed on +0.51% against a 0.50% tolerance
/// (`.ci/ir-baseline.json`). All of it is work done once per text the core schema was content to leave plain,
/// and a medium fixture holds tens of thousands of those - so the decision is now one table lookup on the first
/// byte, the same 128-entry-table move the serializer made for tag emission.
fn json5_could_be_typed(text: &str) -> bool {
    let Some(&first) = text.as_bytes().first() else {
        return false;
    };
    match json5_first_byte_class(first) {
        0 => false,
        1 => true,
        _ => text.trim().chars().next().is_some_and(json5_value_start),
    }
}

/// One byte, three answers: cannot begin a JSON5 value (`0`), does (`1`), or is whitespace, where the bytes
/// after it decide (`2`). High bytes are class `2` deliberately: JSON5's whitespace set lives above ASCII - NBSP,
/// the U+2000 separators, LS/PS - and `resolve_json5_type` trims before it dispatches, so a padded ` Infinity `
/// really is a float while a leading `é` is not. Both are settled by asking the resolver's own precondition in
/// the rare branch rather than by duplicating the Unicode whitespace table here.
fn json5_first_byte_class(byte: u8) -> u8 {
    const TABLE: [u8; 128] = {
        let mut table = [0u8; 128];
        let mut index = 0usize;
        while index < 128 {
            let character = index as u8;
            if matches!(character, b'-' | b'+' | b'.' | b'I' | b'N') || character.is_ascii_digit() {
                table[index] = 1;
            } else if matches!(character, 0x09..=0x0d | b' ') {
                table[index] = 2;
            }
            index += 1;
        }
        table
    };
    if byte >= 0x80 {
        return 2;
    }
    TABLE[usize::from(byte)]
}

/// The characters a JSON5 value can begin with: the non-finite spellings start with `I` or `N`, and JSON5
/// numbers start with a digit, a sign, or a leading dot.
fn json5_value_start(ch: char) -> bool {
    matches!(ch, '-' | '+' | '.' | '0'..='9' | 'I' | 'N')
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
    use pyrs_ast::ast::ScalarStyle;
    use pyrs_schema::types::Schema;

    fn scalar_text(node: &CustomNode) -> String {
        match node {
            CustomNode::Scalar { value, .. } => value.to_string(),
            other => panic!("expected scalar, got {other:?}"),
        }
    }

    /// libFuzzer find (fuzz/artifacts/parse_json/crash-e039bcb…): the
    /// unterminated-block-comment scanner stepped byte-wise and stopped inside
    /// the trailing U+FEFF, so the next `&self.text[pos..]` slice panicked with
    /// "start byte index is not a char boundary". All three dialects must now
    /// return a typed error instead.
    #[test]
    fn truncated_comment_before_multibyte_errors_not_panics() {
        let block = "\r\r{aMNaN/*0\u{feff}";
        let line = "//\u{feff}";
        for src in [block, line] {
            assert!(from_json(src).is_err(), "strict accepted {src:?}");
            let _ = from_jsonc(src);
            let _ = from_json5(src);
        }
    }

    #[test]
    fn escape_followed_by_multibyte_char_errors_not_panics() {
        // Regression for the char-boundary panic `hex4` used to hit: a `\u`
        // escape followed by a multibyte char made `&self.text[pos..pos+4]`
        // slice across a char boundary and abort the process (surfaced by the
        // dialect no-panic fuzz). Must now be a clean parse error. Two U+FEFF
        // (3 bytes each) after `\u` put the 4-byte read window inside the second
        // FEFF.
        let tricky = "\"\\u\u{feff}\u{feff}";
        assert!(matches!(from_json(tricky), Err(ParseError::Syntax { .. })));
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
    fn json5_double_quoted_line_continuation_and_quote_escape() {
        // PR #127: JSON5 allows a backslash-newline line continuation and
        // an escaped single quote inside double-quoted strings.
        let CustomNode::Scalar { value, .. } = &from_json5("\"ab\\\ncd\"").unwrap() else {
            unreachable!()
        };
        assert_eq!(&**value, "abcd");
        let CustomNode::Scalar { value, .. } = &from_json5("\"it\\'s\"").unwrap() else {
            unreachable!()
        };
        assert_eq!(&**value, "it's");
        // Strict JSON still rejects both.
        assert!(from_json("\"ab\\\ncd\"").is_err());
        assert!(from_json("\"it\\'s\"").is_err());
    }

    #[test]
    fn json5_accepts_unicode_whitespace() {
        // PR #128: JSON5 treats VT, FF, NBSP, the Unicode Zs separators,
        // the LS/PS line terminators and ZWNBSP (U+FEFF) as structural
        // whitespace between tokens (unquoted keys `a` / `b` are JSON5).
        let node = from_json5("{\u{a0}a\u{2007}:\u{3000}1,\u{2028}b: 2\u{feff}}").unwrap();
        let CustomNode::Mapping { pairs, .. } = &node else {
            unreachable!()
        };
        assert_eq!(pairs.len(), 2);
        let node = from_json5("{a: 1\u{0b},\u{0c}b: 2}").unwrap();
        let CustomNode::Mapping { pairs, .. } = &node else {
            unreachable!()
        };
        assert_eq!(pairs.len(), 2);
        // STRICT / JSONC keep rejecting every exotic whitespace form.
        assert!(from_json("{\u{a0}a: 1}").is_err());
        assert!(from_json("\u{0b}{}").is_err());
        assert!(from_json("\u{0c}{}").is_err());
        assert!(from_json("\u{feff}{}").is_err());
        assert!(from_jsonc("{\u{2028}\"a\": 1}").is_err());
    }

    #[test]
    fn json5_supports_vertical_tab_and_nul_escapes() {
        // JSON5 adds `\v` (U+000B) and `\0` (U+0000) to the string
        // escape set; they must parse under JSON5 and stay rejected
        // under strict JSON.
        let CustomNode::Scalar {
            value: s,
            style: ScalarStyle::DoubleQuoted,
            ..
        } = &from_json5(r#""a\vb\0c""#).unwrap()
        else {
            unreachable!()
        };
        assert_eq!(&**s, "a\u{b}b\0c");
        // Single-quoted twin.
        let CustomNode::Scalar { value: s, .. } = &from_json5(r#"'x\vy\0z'"#).unwrap() else {
            unreachable!()
        };
        assert_eq!(&**s, "x\u{b}y\0z");
        // Strict JSON rejects both.
        assert!(from_json(r#""a\vb""#).is_err());
        assert!(from_json(r#""a\0b""#).is_err());
    }

    #[test]
    fn json5_trailing_commas() {
        // JSON5 unlocks trailing commas in both arrays and objects.
        let arr = from_json5("[1, 2, 3,]").unwrap();
        assert!(matches!(arr, CustomNode::Sequence { .. }));
        let obj = from_json5(r#"{"a": 1, "b": 2,}"#).unwrap();
        let CustomNode::Mapping { pairs, .. } = &obj else {
            unreachable!()
        };
        assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn json5_single_quoted_strings() {
        let v = from_json5("'hi'").unwrap();
        match &v {
            CustomNode::Scalar { value, .. } => assert_eq!(value.as_ref(), "hi"),
            other => panic!("{other:?}"),
        }
        // Escapes still work in single-quoted strings.
        let v = from_json5(r#"'a\nb'"#).unwrap();
        match &v {
            CustomNode::Scalar { value, .. } => assert_eq!(value.as_ref(), "a\nb"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn json5_unquoted_identifier_keys() {
        let obj = from_json5("{a: 1, _b2: 2, $c: 3}").unwrap();
        let CustomNode::Mapping { pairs, .. } = &obj else {
            unreachable!()
        };
        let keys: Vec<String> = pairs
            .iter()
            .map(|(k, _)| match k {
                CustomNode::Scalar { value, .. } => value.to_string(),
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(keys, vec!["a", "_b2", "$c"]);
    }

    #[test]
    fn json5_unicode_identifier_keys() {
        // PR #129: unquoted keys accept the full Unicode ID_Start / ID_Continue
        // set (via unicode-ident), not just ASCII. Devanagari `\u0939` etc.
        // exercise combining marks (ID_Continue) mid-identifier.
        let obj = from_json5("{é: 1, 名: 2, हिन्दी: 3, Ωmega: 4}").unwrap();
        let CustomNode::Mapping { pairs, .. } = &obj else {
            unreachable!()
        };
        let keys: Vec<String> = pairs
            .iter()
            .map(|(k, _)| match k {
                CustomNode::Scalar { value, .. } => value.to_string(),
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(keys, vec!["é", "名", "हिन्दी", "Ωmega"]);
        // STRICT / JSONC still require quoting for a non-ASCII key.
        assert!(from_json("{é: 1}").is_err());
        assert!(from_jsonc("{\"é\": 1, é: 2}").is_err());
    }

    #[test]
    fn json5_combines_all_dialect_axes() {
        // Single fixture that would be rejected by every narrower dialect
        // but parses cleanly under JSON5: trailing comma, comments,
        // single-quoted key + value, unquoted key.
        let src = "{
            // leading comment
            a: 'alpha',
            \"b\": 2, // trailing comment
        }";
        let obj = from_json5(src).unwrap();
        let CustomNode::Mapping { pairs, .. } = &obj else {
            unreachable!()
        };
        assert_eq!(pairs.len(), 2);
    }

    #[test]
    fn json5_rejects_are_isolated_to_their_flags() {
        // Turning a specific flag off restores the strict rejection on
        // that axis; the other two JSON5 features still work.
        let opts = JsonParseOptions {
            allow_trailing_commas: false,
            ..JsonParseOptions::JSON5
        };
        assert!(from_json_with_options("[1,]", opts).is_err());
        let opts2 = JsonParseOptions {
            allow_unquoted_keys: false,
            ..JsonParseOptions::JSON5
        };
        assert!(from_json_with_options("{a: 1}", opts2).is_err());
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
        let a = crate::to_json_text(&want).unwrap();
        let b = crate::to_json_text(&got).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn jsonc_file_leading_comment_stays_on_the_root() {
        // A standalone note on line 1 belongs to the root container
        // (mirroring the writer's `emit_root_leading`), not to the first
        // member: it must survive re-parse on an empty container and keep
        // the JSONC/JSON5 writer output a fixed point.
        let round = |src: &str| crate::to_jsonc_text(&from_jsonc(src).unwrap()).unwrap();
        assert_eq!(round("// A\n{}"), "// A\n{}");
        let once = round("// A\n{\"k\": 1}");
        assert!(once.starts_with("// A\n"), "{once}");
        assert_eq!(round(&once), once);
        // JSON5 rides the same root-attachment path.
        let round5 = |src: &str| crate::to_json5_text(&from_json5(src).unwrap()).unwrap();
        assert_eq!(round5("// A\n{}"), "// A\n{}");
    }

    #[test]
    fn jsonc_comment_text_is_written_trimmed() {
        // The parser stores comment bodies trimmed; the writer must emit
        // them trimmed too, or `//  spaced  ` would oscillate trailing
        // whitespace across passes (the fixed point demands a stable
        // spelling from the first emit onward).
        let got = crate::to_jsonc_text(&from_jsonc("//   spaced   \n{}").unwrap()).unwrap();
        assert_eq!(got, "// spaced\n{}");
        let again = crate::to_jsonc_text(&from_jsonc(&got).unwrap()).unwrap();
        assert_eq!(again, got);
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
                pyrs_schema::types::YamlType::Str(_)
            ));
        }
    }

    // ------ JSON5 numeric forms (PR #120) -------------------------------------

    #[test]
    fn json5_parses_hexadecimal_leading_plus_and_dots() {
        // Each of these is invalid strict JSON but a legal JSON5 number.
        // The parser keeps the source slice verbatim so #121's writer can
        // reproduce the exact spelling; here we only assert acceptance and
        // the preserved text on the plain scalar.
        for src in ["0xDECAF", "+7", ".5", "5.", "-0x1F", "0XFF", "+.25"] {
            let n =
                from_json5(src).unwrap_or_else(|e| panic!("{src} should parse as JSON5: {e:?}"));
            let CustomNode::Scalar {
                value,
                style: ScalarStyle::Plain,
                ..
            } = &n
            else {
                panic!("{src} should be a plain scalar: {n:?}")
            };
            assert_eq!(value.as_ref(), src, "{src} spelling not preserved");
        }
    }

    /// Spellings that decide whether the guard is sound: the ones that made #312, whitespace-padded
    /// variants of them, and text that merely looks numeric. Split on `|`, so the empty leading and
    /// trailing fields are the empty string - a case that matters, see the scoped assertion below.
    const JSON5_CORPUS: &str = "|Infinity|+Infinity|-Infinity|NaN|nan|0x1F|5.|-5.|+7|1e3|-0|.5| Infinity |\tNaN|\u{a0}Infinity|\u{2009}NaN|\u{feff}.5|\u{2028}-5.|port|host|yes|index|nation|2024-01-01|0.1.2|null|true|~|a|-one|.com|N/A|9 lives|12 Orchard Rd|é|Ελλάδα|-|+|.| |";

    #[test]
    fn json5_guard_is_a_sound_superset() {
        // The filter may skip the resolver only where the resolver would have answered "string" - within
        // the domain it is ever asked about. `quoted_or_plain` short-circuits on `needs_quotes`, so text the
        // core schema already resolves never reaches the guard and needs no cover; asserting the unscoped
        // superset would have been a claim about a function that is never called that way. The empty string
        // is exactly that case: `""` is a null under both profiles and is quoted before the guard runs.
        for text in JSON5_CORPUS.split('|') {
            if needs_quotes(text) {
                continue;
            }
            let guarded = json5_could_be_typed(text);
            let typed = !matches!(Schema::Json5.resolve(text), YamlType::Str(_));
            assert!(
                !typed || guarded,
                "{text:?} resolves to a value but the guard skipped it"
            );
        }
    }

    #[test]
    fn the_guard_changes_no_strings_verdict() {
        // The claim the guard exists to make is not "fewer calls" but "the same output, for fewer calls",
        // so it is asserted against the unguarded expression case by case. Compared as debug text because
        // `CustomNode`'s equality deliberately ignores style and notes (a noted key must still match a plain
        // one) and style is precisely what this function decides.
        for text in JSON5_CORPUS.split('|') {
            let guarded = quoted_or_plain(text.to_string());
            let unguarded =
                if needs_quotes(text) || !matches!(Schema::Json5.resolve(text), YamlType::Str(_)) {
                    CustomNode::double_quoted_scalar(text.to_string())
                } else {
                    CustomNode::plain_scalar(text.to_string())
                };
            assert_eq!(format!("{guarded:?}"), format!("{unguarded:?}"), "{text:?}");
        }
    }

    #[test]
    fn a_json_string_spelling_a_json5_number_keeps_its_type_in_every_reader() {
        // #312's other direction, and the reason `quoted_or_plain` asks the widest resolver. The core
        // schema reads `Infinity` as a string, so the plain spelling used to survive into the hub - and
        // the JSON5 reader then resolved it as a number, giving one document two types depending on
        // which loader read it. A quoted JSON string is a string in every dialect of the family.
        for word in ["Infinity", "-Infinity", "+Infinity"] {
            let src = format!("[\"{word}\"]");
            for node in [from_json(&src), from_jsonc(&src), from_json5(&src)] {
                let CustomNode::Sequence { items, .. } = node.unwrap() else {
                    unreachable!()
                };
                assert!(
                    matches!(
                        &items[0],
                        CustomNode::Scalar {
                            style: ScalarStyle::DoubleQuoted,
                            ..
                        }
                    ),
                    "{word} must land quoted, got {:?}",
                    items[0]
                );
                assert_eq!(scalar_text(&items[0]), word);
            }
        }
        // A number spelled the same way is still a number - quoting is what separates them, not a
        // different representation of the same thing.
        let CustomNode::Sequence { items, .. } = &from_json5("[Infinity]").unwrap() else {
            unreachable!()
        };
        assert_eq!(scalar_text(&items[0]), ".inf");
    }

    #[test]
    fn json5_parses_infinity_and_nan_into_a_number_the_hub_can_carry() {
        // #312. The projection this parser feeds is YAML text, and YAML's core schema resolves
        // `.inf` / `-.inf` / `.nan` as floats but the words `Infinity` and `NaN` as strings. Storing
        // the source word therefore changed the value's type for every consumer that re-read the hub,
        // while `load_json5` - which resolves in memory - was right. The two disagreeing paths is the
        // defect; the hub spelling is the fix, and the JSON5 token is restored by the writer from the
        // resolved value (see `writer::tests`).
        for (src, hub, want) in [
            ("Infinity", ".inf", Ok(f64::INFINITY)),
            ("+Infinity", ".inf", Ok(f64::INFINITY)),
            ("-Infinity", "-.inf", Ok(f64::NEG_INFINITY)),
            ("NaN", ".nan", Err("")),
        ] {
            let n = from_json5(src).unwrap_or_else(|e| panic!("{src}: {e:?}"));
            let CustomNode::Scalar {
                value,
                style: ScalarStyle::Plain,
                ..
            } = &n
            else {
                panic!("{src} should be a plain scalar: {n:?}")
            };
            assert_eq!(value.as_ref(), hub, "{src} did not reach the hub spelling");
            match (
                pyrs_schema::types::Schema::Core.resolve(value.as_ref()),
                want,
            ) {
                (pyrs_schema::types::YamlType::Float(f), Ok(want)) => {
                    assert_eq!(f, want, "{src} resolved to the wrong float")
                }
                (pyrs_schema::types::YamlType::Float(f), Err(_)) => {
                    assert!(f.is_nan(), "{src} should resolve to NaN, got {f}")
                }
                other => panic!("{src} should resolve to a float, got {other:?}"),
            }
        }
    }

    #[test]
    fn json5_rejects_bad_hex_and_bare_dot() {
        // `0x` with no digits, and a lone `.`, are still errors.
        assert!(from_json5("0x").is_err());
        assert!(from_json5(".").is_err());
        assert!(from_json5("+.").is_err());
    }

    #[test]
    fn strict_and_jsonc_still_reject_json5_numbers() {
        // Dialect gate: hex / leading `+` / leading `.` / `Infinity` /
        // `NaN` stay rejected outside JSON5 so the RFC 8259 contract is
        // byte-for-byte unchanged.
        for src in ["0x1F", "+7", ".5", "Infinity", "NaN"] {
            assert!(from_json(src).is_err(), "{src} must be invalid strict JSON");
            assert!(from_jsonc(src).is_err(), "{src} must be invalid JSONC");
        }
    }

    #[test]
    fn json5_leading_zero_relaxed() {
        // JSON5 permits a leading zero (it is octal-ish legacy in JS, but
        // the spec treats `07` as decimal 7); STRICT rejects it.
        assert!(from_json5("07").is_ok());
        assert!(from_json("07").is_err());
    }
}
