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
    borrow::Cow,
    string::{String, ToString},
    vec::Vec,
};
use pyrs_ast::ast::{CustomNode, NodeMap, ScalarStyle};
use pyrs_ast::error::{DepthError, ParseError};
use pyrs_event::{LineTable, StreamEvent, StreamEventType};
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
    let mut p = parser_from(text, opts);
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

/// The scanner for one document and one dialect — the same starting state for both readers.
///
/// The tree route and the stream route must begin identically, or a dialect flag that reaches one
/// and not the other is a divergence no test will think to ask about.
fn parser_from(text: &str, opts: JsonParseOptions) -> Parser<'_> {
    Parser {
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
    }
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

/// Every note a node carries, in the order the source wrote them: the notes above it, then the one
/// that ended its line.
///
/// Only nodes the parser built reach this: it puts an own-line note on the leading list and a
/// same-line note in the inline slot, never a standalone note in the slot the older hand-built
/// shape used, which is the one case where the two views would report the same text twice.
fn notes_of(node: &CustomNode) -> Vec<pyrs_ast::ast::Comment> {
    let mut notes = node.leading_comments().to_vec();
    if let Some(inline) = node.comment() {
        notes.push(inline.clone());
    }
    notes
}

/// The notes the key node a duplicate name just destroyed owned.
///
/// Rebuilt by running the same `attach_pending` over a throwaway node rather than by restating its
/// rule, because that rule - which note rides the leading list and which owns the inline slot,
/// including the two-same-line-notes case #361 added - is what decided where they went in the first
/// place. Costs one node and one list, and only a document with a repeated key pays it.
fn collapsed_key_notes(pending: &[PendingComment]) -> Vec<pyrs_ast::ast::Comment> {
    if pending.is_empty() {
        return Vec::new();
    }
    let mut ghost = quoted_or_plain(String::new());
    attach_pending(&mut ghost, pending);
    notes_of(&ghost)
}

/// Hand the notes of a collapsed pair to the pair that survived.
///
/// `displaced` is the value the duplicate's own value replaced and `carried` holds what the dropped
/// key node owned; the displaced notes came earlier in the source, so they lead. They append after
/// whatever the survivor already carries - the text order of a collapsed pair is not recoverable
/// from one slot - and the second round writes the list order the first produced, which is what the
/// fixed point asks of it.
fn park_notes(
    pairs: &mut NodeMap<CustomNode, CustomNode>,
    index: usize,
    displaced: CustomNode,
    carried: Vec<pyrs_ast::ast::Comment>,
) {
    let mut dead = notes_of(&displaced);
    if dead.is_empty() && carried.is_empty() {
        return;
    }
    dead.extend(carried);
    let Some((_, value)) = pairs.get_index_mut(index) else {
        return;
    };
    for note in dead {
        if note.standalone || value.comment().is_some() {
            value.push_leading_comment(note);
        } else {
            value.set_comment(note);
        }
    }
}

/// Record a stack of pending notes onto `node`: every standalone one appends to
/// the node's leading list in source order, while a same-line trailing note keeps
/// owning the single inline `comment` slot (unchanged from #112).
///
/// One exception, and it is measured rather than assumed (issue #361): when a node is introduced by
/// **two or more** same-line comments, the single inline slot cannot hold them and the loop below
/// overwrote each with the next. Text this library wrote itself was the casualty - the compact form
/// puts a node's notes on one line, so `{/* note 0 *//* note 1 */"k":1}` reached the AST with one
/// note and the writer's second round differed. All of them now go to the leading list, which holds
/// any number and preserves source order. A lone same-line comment still owns the inline slot, so the
/// nuance a human writes (`{"k": 1 // the port\n}`) is untouched.
fn attach_pending(node: &mut CustomNode, pending: &[PendingComment]) {
    // Every key and every value meets this function, and in a document without comments the whole
    // job is discovering that the list is empty - which `from_json_medium` measured at 3.23% of its
    // counted instructions. The list can only be filled by the `Some(b'/')` arm of the separation
    // loop, so an empty answer is the common case rather than an edge case.
    if pending.is_empty() {
        return;
    }
    if pending.len() > 1 && pending.iter().filter(|pc| !pc.own_line).count() > 1 {
        for pc in pending {
            node.push_leading_comment(pyrs_ast::ast::Comment {
                text: alloc::sync::Arc::from(pc.text.as_str()),
                standalone: true,
            });
        }
        return;
    }
    for pc in pending {
        let comment = pyrs_ast::ast::Comment {
            text: alloc::sync::Arc::from(pc.text.as_str()),
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

    /// Whether the byte at `self.pos` is separation this dialect can consume.
    ///
    /// One copy of the question, asked twice: `ws` guards on it and `ws_extended` answers it
    /// again per byte. The sets here are a *sound superset* of the loop's, the direction that
    /// matters being "if the loop would consume something, this says so" - the loop's high-byte arm
    /// can still decline a character that is not JSON5 separation, and calling it for nothing costs
    /// a call, never a wrong position. `the_whitespace_guard_never_declines_what_the_loop_would`
    /// asks both the same question over every byte and every dialect, so the two cannot drift.
    #[inline]
    fn at_separation(&self) -> bool {
        match self.peek() {
            Some(b' ' | b'\t' | b'\n' | b'\r') => true,
            Some(b'/') => self.allow_comments,
            Some(0x0b | 0x0c) => self.allow_json5_numbers,
            Some(b) => b >= 0x80 && self.allow_json5_numbers,
            None => false,
        }
    }

    /// Skip separation before the next token, comments and all.
    ///
    /// A compact document calls this roughly ninety times per parse and has nothing to skip on
    /// almost every one of them, so the job of this function is to answer "no" without a call:
    /// `from_json_medium` spent 6.63% of its counted instructions in the call itself.
    #[inline]
    fn ws(&mut self) {
        if self.at_separation() {
            self.ws_extended();
        }
    }

    fn ws_extended(&mut self) {
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
        let pending = core::mem::take(&mut self.pending_comment);
        attach_pending(node, &pending);
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
            // Bulk-copy the source run up to the next byte that needs a decision: the closing
            // quote, an escape, or a line terminator (which this dialect forbids inside a
            // single-line string). Each is ASCII, and UTF-8 keeps an ASCII byte out of a multi-byte
            // sequence, so the run is whole code points at both ends and `self.pos` stays on a
            // char boundary - the property the per-character loop below used to guarantee by
            // decoding. Other control characters are legal content here, so they are not in the
            // stop set and the run copies through them.
            let start = self.pos;
            while self.pos < self.s.len() {
                let b = self.s[self.pos];
                if b == b'\'' || b == b'\\' || b == b'\n' || b == b'\r' {
                    break;
                }
                self.pos += 1;
            }
            if self.pos > start {
                out.push_str(&self.text[start..self.pos]);
            }
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
                // The scan above stops on exactly these three cases.
                _ => unreachable!(),
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
                Ok(single_quoted_node(s))
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
            // `hub_scalar` is the one place that spellings live, because the event stream reports
            // the source token and replays through the same function.
            Some(b'+') | Some(b'.') if self.allow_json5_numbers => self.number(),
            Some(b'I') if self.allow_json5_numbers => {
                self.expect("Infinity", "expected `Infinity`")?;
                Ok(hub_scalar("Infinity"))
            }
            Some(b'N') if self.allow_json5_numbers => {
                self.expect("NaN", "expected `NaN`")?;
                Ok(hub_scalar("NaN"))
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
            // standalone note; claim the slot here so the value-side `ws()` below starts with a
            // clean slate, but attach it only after the insert - for a repeated name the node that
            // survives is the first one, not this one.
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
            // A key's notes arrive through `key_pending` and nowhere else, so an empty list already
            // answers "this node owns nothing worth rescuing" without reading its slots.
            attach_pending(&mut key_node, &key_pending);
            self.ws();
            self.expect(":", "expected `:` after the object key")?;
            self.ws();
            let mut value = self.value()?;
            self.ws();
            self.flush_pending(&mut value);
            // Duplicate keys: last value wins, first position kept.
            //
            // Two nodes die in that collapse - the incoming key, since the map keeps the first, and
            // the value the new one replaced - and a note either owned used to be dropped with it
            // while the document still wrote it. A note-free key pays for reading two slots that are
            // empty and nothing else: no second hash, since `insert_full` names the slot the key
            // landed in for the price of the insert this route was doing anyway, and the keys
            // themselves are immutable in the map, so what a duplicate cannot keep it hands on.
            let (index, replaced) = pairs.insert_full(key_node, value);
            if let Some(displaced) = replaced {
                park_notes(
                    &mut pairs,
                    index,
                    displaced,
                    collapsed_key_notes(&key_pending),
                );
            }
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
            attach_pending(&mut item, &element_pending);
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
        Ok(hub_scalar(self.number_source()?))
    }

    /// The same grammar, reporting the source slice instead of a node.
    ///
    /// This is the leaf the event stream needs: a number's text is the document's own bytes, and the
    /// only spelling decision the engine makes is the non-finite projection (`hub_scalar`). Keeping
    /// it here means the tree path and the stream path read one grammar and one projection rule,
    /// which is what the replay test can then hold them to.
    fn number_source(&mut self) -> Result<&'a str, ParseError> {
        let text = self.text;
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
            return Ok(&text[start..self.pos]);
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
            return Ok(&text[start..self.pos]);
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
                    // JSON5's `DecimalIntegerLiteral` is `0 | NonZeroDigit DigitalDigits?` - the very
                    // production RFC 8259 uses - so `07` is not a legal JSON5 number either. The
                    // dialect's latitude lives in the integer prefixes handled above (`0x`/`0X`) and
                    // in the dot forms, not in leading zeros. Measured against `pyjson5`, which
                    // rejects `[012]`, `[-01]` and `[-012]`; this branch used to accept them, citing a
                    // spec allowance that is not in the grammar.
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
        Ok(&text[start..self.pos])
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

/// The hub's own spelling for a JSON5 non-finite token, or `None` for any other text.
///
/// One table for both readers of the projection: the tree path applies it where it builds the
/// scalar, and the event replay applies it to the plain scalar a stream reports, so the two cannot
/// disagree about what `Infinity` becomes (see `value_inner` for why the source word cannot stay).
fn non_finite_spelling(source: &str) -> Option<&'static str> {
    match source {
        "Infinity" | "+Infinity" => Some(".inf"),
        "-Infinity" => Some("-.inf"),
        "NaN" => Some(".nan"),
        _ => None,
    }
}

/// A JSON number or keyword as the hub carries it: the source text, except for the non-finite words.
fn hub_scalar(source: &str) -> CustomNode {
    CustomNode::plain_scalar(non_finite_spelling(source).unwrap_or(source))
}

/// A JSON5 single-quoted string as the hub carries it.
///
/// The style is the dialect's own (PR #121) so `to_json5_text` can reproduce `'…'`; unlike a
/// double-quoted string it never goes through `quoted_or_plain`.
fn single_quoted_node(value: String) -> CustomNode {
    CustomNode::Scalar {
        value: value.into(),
        style: pyrs_ast::ast::ScalarStyle::SingleQuoted,
        chomping: pyrs_ast::ast::Chomping::Clip,
        block_indent: None,
        meta: pyrs_ast::ast::NodeMeta::default(),
    }
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

// ---------------------------------------------------------------------------
// The tree-free event stream
// ---------------------------------------------------------------------------

/// Where each comment token inside `[from, to)` begins, in source order.
fn comment_starts(text: &str, from: usize, to: usize) -> Vec<usize> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut index = from;
    while index + 1 < to {
        if bytes[index] == b'/' {
            if bytes[index + 1] == b'/' {
                found.push(index);
                index += 2;
                while index < to && bytes[index] != b'\n' {
                    index += 1;
                }
                continue;
            }
            if bytes[index + 1] == b'*' {
                found.push(index);
                index += 2;
                while index + 1 < to && !(bytes[index] == b'*' && bytes[index + 1] == b'/') {
                    index += 1;
                }
                index += 2;
                continue;
            }
        }
        index += 1;
    }
    found
}

/// The collector the stream walk writes into: the source, its line table, and the events so far.
struct StreamOut<'a> {
    text: &'a str,
    lines: LineTable<'a>,
    events: Vec<StreamEvent<'a>>,
}

impl<'a> StreamOut<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            text,
            lines: LineTable::new(text),
            events: Vec::new(),
        }
    }

    fn at(&mut self, pos: usize, event_type: StreamEventType<'a>) {
        let (line, column) = self.lines.line_col(pos);
        self.events.push(StreamEvent {
            event_type,
            line,
            column,
        });
    }

    /// A scalar whose text is the source's own bytes: a number, a keyword, an identifier key.
    fn plain(&mut self, from: usize, to: usize) {
        let text = self.text;
        self.at(
            from,
            StreamEventType::Scalar {
                value: Cow::Borrowed(&text[from..to]),
                style: ScalarStyle::Plain,
                anchor: None,
                tag: None,
            },
        );
    }

    /// A string whose text a decoder produced, reported with the style it was written in.
    fn decoded(&mut self, pos: usize, value: String, style: ScalarStyle) {
        self.at(
            pos,
            StreamEventType::Scalar {
                value: Cow::Owned(value),
                style,
                anchor: None,
                tag: None,
            },
        );
    }

    /// Report the notes one separation run carried, in source order, each at its own start.
    ///
    /// `ws_extended` records a note's text and whether it began on a line of its own, but not where
    /// it began, and adding an offset to `PendingComment` would charge the tree path for a fact only
    /// the stream asks for. The run is short and its grammar is one line of thought — whitespace plus
    /// `//…` and `/*…*/` — so the starts are re-read here and paired with the records in order.
    fn drain(&mut self, pending: &mut Vec<PendingComment>, run_start: usize, run_end: usize) {
        if pending.is_empty() {
            return;
        }
        let starts = comment_starts(self.text, run_start, run_end);
        for (index, note) in pending.drain(..).enumerate() {
            let pos = starts.get(index).copied().unwrap_or(run_end);
            self.at(
                pos,
                StreamEventType::Comment {
                    text: alloc::sync::Arc::from(note.text.as_str()),
                    standalone: note.own_line,
                },
            );
        }
    }
}

/// Describe a JSON document as the events it is made of, without building the node model.
///
/// The stream is the shared `pyrs-event` vocabulary, so it has the shape the YAML engine already
/// reports: `StreamStart` / `DocumentStart` around the value, one `DocumentEnd` / `StreamEnd` after
/// it, `Scalar` / `MappingStart`-`MappingEnd` / `SequenceStart`-`SequenceEnd` for the nodes, and
/// `Comment` for each note. `anchor` and `tag` are `None` throughout and `Alias` never appears —
/// those are YAML's grammar, and this is the second producer that says so rather than a guess about
/// the first.
///
/// Two reported facts are the source's own, not the hub's projection:
///
/// - `style` is how the scalar was *written*. A `"…"` string is `DoubleQuoted` even when the AST
///   stores it plain because `quoted_or_plain` saw no need for quotes, and a JSON5 `'k'` key is
///   `SingleQuoted` even though the hub records identifier and single-quoted keys alike as plain
///   text. The event keeps the spelling; the reader of the stream decides what to store.
/// - a JSON5 number arrives as its token (`Infinity`, `-Infinity`, `NaN`), while the tree stores the
///   hub's `.inf` / `-.inf` / `.nan`. `hub_scalar` is the one function that makes that projection
///   for both routes.
///
/// A number's text and a keyword's are the source slice; a string's is what the escapes decoded to.
/// Errors are the same `ParseError` the tree route reports for the same input, at the same position,
/// because both walks share the scanners and the dispatch order.
pub fn parse_stream(text: &str) -> Result<Vec<StreamEvent<'_>>, ParseError> {
    parse_stream_with_options(text, JsonParseOptions::STRICT)
}

/// Describe a JSON, JSONC or JSON5 document as events, with the dialect chosen by `opts`.
pub fn parse_stream_with_options(
    text: &str,
    opts: JsonParseOptions,
) -> Result<Vec<StreamEvent<'_>>, ParseError> {
    let mut p = parser_from(text, opts);
    let mut out = StreamOut::new(text);
    out.at(0, StreamEventType::StreamStart);
    out.at(0, StreamEventType::DocumentStart);
    let before = p.pos;
    p.ws();
    out.drain(&mut p.pending_comment, before, p.pos);
    p.stream_value(&mut out)?;
    let before = p.pos;
    p.ws();
    out.drain(&mut p.pending_comment, before, p.pos);
    if p.pos != p.s.len() {
        return Err(p.err("trailing characters after the JSON value"));
    }
    out.at(p.pos, StreamEventType::DocumentEnd);
    out.at(p.pos, StreamEventType::StreamEnd);
    Ok(out.events)
}

impl<'a> Parser<'a> {
    /// One nesting level of the stream walk, with the tree walk's budget and refusal.
    fn stream_value(&mut self, out: &mut StreamOut<'a>) -> Result<(), ParseError> {
        self.depth += 1;
        if self.depth > self.max_depth {
            self.depth -= 1;
            return Err(ParseError::MaxDepthExceeded(DepthError(self.max_depth)));
        }
        let descended = self.stream_value_inner(out);
        self.depth -= 1;
        descended
    }

    /// The dispatch of `value_inner`, reporting events instead of nodes.
    fn stream_value_inner(&mut self, out: &mut StreamOut<'a>) -> Result<(), ParseError> {
        let token = self.pos;
        match self.peek() {
            Some(b'{') => self.stream_object(out),
            Some(b'[') => self.stream_array(out),
            Some(b'"') => {
                let value = self.string()?;
                out.decoded(token, value, ScalarStyle::DoubleQuoted);
                Ok(())
            }
            Some(b'\'') if self.allow_single_quoted => {
                let value = self.single_quoted_string()?;
                out.decoded(token, value, ScalarStyle::SingleQuoted);
                Ok(())
            }
            Some(b't') => {
                self.expect("true", "expected literal `true`")?;
                out.plain(token, self.pos);
                Ok(())
            }
            Some(b'f') => {
                self.expect("false", "expected literal `false`")?;
                out.plain(token, self.pos);
                Ok(())
            }
            Some(b'n') => {
                self.expect("null", "expected literal `null`")?;
                out.plain(token, self.pos);
                Ok(())
            }
            Some(b'+') | Some(b'.') if self.allow_json5_numbers => self.stream_number(out),
            Some(b'I') if self.allow_json5_numbers => {
                self.expect("Infinity", "expected `Infinity`")?;
                out.plain(token, self.pos);
                Ok(())
            }
            Some(b'N') if self.allow_json5_numbers => {
                self.expect("NaN", "expected `NaN`")?;
                out.plain(token, self.pos);
                Ok(())
            }
            Some(b'-') | Some(b'0'..=b'9') => self.stream_number(out),
            _ => Err(self.err("expected a JSON value")),
        }
    }

    /// The grammar is `number_source`; only the report differs from the tree walk.
    fn stream_number(&mut self, out: &mut StreamOut<'a>) -> Result<(), ParseError> {
        let token = self.pos;
        let spelling = self.number_source()?;
        out.at(
            token,
            StreamEventType::Scalar {
                value: Cow::Borrowed(spelling),
                style: ScalarStyle::Plain,
                anchor: None,
                tag: None,
            },
        );
        Ok(())
    }

    /// The dispatch of `object`, with every separation run reported as it is consumed.
    fn stream_object(&mut self, out: &mut StreamOut<'a>) -> Result<(), ParseError> {
        let open = self.pos;
        self.pos += 1; // '{'
        out.at(
            open,
            StreamEventType::MappingStart {
                anchor: None,
                tag: None,
            },
        );
        let before = self.pos;
        self.ws();
        out.drain(&mut self.pending_comment, before, self.pos);
        if self.peek() == Some(b'}') {
            let close = self.pos;
            self.pos += 1;
            out.at(close, StreamEventType::MappingEnd);
            return Ok(());
        }
        loop {
            let before = self.pos;
            self.ws();
            out.drain(&mut self.pending_comment, before, self.pos);
            let token = self.pos;
            if self.peek() == Some(b'"') {
                let key = self.string()?;
                out.decoded(token, key, ScalarStyle::DoubleQuoted);
            } else if self.allow_single_quoted && self.peek() == Some(b'\'') {
                let key = self.single_quoted_string()?;
                out.decoded(token, key, ScalarStyle::SingleQuoted);
            } else if self.allow_unquoted_keys && self.at_identifier_start() {
                let key = self.identifier_key()?;
                out.decoded(token, key, ScalarStyle::Plain);
            } else {
                return Err(self.err("expected a quoted object key"));
            }
            let before = self.pos;
            self.ws();
            out.drain(&mut self.pending_comment, before, self.pos);
            self.expect(":", "expected `:` after the object key")?;
            let before = self.pos;
            self.ws();
            out.drain(&mut self.pending_comment, before, self.pos);
            self.stream_value(out)?;
            let before = self.pos;
            self.ws();
            out.drain(&mut self.pending_comment, before, self.pos);
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                    let before = self.pos;
                    self.ws();
                    out.drain(&mut self.pending_comment, before, self.pos);
                    if self.allow_trailing_commas && self.peek() == Some(b'}') {
                        let close = self.pos;
                        self.pos += 1;
                        out.at(close, StreamEventType::MappingEnd);
                        return Ok(());
                    }
                }
                Some(b'}') => {
                    let close = self.pos;
                    self.pos += 1;
                    out.at(close, StreamEventType::MappingEnd);
                    return Ok(());
                }
                _ => return Err(self.err("expected `,` or `}` in object")),
            }
        }
    }

    /// The dispatch of `array`, reported the same way.
    fn stream_array(&mut self, out: &mut StreamOut<'a>) -> Result<(), ParseError> {
        let open = self.pos;
        self.pos += 1; // '['
        out.at(
            open,
            StreamEventType::SequenceStart {
                anchor: None,
                tag: None,
            },
        );
        let before = self.pos;
        self.ws();
        out.drain(&mut self.pending_comment, before, self.pos);
        if self.peek() == Some(b']') {
            let close = self.pos;
            self.pos += 1;
            out.at(close, StreamEventType::SequenceEnd);
            return Ok(());
        }
        loop {
            let before = self.pos;
            self.ws();
            out.drain(&mut self.pending_comment, before, self.pos);
            self.stream_value(out)?;
            let before = self.pos;
            self.ws();
            out.drain(&mut self.pending_comment, before, self.pos);
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                    let before = self.pos;
                    self.ws();
                    out.drain(&mut self.pending_comment, before, self.pos);
                    if self.allow_trailing_commas && self.peek() == Some(b']') {
                        let close = self.pos;
                        self.pos += 1;
                        out.at(close, StreamEventType::SequenceEnd);
                        return Ok(());
                    }
                }
                Some(b']') => {
                    let close = self.pos;
                    self.pos += 1;
                    out.at(close, StreamEventType::SequenceEnd);
                    return Ok(());
                }
                _ => return Err(self.err("expected `,` or `]` in array")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::to_jsonc_text;
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
    fn a_duplicate_key_keeps_the_notes_its_own_nodes_carried() {
        // Found by `fuzz/fuzz_targets/json_events_replay.rs`: the collapse keeps the first key node
        // and the last value, so the incoming key and the value it replaced are both dropped, and
        // every note either owned used to be dropped with it. `first` rode the replaced value and
        // `before the repeat` the incoming key; both must still reach the text.
        let text = "{\n  \"a\": 1 // first\n,\n  // before the repeat\n  \"a\": 2\n}";
        let tree = from_jsonc(text).expect("valid JSONC");
        let written = to_jsonc_text(&tree).expect("writable");
        assert!(written.contains("first"), "{written}");
        assert!(written.contains("before the repeat"), "{written}");
        // and the second round writes the same text, which is what every writer in this crate owes
        let again = to_jsonc_text(&from_jsonc(&written).expect("its own output re-parses"))
            .expect("writable");
        assert_eq!(written, again);
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
    fn the_whitespace_guard_never_declines_what_the_loop_would() {
        // `ws` answers "nothing to skip" without calling `ws_extended`. That is sound only while
        // the loop's byte set stays inside the guard's, so ask both the same question over every
        // byte a token boundary can present, in every dialect that changes the answer, and require
        // the same position and the same number of pending notes afterwards.
        let mut cases: Vec<(JsonParseOptions, String)> = Vec::new();
        for opts in [
            JsonParseOptions::STRICT,
            JsonParseOptions::JSONC,
            JsonParseOptions::JSON5,
        ] {
            for byte in 0u8..=0x7f {
                cases.push((opts, format!("{}1", char::from(byte))));
            }
            // JSON5's high-byte separation - NBSP, a U+2000 separator, LS/PS, the BOM - and one
            // high character that is not separation at all, which the loop declines and the guard
            // over-calls for. Over-calling is the safe direction: it cannot move a position.
            for ch in [
                '\u{a0}', '\u{2000}', '\u{2028}', '\u{2029}', '\u{feff}', 'é',
            ] {
                cases.push((opts, format!("{ch}1")));
            }
            // The `/` answer depends on the flag and on what follows it, so both comment forms
            // and the bare slash are here.
            for head in ["//x\n", "/*x*/", "/x"] {
                cases.push((opts, format!("{head}1")));
            }
        }
        for (opts, src) in cases {
            let guarded = {
                let mut p = whitespace_probe(&src, opts);
                p.ws();
                (p.pos, p.pending_comment.len())
            };
            let direct = {
                let mut p = whitespace_probe(&src, opts);
                p.ws_extended();
                (p.pos, p.pending_comment.len())
            };
            assert_eq!(guarded, direct, "{src:?} under {opts:?}");
        }
    }

    /// A parser positioned at the start of `src`, with nothing read yet - the state both halves of
    /// the guard-equivalence test above need.
    fn whitespace_probe(src: &str, opts: JsonParseOptions) -> Parser<'_> {
        Parser {
            s: src.as_bytes(),
            text: src,
            pos: 0,
            depth: 0,
            max_depth: DEFAULT_MAX_DEPTH,
            allow_comments: opts.allow_comments,
            allow_trailing_commas: opts.allow_trailing_commas,
            allow_single_quoted: opts.allow_single_quoted,
            allow_unquoted_keys: opts.allow_unquoted_keys,
            allow_json5_numbers: opts.allow_json5_numbers,
            pending_comment: Vec::new(),
        }
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
    fn json5_single_quoted_runs_stop_only_where_the_decoding_loop_stopped() {
        // The scanner bulk-copies up to the next byte that needs a decision. Multi-byte text must
        // arrive as one slice, and the stop set must be the bytes it always was: a control
        // character is legal content in this dialect, so the run copies *through* one, while a
        // line terminator and the end of input still end the string as errors.
        let obj = from_json5("{ 'a\\tb\u{1}c': 'héllo — 名前', 'd': '尾' }").unwrap();
        let CustomNode::Mapping { pairs, .. } = &obj else {
            unreachable!()
        };
        let got: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| match (k, v) {
                (CustomNode::Scalar { value: k, .. }, CustomNode::Scalar { value: v, .. }) => {
                    (k.to_string(), v.to_string())
                }
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(
            got,
            vec![
                ("a\tb\u{1}c".to_string(), "héllo — 名前".to_string()),
                ("d".to_string(), "尾".to_string()),
            ]
        );

        for bad in ["{'a': 'é", "{'a': 'é\n'}", "['尾"] {
            assert!(
                matches!(from_json5(bad), Err(ParseError::Syntax { .. })),
                "{bad:?} parsed instead of rejecting"
            );
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
    fn a_run_of_same_line_comments_keeps_every_note() {
        // Issue #361. Two comments sharing a line used to compete for the single inline slot and each
        // overwrote the last, so `{/* a *//* b */}` reached the AST with one note - which is how the
        // compact writer lost comments on re-reading its own output. Measured placement after the fix:
        // both notes on the value's leading list in source order, inline slot empty.
        let ast = crate::from_jsonc("{\n\"k\": 1 /* unit */ /* owner */\n}").unwrap();
        let pyrs_ast::ast::CustomNode::Mapping { pairs, .. } = &ast else {
            panic!("an object was expected")
        };
        let (_, value) = pairs.iter().next().expect("one pair");
        let notes: Vec<&str> = value
            .leading_comments()
            .iter()
            .map(|c| c.text.as_ref())
            .collect();
        assert_eq!(
            notes,
            ["unit", "owner"],
            "the run of same-line notes did not survive"
        );
        assert_eq!(value.comment().map(|c| c.text.as_ref()), None);
        // A lone same-line comment is the spelling a human writes, and still owns the inline slot.
        let one = crate::from_jsonc("{\n\"k\": 1 // port\n}").unwrap();
        let pyrs_ast::ast::CustomNode::Mapping { pairs, .. } = &one else {
            panic!("an object was expected")
        };
        let (_, value) = pairs.iter().next().expect("one pair");
        assert_eq!(value.comment().map(|c| c.text.as_ref()), Some("port"));
        assert!(value.leading_comments().is_empty());
    }

    #[test]
    fn json5_rejects_leading_zeros_exactly_like_strict() {
        // The reversal of a decision that was pinned in the wrong direction: JSON5 permits `0x` and
        // `.5` and `5.`, but its decimal integer is `0 | NonZeroDigit DigitalDigits?`. `pyjson5`
        // rejects every case below, and so does the corpus file `n_number_with_leading_zero.json`.
        for src in ["07", "012", "-01", "-012", "+07"] {
            assert!(
                from_json5(src).is_err(),
                "{src} is not a JSON5 number: leading zero"
            );
            assert!(
                from_json(src).is_err(),
                "{src} must stay invalid strict JSON"
            );
        }
        // The two `0`-prefixed forms that ECMAScript has but JSON5's grammar does
        // not: the reference implementation rejects these too, so the only latitude
        // this parser keeps on the zero prefix is hexadecimal.
        for src in ["0o17", "0b101", "-0o17", "0O17"] {
            assert!(
                from_json5(src).is_err(),
                "{src} is not a JSON5 number: no octal or binary prefix in the grammar"
            );
        }
        // What the zero prefix *may* legally do: stand alone, take a fraction or exponent, or be the
        // start of a hexadecimal literal that JSON5 does allow.
        for src in ["0", "-0", "0.5", "0e1", "0E+1", "0x0", "0XFF", "-0x1F"] {
            assert!(
                from_json5(src).is_ok(),
                "{src} must stay valid JSON5: {:?}",
                from_json5(src).err()
            );
        }
    }

    // --- the tree-free event stream -----------------------------------------

    /// One event reduced to what a caller can compare: its kind, its text, and no position.
    fn shape(event: &StreamEvent<'_>) -> String {
        match &event.event_type {
            StreamEventType::StreamStart => "stream-start".into(),
            StreamEventType::StreamEnd => "stream-end".into(),
            StreamEventType::DocumentStart => "document-start".into(),
            StreamEventType::DocumentEnd => "document-end".into(),
            StreamEventType::MappingStart { anchor, tag } => {
                format!("mapping-start[{anchor:?},{tag:?}]")
            }
            StreamEventType::MappingEnd => "mapping-end".into(),
            StreamEventType::SequenceStart { anchor, tag } => {
                format!("sequence-start[{anchor:?},{tag:?}]")
            }
            StreamEventType::SequenceEnd => "sequence-end".into(),
            StreamEventType::Scalar {
                value,
                style,
                anchor,
                tag,
            } => {
                assert!(
                    anchor.is_none() && tag.is_none(),
                    "a JSON scalar cannot own an anchor or a tag: {value}"
                );
                format!("scalar[{style:?}]={value}")
            }
            StreamEventType::Alias { name } => panic!("JSON has no alias: {name}"),
            StreamEventType::Comment { text, standalone } => {
                format!("comment[standalone={standalone}]={text}")
            }
        }
    }

    /// Replay a stream onto the node model: the same `NodeMap`, the same `quoted_or_plain`, the same
    /// `hub_scalar` and the same single-quoted arm the tree walk uses, so a disagreement between the
    /// two routes is a disagreement about the document rather than about a copy of a rule.
    ///
    /// `Comment` events are not placed: which node a same-line note rides is exactly what the stream
    /// cannot report — `a_same_line_note_between_a_value_and_the_next_member_has_two_readings` names
    /// that — so the replay compares the trees with their note slots emptied
    /// (`undecorated`) and counts the notes themselves in
    /// `the_stream_loses_and_invents_no_note`.
    fn fold(events: &[StreamEvent<'_>]) -> CustomNode {
        #[derive(Debug)]
        enum Open {
            Map(NodeMap<CustomNode, CustomNode>, Option<CustomNode>),
            Seq(Vec<CustomNode>),
        }
        fn place(stack: &mut [Open], root: &mut Option<CustomNode>, node: CustomNode) {
            match stack.last_mut() {
                // Inside a mapping the scalars that are not containers arrive in pairs: name, value.
                Some(Open::Map(pairs, name)) => match name.take() {
                    None => *name = Some(node),
                    Some(name) => {
                        pairs.insert(name, node);
                    }
                },
                Some(Open::Seq(items)) => items.push(node),
                None => *root = Some(node),
            }
        }
        let mut stack: Vec<Open> = Vec::new();
        let mut root: Option<CustomNode> = None;
        for event in events {
            match &event.event_type {
                StreamEventType::MappingStart { .. } => {
                    stack.push(Open::Map(NodeMap::default(), None));
                }
                StreamEventType::SequenceStart { .. } => stack.push(Open::Seq(Vec::new())),
                StreamEventType::MappingEnd => match stack.pop() {
                    Some(Open::Map(pairs, _)) => {
                        place(&mut stack, &mut root, CustomNode::plain_mapping(pairs))
                    }
                    other => panic!("a mapping closed where {other:?} was open"),
                },
                StreamEventType::SequenceEnd => match stack.pop() {
                    Some(Open::Seq(items)) => {
                        place(&mut stack, &mut root, CustomNode::plain_sequence(items))
                    }
                    other => panic!("a sequence closed where {other:?} was open"),
                },
                StreamEventType::Scalar { value, style, .. } => {
                    // A mapping's name is the hub's plain-or-quoted text whatever it was written as
                    // — an identifier, a `'k'` or a `"k"` all reach `quoted_or_plain` (the tree's
                    // `object` does exactly this) — while a value keeps the spelling it arrived in.
                    let as_name = matches!(stack.last(), Some(Open::Map(_, None)));
                    let node = if as_name {
                        quoted_or_plain(value.to_string())
                    } else {
                        match style {
                            ScalarStyle::SingleQuoted => single_quoted_node(value.to_string()),
                            ScalarStyle::DoubleQuoted => quoted_or_plain(value.to_string()),
                            // The tree stores the word `null` as a null node, and every other plain
                            // scalar as the hub's own spelling of the token.
                            ScalarStyle::Plain => {
                                if *value == "null" {
                                    CustomNode::plain_null()
                                } else {
                                    hub_scalar(value)
                                }
                            }
                            other => panic!("JSON never writes a scalar in {other:?}"),
                        }
                    };
                    place(&mut stack, &mut root, node);
                }
                StreamEventType::Comment { .. }
                | StreamEventType::StreamStart
                | StreamEventType::StreamEnd
                | StreamEventType::DocumentStart
                | StreamEventType::DocumentEnd => {}
                StreamEventType::Alias { name } => panic!("JSON has no alias: {name}"),
            }
        }
        root.expect("a document's stream reports its root value")
    }

    /// The same tree with every note slot emptied.
    ///
    /// `CustomNode`'s equality does compare decorations — only `source_range` is excluded — so a
    /// replay that cannot report which node owns a same-line note needs this to be asked about what
    /// both routes do settle: structure, order, values and styles.
    fn undecorated(node: &CustomNode) -> CustomNode {
        match node {
            CustomNode::Scalar {
                value,
                style,
                chomping,
                block_indent,
                ..
            } => CustomNode::Scalar {
                value: value.clone(),
                style: *style,
                chomping: *chomping,
                block_indent: *block_indent,
                meta: pyrs_ast::ast::NodeMeta::default(),
            },
            CustomNode::Mapping {
                pairs, flow_style, ..
            } => CustomNode::Mapping {
                pairs: pairs
                    .iter()
                    .map(|(name, value)| (undecorated(name), undecorated(value)))
                    .collect(),
                flow_style: *flow_style,
                meta: pyrs_ast::ast::NodeMeta::default(),
            },
            CustomNode::Sequence {
                items, flow_style, ..
            } => CustomNode::Sequence {
                items: items.iter().map(undecorated).collect(),
                flow_style: *flow_style,
                meta: pyrs_ast::ast::NodeMeta::default(),
            },
            other => other.clone(),
        }
    }

    /// Every note a tree carries, as text and standalone flag, in node order.
    fn tree_notes(node: &CustomNode, into: &mut Vec<(String, bool)>) {
        if let Some(comment) = node.comment() {
            into.push((comment.text.to_string(), comment.standalone));
        }
        for comment in node.leading_comments() {
            into.push((comment.text.to_string(), comment.standalone));
        }
        match node {
            CustomNode::Mapping { pairs, .. } => {
                for (name, value) in pairs.iter() {
                    tree_notes(name, into);
                    tree_notes(value, into);
                }
            }
            CustomNode::Sequence { items, .. } => {
                for item in items {
                    tree_notes(item, into);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn the_stream_reports_the_document_in_order() {
        let text = "{\n \"a\": [1],\n \"b\": null\n}";
        let events = parse_stream(text).expect("valid strict JSON");
        let read: Vec<String> = events
            .iter()
            .map(|event| format!("{}@{}:{}", shape(event), event.line, event.column,))
            .collect();
        // Positions are the granit convention the crate's own diagnostics use: 0-indexed line,
        // column counted in characters from the start of that line.
        assert_eq!(
            read,
            [
                "stream-start@0:0",
                "document-start@0:0",
                "mapping-start[None,None]@0:0",
                "scalar[DoubleQuoted]=a@1:1",
                "sequence-start[None,None]@1:6",
                "scalar[Plain]=1@1:7",
                "sequence-end@1:8",
                "scalar[DoubleQuoted]=b@2:1",
                "scalar[Plain]=null@2:6",
                "mapping-end@3:0",
                "document-end@3:1",
                "stream-end@3:1",
            ]
        );
    }

    #[test]
    fn a_number_borrows_the_source_and_a_decoded_string_owns_its_text() {
        let text = r#"{"n": 12345, "s": "a\u0062", "t": true}"#;
        let events = parse_stream(text).expect("valid strict JSON");
        // The scalar that follows a given name, reported the way the stream reports it.
        let value_of = |name: &str| -> StreamEventType<'_> {
            events
                .iter()
                .zip(events.iter().skip(1))
                .find(|(key, _)| match &key.event_type {
                    StreamEventType::Scalar { value, .. } => **value == *name,
                    _ => false,
                })
                .map(|(_, value)| value.event_type.clone())
                .unwrap_or_else(|| panic!("no member named {name} in the stream"))
        };
        let StreamEventType::Scalar {
            value,
            style: ScalarStyle::Plain,
            ..
        } = value_of("n")
        else {
            panic!("12345 must arrive as a plain scalar");
        };
        let Cow::Borrowed(number) = value else {
            panic!("a number is the source's own bytes, not a decoded copy");
        };
        assert_eq!(number, "12345");
        let StreamEventType::Scalar {
            value,
            style: ScalarStyle::Plain,
            ..
        } = value_of("t")
        else {
            panic!("true must arrive as a plain scalar");
        };
        let Cow::Borrowed(word) = value else {
            panic!("a keyword is the source's own bytes");
        };
        assert_eq!(word, "true");
        let StreamEventType::Scalar {
            value,
            style: ScalarStyle::DoubleQuoted,
            ..
        } = value_of("s")
        else {
            panic!("\"a\\u0062\" must arrive as a double-quoted scalar");
        };
        let Cow::Owned(decoded) = value else {
            panic!("an escaped string has no single source slice to borrow");
        };
        assert_eq!(decoded, "ab");
    }

    #[test]
    fn a_json_stream_carries_no_anchor_tag_or_alias_and_reports_every_dialect_shape() {
        // JSON5 with each axis on, so no scalar form is left unasked.
        let text =
            "{a: 'k', /*note*/ b: [0x1F, .5, Infinity, -Infinity, NaN, +7, 5.], \"c\": {'d': 'e'}}";
        let events = parse_stream_with_options(text, JsonParseOptions::JSON5).expect("valid JSON5");
        let shapes: Vec<String> = events.iter().map(shape).collect();
        assert!(
            shapes
                .iter()
                .all(|line| !line.contains("Alias") && !line.contains("Some(")),
            "the vocabulary's YAML-only fields must stay empty: {shapes:?}"
        );
        // The token is what the stream reports; `.inf` is the tree's projection of it.
        assert!(shapes.contains(&"scalar[Plain]=Infinity".to_string()));
        assert!(shapes.contains(&"scalar[Plain]=-Infinity".to_string()));
        assert!(shapes.contains(&"scalar[Plain]=NaN".to_string()));
        assert!(shapes.contains(&"scalar[Plain]=0x1F".to_string()));
        // A key written as an identifier or in single quotes keeps its own spelling in the stream,
        // which is the fact the hub discards.
        assert!(shapes.contains(&"scalar[Plain]=a".to_string()));
        assert!(shapes.contains(&"scalar[SingleQuoted]=k".to_string()));
        assert!(shapes.contains(&"scalar[SingleQuoted]=d".to_string()));
        assert!(shapes.contains(&"comment[standalone=false]=note".to_string()));
    }

    #[test]
    fn the_stream_replays_the_tree_the_direct_parser_builds() {
        // Documents with no note in them: the replay is exact, decorations included.
        let plain: &[(JsonParseOptions, &str)] = &[
            (JsonParseOptions::STRICT, "{}"),
            (JsonParseOptions::STRICT, "[]"),
            (JsonParseOptions::STRICT, "null"),
            (JsonParseOptions::STRICT, "  1e3  "),
            (
                JsonParseOptions::STRICT,
                r#"{"a":-0,"b":[1,2,{"c":"x"}],"d":null}"#,
            ),
            (JsonParseOptions::STRICT, r#"{"k":1,"k":2,"k":3}"#),
            (
                JsonParseOptions::STRICT,
                r#"{"true":1,"1":"x","Infinity":2}"#,
            ),
            (
                JsonParseOptions::STRICT,
                r#"{"n":[0.5e-3,-1.0E+2],"s":"A","t":true}"#,
            ),
            (
                JsonParseOptions::JSON5,
                "{a: 'b', c: [Infinity, -0x1, .5, 5.]}",
            ),
            (JsonParseOptions::JSON5, "{'k':'v','k':'w'}"),
            (JsonParseOptions::JSON5, "[1,2,]"),
            (JsonParseOptions::JSON5, "{\"深\": \"刻\", é: 1}"),
        ];
        for (opts, text) in plain {
            let tree = from_json_with_options(text, *opts)
                .unwrap_or_else(|e| panic!("{text} must parse: {e}"));
            let events = parse_stream_with_options(text, *opts)
                .unwrap_or_else(|e| panic!("{text} must stream: {e}"));
            assert_eq!(
                fold(&events),
                tree,
                "the stream and the tree disagree about {text}"
            );
        }
        // Documents carrying notes: the two routes agree on everything a stream can report — the
        // slot a same-line note rides is the one thing it cannot.
        let noted: &[(JsonParseOptions, &str)] = &[
            (JsonParseOptions::JSONC, "{/*a*/}"),
            (JsonParseOptions::JSONC, "{\"a\": /*b*/ 1}"),
            (JsonParseOptions::JSONC, "[1 /* trailing */, 2]"),
            (
                JsonParseOptions::JSON5,
                "{/*k*/ a: /*v*/ 'b' /*t*/, c: 1 // end\n}",
            ),
        ];
        for (opts, text) in noted {
            let tree = from_json_with_options(text, *opts)
                .unwrap_or_else(|e| panic!("{text} must parse: {e}"));
            let events = parse_stream_with_options(text, *opts)
                .unwrap_or_else(|e| panic!("{text} must stream: {e}"));
            assert_eq!(
                undecorated(&fold(&events)),
                undecorated(&tree),
                "the stream and the tree disagree about {text}"
            );
        }
    }

    #[test]
    fn both_walks_refuse_the_same_input_at_the_same_place() {
        let rejected: &[(JsonParseOptions, &str)] = &[
            (JsonParseOptions::STRICT, ""),
            (JsonParseOptions::STRICT, "   "),
            (JsonParseOptions::STRICT, "{"),
            (JsonParseOptions::STRICT, "}"),
            (JsonParseOptions::STRICT, "["),
            (JsonParseOptions::STRICT, "[,]"),
            (JsonParseOptions::STRICT, "{,}"),
            (JsonParseOptions::STRICT, "{\"a\"}"),
            (JsonParseOptions::STRICT, "{\"a\":}"),
            (JsonParseOptions::STRICT, "{\"a\":1,}"),
            (JsonParseOptions::STRICT, "[1,2,]"),
            (JsonParseOptions::STRICT, "01"),
            (JsonParseOptions::STRICT, "-"),
            (JsonParseOptions::STRICT, "1e"),
            (JsonParseOptions::STRICT, "1 2"),
            (JsonParseOptions::STRICT, "\"unterminated"),
            (JsonParseOptions::STRICT, "\"\\q\""),
            (JsonParseOptions::STRICT, "// line"),
            (JsonParseOptions::STRICT, "/* block"),
            (JsonParseOptions::STRICT, "Infinity"),
            (JsonParseOptions::STRICT, "'single'"),
            (JsonParseOptions::STRICT, "{a: 1}"),
            (JsonParseOptions::JSONC, "{a: 1}"),
            (JsonParseOptions::JSONC, "{'k': 1}"),
            (JsonParseOptions::JSONC, "[1,]"),
            (JsonParseOptions::JSONC, "0x1F"),
            (JsonParseOptions::JSON5, "{\"a\": \"\\ud800\"}"),
            (JsonParseOptions::JSON5, ".5e"),
            (JsonParseOptions::JSON5, "-Infinityx"),
            (JsonParseOptions::JSON5, "{\"a\": }}"),
            (
                JsonParseOptions {
                    max_depth: 2,
                    ..JsonParseOptions::STRICT
                },
                "[[[1]]]",
            ),
            (
                JsonParseOptions {
                    max_depth: 1,
                    ..JsonParseOptions::JSON5
                },
                "{a: [1]}",
            ),
        ];
        for (opts, text) in rejected {
            let tree = from_json_with_options(text, *opts);
            let stream = parse_stream_with_options(text, *opts);
            match (tree, stream) {
                (Err(first), Err(second)) => {
                    assert_eq!(first, second, "{text} refused differently")
                }
                (Ok(_), Ok(_)) => panic!("{text} was expected to be refused"),
                (tree, stream) => panic!(
                    "{text}: the tree route said {:?} and the stream route said {:?}",
                    tree.as_ref().map(|_| "ok"),
                    stream.as_ref().map(|_| "ok")
                ),
            }
        }
    }

    #[test]
    fn the_stream_loses_and_invents_no_note() {
        let noted: &[(JsonParseOptions, &str)] = &[
            (JsonParseOptions::JSONC, "// above\n{}"),
            (JsonParseOptions::JSONC, "{} // after"),
            (JsonParseOptions::JSONC, "{\"a\": /*x*/ 1 /*y*/}"),
            (
                JsonParseOptions::JSONC,
                "{\"a\": [\n  // line\n  1,\n  /* block */ 2\n] /* tail */}",
            ),
            (JsonParseOptions::JSONC, "[/* one */ /* two */ 1]"),
            (
                JsonParseOptions::JSON5,
                "{/*k*/ a: /*v*/ 'b' /*t*/, c: 1 // end\n}",
            ),
            (JsonParseOptions::JSON5, "[1, /* between */ 2]"),
        ];
        for (opts, text) in noted {
            let tree = from_json_with_options(text, *opts)
                .unwrap_or_else(|e| panic!("{text} must parse: {e}"));
            let mut kept = Vec::new();
            tree_notes(&tree, &mut kept);
            let mut from_tree: Vec<String> = kept.into_iter().map(|(text, _alone)| text).collect();
            let mut from_stream: Vec<String> = parse_stream_with_options(text, *opts)
                .unwrap_or_else(|e| panic!("{text} must stream: {e}"))
                .iter()
                .filter_map(|event| match &event.event_type {
                    StreamEventType::Comment { text, .. } => Some(text.to_string()),
                    _ => None,
                })
                .collect();
            from_tree.sort();
            from_stream.sort();
            assert_eq!(
                from_tree, from_stream,
                "the notes the tree keeps and the notes the stream reports must be the same set of {text}"
            );
        }
    }

    #[test]
    fn a_note_reports_whether_it_began_on_a_line_of_its_own() {
        // The flag is what the scanner saw, which is not always the flag the tree ends up recording:
        // two same-line notes on one node are re-filed as leading notes (issue #361), and the stream
        // keeps the reading the bytes gave.
        let text = "{\n  // own line\n  \"a\": 1, /* same line */ \"b\": 2\n}";
        let stream = parse_stream_with_options(text, JsonParseOptions::JSONC).expect("valid JSONC");
        let notes: Vec<(String, bool)> = stream
            .iter()
            .filter_map(|event| match &event.event_type {
                StreamEventType::Comment { text, standalone } => {
                    Some((text.to_string(), *standalone))
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            notes,
            [
                ("own line".to_string(), true),
                ("same line".to_string(), false)
            ]
        );
    }

    #[test]
    fn a_same_line_note_between_a_value_and_the_next_member_has_two_readings() {
        // Both documents produce the same events in the same order: the value, then the note, then
        // the next name. The tree has to choose a slot, and it chooses by which side of the comma
        // the note sat - a fact a note's own position cannot carry. This is what the shared
        // vocabulary still cannot express, and the reason the replay compares structure and values
        // and counts notes rather than placing them.
        let before_comma = r#"{"a": 1 /* x */, "b": 2}"#;
        let after_comma = r#"{"a": 1, /* x */ "b": 2}"#;
        let shapes_of = |text: &str| -> Vec<String> {
            parse_stream_with_options(text, JsonParseOptions::JSONC)
                .expect("valid JSONC")
                .iter()
                .map(shape)
                .collect()
        };
        assert_eq!(shapes_of(before_comma), shapes_of(after_comma));

        // Which node each note landed on, as (owner, text, standalone).
        let owners_of = |text: &str| -> Vec<(String, String, bool)> {
            let tree = from_json_with_options(text, JsonParseOptions::JSONC).expect("valid JSONC");
            let CustomNode::Mapping { pairs, .. } = tree else {
                panic!("{text} is not a mapping");
            };
            let mut owners = Vec::new();
            for (name, value) in pairs.iter() {
                for (owner, node) in [("name", name), ("value", value)] {
                    let mut kept = Vec::new();
                    tree_notes(node, &mut kept);
                    for (note, alone) in kept {
                        owners.push((owner.to_string(), note, alone));
                    }
                }
            }
            owners
        };
        let only = |owner: &str| vec![(owner.to_string(), "x".to_string(), false)];
        assert_eq!(owners_of(before_comma), only("value"));
        assert_eq!(owners_of(after_comma), only("name"));
    }

    #[cfg(feature = "std")]
    #[test]
    fn the_seed_corpus_and_the_stream_agree_on_what_is_valid() {
        // The same inputs the writer fixed point runs over, asked of the second walk: whatever the
        // corpus holds, both routes must accept it together, refuse it together with the same
        // diagnostic, and — when it parses — replay to the tree the direct parser built.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fuzz/seeds/json_roundtrip");
        let mut paths: Vec<_> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("seed corpus missing at {}: {e}", dir.display()))
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.extension().is_some_and(|ext| ext == "seed"))
            .collect();
        paths.sort();
        assert!(!paths.is_empty(), "the JSON seed corpus is empty");
        let mut replayed = 0usize;
        for path in paths {
            let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let text = String::from_utf8_lossy(&bytes).into_owned();
            for opts in [
                JsonParseOptions::STRICT,
                JsonParseOptions::JSONC,
                JsonParseOptions::JSON5,
            ] {
                let tree = from_json_with_options(&text, opts);
                let stream = parse_stream_with_options(&text, opts);
                match (tree, stream) {
                    (Ok(node), Ok(events)) => {
                        assert_eq!(
                            undecorated(&fold(&events)),
                            undecorated(&node),
                            "{} [{opts:?}]: the stream replays a different document",
                            path.display()
                        );
                        replayed += 1;
                    }
                    (Err(first), Err(second)) => assert_eq!(
                        first,
                        second,
                        "{} [{opts:?}]: the two routes refused differently",
                        path.display()
                    ),
                    (tree, stream) => panic!(
                        "{} [{opts:?}]: one route accepted and the other refused ({:?} / {:?})",
                        path.display(),
                        tree.is_ok(),
                        stream.is_ok()
                    ),
                }
            }
        }
        assert!(
            replayed >= 25,
            "the corpus only replayed {replayed} documents; it used to carry at least 25"
        );
    }
}
