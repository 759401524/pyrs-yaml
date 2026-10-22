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
use pyrs_event::{LineTable, NoteOwner, StreamEvent, StreamEventType};
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
        inline_comments: 0,
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
    /// How many same-line comments `ws()` has consumed. `object` snapshots this on entry and folds
    /// its own map on exit when the count moved: a key's same-line note is part of node equality,
    /// so it can keep a repeated name from collapsing at insert time. Only the cold comment path
    /// writes here, and a document without comments never changes the count.
    inline_comments: usize,
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
    let mut ghost = quoted_or_plain(Cow::Borrowed(""));
    attach_pending(&mut ghost, pending);
    notes_of(&ghost)
}

/// Whether two key nodes name the same member.
///
/// A parsed key node derives its style and every other equality field from its text; only the
/// inline note slot can make two same-name keys unequal. The map's own collapse tests node
/// equality, so without this predicate a note on one key kept a repeated name alive - and a
/// re-read of the strict writer's output folded it, which is not a fixed point.
fn same_key_name(a: &CustomNode, b: &CustomNode) -> bool {
    match (a, b) {
        (CustomNode::Scalar { value: left, .. }, CustomNode::Scalar { value: right, .. }) => {
            left == right
        }
        _ => false,
    }
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

/// Collapse a repeated name inside one mapping after a key's note kept the two apart.
///
/// `insert_full` collapses on node equality and a key's same-line note is part of it, so a repeated
/// name with one on either key stayed twice - the strict writer then emitted the name twice and a
/// re-read folded it, which is not a fixed point. `object` runs this when a note reached one of its
/// keys, which is a sound superset (a value's note also triggers it).
///
/// One side of every such split carries a note, so only noted keys are matched against the rest;
/// clean keys are skipped with one slot read. The all-pairs scan this replaces was measured at 224
/// instructions per `from_jsonc_medium` parse.
fn collapse_noted_map(pairs: &mut NodeMap<CustomNode, CustomNode>) {
    let mut index = 0;
    while index < pairs.len() {
        let noted = match pairs.get_index(index) {
            Some((key, _)) => key.comment().is_some(),
            None => break,
        };
        if !noted {
            index += 1;
            continue;
        }
        // Every later duplicate of this name folds into this entry - the name's first position -
        // before the entry itself may fold back into an earlier one.
        let later = (index + 1..pairs.len()).find(|&candidate| {
            match (pairs.get_index(index), pairs.get_index(candidate)) {
                (Some((first, _)), Some((other, _))) => same_key_name(first, other),
                _ => false,
            }
        });
        if let Some(later) = later {
            let Some((key_node, value_node)) = pairs.shift_remove_index(later) else {
                break;
            };
            let Some((_, slot)) = pairs.get_index_mut(index) else {
                break;
            };
            let displaced = core::mem::replace(slot, value_node);
            let carried = notes_of(&key_node);
            park_notes(pairs, index, displaced, carried);
            // Stay here: another duplicate of the same name may follow.
            continue;
        }
        // No later duplicate, so this noted entry may belong to an earlier clean key.
        let earlier = match pairs.get_index(index) {
            Some((key, _)) => pairs
                .keys()
                .take(index)
                .position(|seen| same_key_name(seen, key)),
            None => break,
        };
        let Some(earlier) = earlier else {
            index += 1;
            continue;
        };
        let Some((key_node, value_node)) = pairs.shift_remove_index(index) else {
            break;
        };
        let Some((_, slot)) = pairs.get_index_mut(earlier) else {
            break;
        };
        let displaced = core::mem::replace(slot, value_node);
        let carried = notes_of(&key_node);
        park_notes(pairs, earlier, displaced, carried);
        // The entry now at this index was never examined.
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
    /// The source offset the comment token starts at.
    ///
    /// The tree route never reads it - it files a note by the rule, not by the
    /// position - but the stream route reports every note at its own start, and
    /// one group can span a value's text, so the offset has to travel with the
    /// record instead of being re-read out of the source afterwards.
    at: usize,
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
                            at: body_start - 2,
                        });
                        if !own_line_seen {
                            self.inline_comments += 1;
                        }
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
                            at: body_start - 2,
                        });
                        if !own_line_seen {
                            self.inline_comments += 1;
                        }
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
    fn single_quoted_string(&mut self) -> Result<Cow<'a, str>, ParseError> {
        debug_assert_eq!(self.peek(), Some(b'\''));
        self.pos += 1;
        let content = self.pos;
        // `None` means the source has held no escape so far, so the body is still borrowable;
        // the first escape materialises the decoded buffer with the literal run before it.
        let mut out: Option<String> = None;
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
            if self.pos > start
                && let Some(out) = &mut out
            {
                out.push_str(&self.text[start..self.pos]);
            }
            let b = self.peek().ok_or_else(|| self.err("unterminated string"))?;
            match b {
                b'\'' => {
                    let end = self.pos;
                    self.pos += 1;
                    return Ok(match out {
                        Some(out) => Cow::Owned(out),
                        None => Cow::Borrowed(&self.text[content..end]),
                    });
                }
                b'\\' => {
                    let out = out.get_or_insert_with(|| {
                        let mut decoded = String::with_capacity(self.pos - content + 8);
                        decoded.push_str(&self.text[content..self.pos]);
                        decoded
                    });
                    self.pos += 1;
                    if self.line_continuation() {
                        // The continuation contributes nothing to the decoded text. This scanner is
                        // JSON5-only, so unlike the double-quoted twin it needs no preset guard.
                        continue;
                    }
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
                        b'u' => self.unicode_escape(&mut *out)?,
                        // JSON5-only string escapes (allow_json5_numbers is
                        // true only for the JSON5 preset): vertical tab, NUL
                        // and the two-digit hex escape. Strict JSON / JSONC
                        // still reject them.
                        b'v' if self.allow_json5_numbers => out.push('\u{b}'),
                        b'0' if self.allow_json5_numbers => out.push('\0'),
                        b'x' if self.allow_json5_numbers => self.x_escape(&mut *out)?,
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
    /// name: an ASCII `A-Z a-z _ $`, any Unicode `ID_Start` (PR #129), or the
    /// backslash that opens an escape in the identifier position. Used only as
    /// a cheap dispatch guard; `identifier_key` re-checks exactly.
    fn at_identifier_start(&self) -> bool {
        match self.peek() {
            Some(b'A'..=b'Z' | b'a'..=b'z' | b'_' | b'$') => true,
            // Only the JSON5 preset reaches an identifier position with a backslash in it; strict
            // JSON / JSONC have no unquoted keys to escape in, so the guard cannot widen them.
            Some(b'\\') => self.allow_json5_numbers,
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

    /// One identifier character, either a literal code point or a backslash-u escape, where `first`
    /// says which position in the name it is. `Ok(None)` means the next character is not part of the
    /// identifier, which is how `identifier_key` ends.
    ///
    /// The asymmetry is the ES5 grammar JSON5 inherits: a *literal* start must be `ID_Start` and a
    /// literal continuation `ID_Continue`, while an *escaped* character needs no such check at all -
    /// `IdentifierStart :: ... | backslash UnicodeEscapeSequence` accepts any scalar, so an escaped
    /// digit, an escaped dot and an escaped space are all legal keys and decode to `1`, `.` and ` `,
    /// while an escaped surrogate half and a truncated escape are rejected. Only the JSON5 preset
    /// takes this path, because the dialects without unquoted keys never reach it.
    fn ident_char(&mut self, first: bool) -> Result<Option<char>, ParseError> {
        if self.allow_json5_numbers && self.peek() == Some(b'\\') {
            let at = self.pos;
            self.pos += 1;
            if self.peek() != Some(b'u') {
                // Leave the backslash in place: the caller's error names the key position, and a
                // half-consumed escape would move where that points.
                self.pos = at;
                return Err(self.err("only a backslash-u escape is valid in an identifier key"));
            }
            self.pos += 1;
            let code = self.hex4()?;
            let Some(ch) = char::from_u32(code) else {
                return Err(self.err("identifier escape is not a Unicode scalar value"));
            };
            return Ok(Some(ch));
        }
        let Some(ch) = self.text[self.pos..].chars().next() else {
            return Ok(None);
        };
        let valid = ch == '$'
            || ch == '_'
            || (first && unicode_ident::is_xid_start(ch))
            || (!first && unicode_ident::is_xid_continue(ch));
        if !valid {
            return Ok(None);
        }
        self.pos += ch.len_utf8();
        Ok(Some(ch))
    }

    /// JSON5 identifier key (unquoted); `ident_char` holds the character rule. PR #129 lifted the
    /// former ASCII-only limit to full Unicode via the `unicode-ident` tables, so e.g.
    /// `{ é: 1, 名: 2, हिन्दी: 3 }` parses, and the escape path is what adds escaped
    /// keys on top of that.
    ///
    /// The plain run is the loop it was before escapes existed, and that is measured rather than
    /// stylistic: checking for a backslash on every character cost +7.18% on `from_json5_medium` at
    /// the Ir gate, while `from_json_medium` and `from_jsonc_medium` - which share the string scanner
    /// and its escape arms but not this loop - moved by -0.07%. So the escape is looked for only
    /// where the loop stops. Then both handoffs moved onto branches a plain key already takes: a
    /// leading backslash is not `ID_Start`, so the entry test lives in the branch that rejects it,
    /// and that second split is the difference between +2.54% and +0.32% on the same scenario.
    fn identifier_key(&mut self) -> Result<Cow<'a, str>, ParseError> {
        let start = self.pos;
        let first = self.text[start..]
            .chars()
            .next()
            .ok_or_else(|| self.err("expected an identifier key"))?;
        if !(first == '$' || first == '_' || unicode_ident::is_xid_start(first)) {
            // A backslash is not `ID_Start`, so an escaped key is taken on the branch that already
            // rejects it. Checked here rather than as an extra test before the loop, because that
            // order makes every plain key pay one branch for a spelling it never sees.
            if self.allow_json5_numbers && first == '\\' {
                return self.escaped_identifier_tail("");
            }
            return Err(self.err("expected an identifier key"));
        }
        self.pos += first.len_utf8();
        while self.pos < self.s.len() {
            match self.text[self.pos..].chars().next() {
                Some(ch) if ch == '$' || ch == '_' || unicode_ident::is_xid_continue(ch) => {
                    self.pos += ch.len_utf8();
                }
                // A key that keeps going through an escape cannot be a slice of the source.
                Some('\\') if self.allow_json5_numbers => {
                    return self.escaped_identifier_tail(&self.text[start..self.pos]);
                }
                _ => break,
            }
        }
        Ok(Cow::Borrowed(&self.text[start..self.pos]))
    }

    /// The general path for a key containing at least one escape. `prefix` is the literal run already
    /// validated, empty when the key begins with the backslash; `out.is_empty()` is therefore the
    /// "is this the first character" test. Rare by construction, so it decodes one character at a
    /// time and owns its text - the borrowed form above cannot express a decoded escape.
    fn escaped_identifier_tail(&mut self, prefix: &str) -> Result<Cow<'a, str>, ParseError> {
        let mut out = String::from(prefix);
        while let Some(ch) = self.ident_char(out.is_empty())? {
            out.push(ch);
        }
        if out.is_empty() {
            return Err(self.err("expected an identifier key"));
        }
        Ok(Cow::Owned(out))
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
        let inline_seen = self.inline_comments;
        // A note written between the colon and this `{` is consumed by the enclosing `ws()` and
        // reaches the first key through the pending slot, so the counter above never moves.
        let inherited_note = !self.pending_comment.is_empty();
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
            //
            // Equality alone is not the test, though: a key's same-line note is part of node
            // equality, so a repeated name with one on either key stays twice here. That is
            // collapsed when this map closes, and only if a note reached one of its keys - asking
            // about the note per key measured +0.8% on `from_json_medium`.
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
                        if inherited_note || self.inline_comments != inline_seen {
                            self.inline_comments = inline_seen;
                            collapse_noted_map(&mut pairs);
                        }
                        return Ok(CustomNode::plain_mapping(pairs));
                    }
                }
                Some(b'}') => {
                    self.pos += 1;
                    if inherited_note || self.inline_comments != inline_seen {
                        self.inline_comments = inline_seen;
                        collapse_noted_map(&mut pairs);
                    }
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
    ///
    /// A body with no escape is returned as a slice of the source: the AST holds an `Arc<str>`,
    /// and building a `String` first made every scalar allocate twice (once for the decode, once
    /// for the `Arc`) even when the source text was already the answer.
    ///
    /// What the body may hold unescaped is the dialect's question, not this scanner's. The scan
    /// stops on the whole C0 range because one compare per byte is the cheap test, and the arm
    /// below asks which of those bytes this dialect forbids: RFC 8259 §7 forbids every one of
    /// them, JSON5 forbids only a line terminator - so a raw tab, NUL, vertical tab or form feed
    /// is content there, which is what this file's own single-quoted string has always accepted,
    /// and what `pyjson5` answers for the same text in either spelling.
    fn string(&mut self) -> Result<Cow<'a, str>, ParseError> {
        debug_assert_eq!(self.peek(), Some(b'"'));
        self.pos += 1; // skip opening quote
        let content = self.pos;
        // `None` means the source has held no escape so far, so the body is still borrowable;
        // the first escape materialises the decoded buffer with the literal run before it.
        let mut out: Option<String> = None;
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
            // SAFETY: we only advanced over bytes that are not ASCII < 0x20,
            // not " or \ — so UTF-8 sequences are preserved intact.
            if self.pos > start
                && let Some(out) = &mut out
            {
                out.push_str(&self.text[start..self.pos]);
            }
            // Handle the stopping character.
            let Some(b) = self.peek() else {
                return Err(self.err("unterminated string"));
            };
            match b {
                b'"' => {
                    let end = self.pos;
                    self.pos += 1;
                    return Ok(match out {
                        Some(out) => Cow::Owned(out),
                        None => Cow::Borrowed(&self.text[content..end]),
                    });
                }
                b'\\' => {
                    let out = out.get_or_insert_with(|| {
                        let mut decoded = String::with_capacity(self.pos - content + 8);
                        decoded.push_str(&self.text[content..self.pos]);
                        decoded
                    });
                    self.pos += 1;
                    if self.allow_json5_numbers && self.line_continuation() {
                        // Strict JSON / JSONC fall through to the one-byte arms and reject a
                        // backslash before a newline, exactly as before this helper existed.
                        continue;
                    }
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
                        b'u' => self.unicode_escape(&mut *out)?,
                        // JSON5-only escapes (see the single-quoted twin).
                        b'v' if self.allow_json5_numbers => out.push('\u{b}'),
                        b'0' if self.allow_json5_numbers => out.push('\0'),
                        b'x' if self.allow_json5_numbers => self.x_escape(&mut *out)?,
                        b'\'' if self.allow_json5_numbers => out.push('\''),
                        _ => return Err(self.err("invalid escape sequence")),
                    }
                }
                0x00..=0x1f => {
                    // The scan stopped here on one compare, which is the cheap question; this arm
                    // asks the dialect's. JSON5 forbids only a line terminator, so every other
                    // control byte is content: take it and keep scanning, and when the body has
                    // held no escape yet, take it by leaving the source slice alone. Strict JSON
                    // and JSONC fall through to the refusal they have always given.
                    if self.allow_json5_numbers && b != b'\n' && b != b'\r' {
                        if let Some(out) = &mut out {
                            out.push(b as char);
                        }
                        self.pos += 1;
                        continue;
                    }
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

    /// JSON5's `LineContinuation`: a backslash immediately before a LineTerminator removes both.
    /// Returns `true` when it consumed one, so the caller continues the string from the next byte.
    ///
    /// The spec's LineTerminator set is LF, CR, CRLF, LS (U+2028) and PS (U+2029). A byte-level arm
    /// that reads only the terminator's first byte got two of these wrong: `\<CRLF>` swallowed the CR
    /// and left the LF as a raw newline inside a single-line string - which the scanners then reject,
    /// so the *single-quoted* path failed on input the *double-quoted* path accepted - and LS/PS were
    /// unreachable at the byte level at all. Matching the whole terminator is what closes both.
    fn line_continuation(&mut self) -> bool {
        match self.peek() {
            Some(b'\n') => {
                self.pos += 1;
                true
            }
            Some(b'\r') => {
                self.pos += 1;
                if self.peek() == Some(b'\n') {
                    self.pos += 1;
                }
                true
            }
            Some(0xE2)
                if self.s.get(self.pos + 1) == Some(&0x80)
                    && matches!(self.s.get(self.pos + 2), Some(&0xA8) | Some(&0xA9)) =>
            {
                self.pos += 3;
                true
            }
            _ => false,
        }
    }

    /// The JSON5 `\xHH` escape: exactly two hex digits, so its range is U+0000..=U+00FF.
    ///
    /// Accumulated digit by digit instead of `from_str_radix` over a slice, because that call accepts
    /// a leading sign - `u8::from_str_radix("+1", 16)` is `Ok(1)` - which would make `\x+1` a valid
    /// escape when the spec allows only hex digits. `hex4` cannot be reused: four digits can land on a
    /// surrogate, and two cannot.
    fn x_escape(&mut self, out: &mut String) -> Result<(), ParseError> {
        if self.pos + 2 > self.s.len() {
            return Err(self.err("truncated \\x escape"));
        }
        let mut value = 0u8;
        for _ in 0..2 {
            // `peek` cannot be None here, and every accepted byte is ASCII, so the `as char` lands
            // on the same code point the source spelled.
            let digit = self
                .peek()
                .filter(u8::is_ascii_hexdigit)
                .and_then(|b| (b as char).to_digit(16))
                .ok_or_else(|| self.err("invalid hex digits in \\x escape"))?;
            value = value * 16 + digit as u8;
            self.pos += 1;
        }
        // Two hex digits are at most 0xFF and all of U+0000..=U+00FF are Unicode scalars.
        out.push(value as char);
        Ok(())
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
fn single_quoted_node(value: Cow<'_, str>) -> CustomNode {
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
fn quoted_or_plain(value: Cow<'_, str>) -> CustomNode {
    if needs_quotes(&value)
        || (json5_could_be_typed(&value)
            && (json_number_text(&value)
                || !matches!(Schema::Json5.resolve(&value), YamlType::Str(_))))
    {
        CustomNode::double_quoted_scalar(value)
    } else {
        CustomNode::plain_scalar(value)
    }
}

/// Whether the text is a number in JSON's own grammar.
///
/// The two grammars disagree at the edges: the resolvers answer `Str` for an integer spelling
/// outside their parse - `"9999999999999999999"`, nineteen digits, is a string to them - while JSON
/// puts no bound on an integer literal. Projecting such a string as a plain scalar therefore let the
/// JSON writer hand the same text back as a *number*: `load_json` kept `str`, and one trip through
/// the hub changed what the document meant (libFuzzer `json_roundtrip` crash-09341b75). Asking the
/// JSON grammar is the sound form of the question "will a reader type this", and it is a byte walk
/// asked only of text the first-byte guard already let through - the resolver itself costs +14%
/// unasked, per the note above.
fn json_number_text(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut i = usize::from(bytes.first() == Some(&b'-'));
    // int = 0 / [1-9][0-9]* - JSON allows no leading zero and no `+`.
    match bytes.get(i) {
        Some(&b'0') => i += 1,
        Some(byte) if byte.is_ascii_digit() => {
            while matches!(bytes.get(i), Some(byte) if byte.is_ascii_digit()) {
                i += 1;
            }
        }
        _ => return false,
    }
    // frac = *( "." 1*DIGIT ) - a point has to be followed by a digit.
    if bytes.get(i) == Some(&b'.') {
        i += 1;
        let digits = i;
        while matches!(bytes.get(i), Some(byte) if byte.is_ascii_digit()) {
            i += 1;
        }
        if i == digits {
            return false;
        }
    }
    // exp = ( "e" / "E" ) [ "-" / "+" ] 1*DIGIT
    if matches!(bytes.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(bytes.get(i), Some(b'-' | b'+')) {
            i += 1;
        }
        let digits = i;
        while matches!(bytes.get(i), Some(byte) if byte.is_ascii_digit()) {
            i += 1;
        }
        if i == digits {
            return false;
        }
    }
    i == bytes.len()
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

/// Which node a group of notes belongs to, in the order a consumer builds nodes.
///
/// The direction is what a source position cannot say: a note written *before* a value can belong
/// to the value, and a note written *after* a value can belong to the value too. The producer
/// knows which node it filed the note under, so it answers in the consumer's build order.
#[derive(Clone, Copy)]
enum NoteSide {
    /// The node the consumer builds next.
    Following,
    /// The node the consumer built last.
    Preceding,
}

/// Which slot the notes of one group take.
#[derive(Clone, Copy, PartialEq, Eq)]
enum GroupSlot {
    /// The tree's own rule (issue #361): two same-line notes cannot fit the one inline slot, so
    /// every note in such a group leads; otherwise an own-line note leads and a same-line note
    /// takes the inline slot.
    ByRule,
    /// Every note leads. The notes above the document are the one place the tree says so
    /// outright, because the writer places them above the root rather than inside it.
    AllLeading,
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
    fn decoded(&mut self, pos: usize, value: Cow<'a, str>, style: ScalarStyle) {
        self.at(
            pos,
            StreamEventType::Scalar {
                value,
                style,
                anchor: None,
                tag: None,
            },
        );
    }

    /// Report one group of notes, in source order, each at its own start.
    ///
    /// The slot is the tree's own rule (issue #361): a group holding two same-line notes cannot
    /// fit them in the one inline slot, so every note in it goes to the leading list; otherwise an
    /// own-line note leads and a same-line note takes the inline slot.
    ///
    /// `side` says which node the group belongs to in the consumer's build order. The two facts
    /// are separate on purpose: `owns_line` is what the source did, `owner` is where the note
    /// lands, and only the producer knows the second one.
    fn group(&mut self, group: Vec<PendingComment>, side: NoteSide, slot: GroupSlot) {
        if group.is_empty() {
            return;
        }
        let all_leading = slot == GroupSlot::AllLeading
            || (group.len() > 1 && group.iter().filter(|note| !note.own_line).count() > 1);
        for note in group {
            let leading = all_leading || note.own_line;
            let owner = match (side, leading) {
                (NoteSide::Following, true) => NoteOwner::FollowingLeading,
                (NoteSide::Following, false) => NoteOwner::FollowingInline,
                (NoteSide::Preceding, true) => NoteOwner::PrecedingLeading,
                (NoteSide::Preceding, false) => NoteOwner::PrecedingInline,
            };
            self.at(
                note.at,
                StreamEventType::Comment {
                    text: alloc::sync::Arc::from(note.text.as_str()),
                    owns_line: note.own_line,
                    owner,
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
    p.ws();
    // The tree files the notes above the document on the root itself rather than on its first
    // member, so the stream reports them before the root's own events and every one of them
    // leads - the one group whose slot the tree states outright.
    let root_leading = core::mem::take(&mut p.pending_comment);
    out.group(root_leading, NoteSide::Following, GroupSlot::AllLeading);
    p.stream_value(&mut out)?;
    p.ws();
    let root_trailing = core::mem::take(&mut p.pending_comment);
    out.group(root_trailing, NoteSide::Preceding, GroupSlot::ByRule);
    if p.pos != p.s.len() {
        return Err(p.err("trailing characters after the JSON value"));
    }
    out.at(p.pos, StreamEventType::DocumentEnd);
    out.at(p.pos, StreamEventType::StreamEnd);
    Ok(out.events)
}

/// A stream that does not describe one JSON document, in the shape the scanner's own refusals use.
fn refuse(msg: &str) -> ParseError {
    ParseError::Syntax {
        message: format!("JSON event stream: {msg}"),
        line: 0,
        col: 0,
    }
}

/// Rebuild a document's tree from the events [`parse_stream`] reports for it.
///
/// The second reader of the stream, and the reason the vocabulary can be checked rather than
/// trusted: a mapping's name and value arrive as two scalars in document order, a `NodeMap` decides
/// duplicates exactly as the tree walk's does, and the three resolution rules - `quoted_or_plain`
/// for text, `hub_scalar` for a number or keyword, `single_quoted_node` for JSON5's own style - are
/// the same functions the scanner calls, not a second copy of them.
///
/// The notes come back too. A `Comment` event names the node it rides in the consumer's own build
/// order - the node the consumer builds next, or the one it built last - and the slot it takes
/// there, so the fold puts every note back where the tree filed it: the leading list or the one
/// inline slot, on the key or on the value, above the node or after it. That is the whole point of
/// the two-field shape; a single boolean read two ways could not say any of it.
///
/// What a JSON stream cannot carry is YAML's grammar: `anchor`, `tag` and `Alias` are refused
/// rather than silently dropped.
///
/// An empty stream, a second root value, a container closed with nothing open or with the wrong
/// kind open, and a block scalar style JSON never writes are all refusals too: this is a public
/// reader of a caller-supplied list, so it validates instead of trusting.
pub fn from_events(events: &[StreamEvent<'_>]) -> Result<CustomNode, ParseError> {
    /// The notes one node owns, in the order the source wrote them.
    #[derive(Default)]
    struct Owned {
        leading: Vec<pyrs_ast::ast::Comment>,
        inline: Option<pyrs_ast::ast::Comment>,
    }

    /// The member a mapping is in the middle of: the key, and the value once it has arrived.
    ///
    /// One slot rather than two, because a key and a complete member are never both pending:
    /// the value that completes a member is the one that ends the key's wait.
    enum Pending {
        /// The key has arrived; its value has not.
        Key(CustomNode),
        /// The member is complete but not filed: a note may still ride its value.
        Member(CustomNode, CustomNode),
    }

    /// One open container, with the state the tree walk keeps while it builds the same container.
    enum Frame {
        Map {
            pairs: NodeMap<CustomNode, CustomNode>,
            /// The member the mapping is in the middle of.
            pending: Option<Pending>,
            /// The notes the mapping itself owns.
            own: Owned,
            /// Whether a note was seen inside this mapping.
            noted: bool,
        },
        Seq {
            items: Vec<CustomNode>,
            /// The item that is complete but not filed yet: a note may still ride it.
            item: Option<CustomNode>,
            /// The notes the sequence itself owns.
            own: Owned,
            /// Whether a note was seen inside this sequence.
            noted: bool,
        },
    }

    /// Put a note where the stream says it rides: the leading list, or the one inline slot.
    ///
    /// The tree's own `standalone` flag says the same thing - a note in the leading list carries
    /// `true`, the note in the inline slot carries `false` - so a folded tree and a parsed one
    /// compare equal on their decorations and not only on their structure.
    fn apply(node: &mut CustomNode, text: alloc::sync::Arc<str>, leading: bool) {
        let comment = pyrs_ast::ast::Comment {
            text,
            standalone: leading,
        };
        if leading {
            node.push_leading_comment(comment);
        } else {
            node.set_comment(comment);
        }
    }

    /// The notes a container owns, split into the leading list and the inline slot.
    fn take_owned(following: &mut Vec<(alloc::sync::Arc<str>, bool)>) -> Owned {
        let mut own = Owned::default();
        for (text, leading) in core::mem::take(following) {
            let comment = pyrs_ast::ast::Comment {
                text,
                standalone: leading,
            };
            if leading {
                own.leading.push(comment);
            } else {
                own.inline = Some(comment);
            }
        }
        own
    }

    /// The notes a container owns, put on the container itself.
    fn apply_own(node: &mut CustomNode, own: Owned) {
        for comment in own.leading {
            node.push_leading_comment(comment);
        }
        if let Some(comment) = own.inline {
            node.set_comment(comment);
        }
    }

    /// File a map member the way the tree walk files it: the last value wins, the first position
    /// is kept, and the notes the loser carried move to the survivor.
    fn file_member(pairs: &mut NodeMap<CustomNode, CustomNode>, member: (CustomNode, CustomNode)) {
        let (key, value) = member;
        // The key's own notes are part of its identity, so they have to be read before the key
        // moves into the map - a name repeated while a note kept the two apart stays twice.
        let carried = notes_of(&key);
        let (index, replaced) = pairs.insert_full(key, value);
        if let Some(displaced) = replaced {
            park_notes(pairs, index, displaced, carried);
        }
    }

    /// Put a completed node where it belongs: the member of a mapping, the item of a sequence, or
    /// the document's root.
    fn place(
        stack: &mut [Frame],
        root: &mut Option<CustomNode>,
        node: CustomNode,
    ) -> Result<(), ParseError> {
        match stack.last_mut() {
            Some(Frame::Map { pending, .. }) => {
                // The value completes the member the key started.
                let Some(Pending::Key(key)) = pending.take() else {
                    return Err(refuse("a mapping's value arrived without a key"));
                };
                *pending = Some(Pending::Member(key, node));
            }
            Some(Frame::Seq { item, .. }) => *item = Some(node),
            None => {
                if root.is_some() {
                    return Err(refuse("a second root value in one stream"));
                }
                *root = Some(node);
            }
        }
        Ok(())
    }

    /// Record that a note was seen inside the container that is open now.
    ///
    /// The tree folds a repeated name when a note reached one of its keys, and the fold needs the
    /// same answer: a note inside the container is the signal, and a container with no note in it
    /// has nothing to fold.
    fn mark_noted(stack: &mut [Frame]) {
        match stack.last_mut() {
            Some(Frame::Map { noted, .. }) => *noted = true,
            Some(Frame::Seq { noted, .. }) => *noted = true,
            None => {}
        }
    }

    let mut stack: Vec<Frame> = Vec::new();
    let mut root: Option<CustomNode> = None;
    // The notes that belong to the node the consumer builds next.
    let mut following: Vec<(alloc::sync::Arc<str>, bool)> = Vec::new();

    for event in events {
        match &event.event_type {
            StreamEventType::MappingStart { anchor, tag } => {
                if anchor.is_some() || tag.is_some() {
                    return Err(refuse(
                        "an anchored or tagged mapping is not JSON's to report",
                    ));
                }
                match stack.last_mut() {
                    Some(Frame::Map { pending, .. }) => {
                        if !matches!(pending, Some(Pending::Key(_))) {
                            return Err(refuse("a mapping cannot stand in a key's place"));
                        }
                    }
                    Some(Frame::Seq { items, item, .. }) => {
                        if let Some(done) = item.take() {
                            items.push(done);
                        }
                    }
                    None => {}
                }
                // The notes that came before this container are its own.
                stack.push(Frame::Map {
                    pairs: NodeMap::default(),
                    pending: None,
                    own: take_owned(&mut following),
                    noted: false,
                });
            }
            StreamEventType::SequenceStart { anchor, tag } => {
                if anchor.is_some() || tag.is_some() {
                    return Err(refuse(
                        "an anchored or tagged sequence is not JSON's to report",
                    ));
                }
                match stack.last_mut() {
                    Some(Frame::Map { pending, .. }) => {
                        if !matches!(pending, Some(Pending::Key(_))) {
                            return Err(refuse("a sequence cannot stand in a key's place"));
                        }
                    }
                    Some(Frame::Seq { items, item, .. }) => {
                        if let Some(done) = item.take() {
                            items.push(done);
                        }
                    }
                    None => {}
                }
                stack.push(Frame::Seq {
                    items: Vec::new(),
                    item: None,
                    own: take_owned(&mut following),
                    noted: false,
                });
            }
            StreamEventType::MappingEnd => {
                let node = match stack.pop() {
                    Some(Frame::Map {
                        mut pairs,
                        pending,
                        own,
                        noted,
                    }) => {
                        match pending {
                            Some(Pending::Key(_)) => {
                                return Err(refuse("a mapping ended on a key with no value"));
                            }
                            Some(Pending::Member(key, value)) => {
                                file_member(&mut pairs, (key, value));
                            }
                            None => {}
                        }
                        if noted {
                            collapse_noted_map(&mut pairs);
                        }
                        let mut node = CustomNode::plain_mapping(pairs);
                        apply_own(&mut node, own);
                        node
                    }
                    Some(Frame::Seq { .. }) => {
                        return Err(refuse(
                            "a container closed against the wrong kind of open frame",
                        ));
                    }
                    None => return Err(refuse("a container closed with nothing open")),
                };
                place(&mut stack, &mut root, node)?;
            }
            StreamEventType::SequenceEnd => {
                let node = match stack.pop() {
                    Some(Frame::Seq {
                        mut items,
                        item,
                        own,
                        ..
                    }) => {
                        if let Some(done) = item {
                            items.push(done);
                        }
                        let mut node = CustomNode::plain_sequence(items);
                        apply_own(&mut node, own);
                        node
                    }
                    Some(Frame::Map { .. }) => {
                        return Err(refuse(
                            "a container closed against the wrong kind of open frame",
                        ));
                    }
                    None => return Err(refuse("a container closed with nothing open")),
                };
                place(&mut stack, &mut root, node)?;
            }
            StreamEventType::Scalar {
                value,
                style,
                anchor,
                tag,
            } => {
                if anchor.is_some() || tag.is_some() {
                    return Err(refuse(
                        "an anchored or tagged scalar is not JSON's to report",
                    ));
                }
                // A mapping's name is the hub's plain-or-quoted text whatever spelling it arrived
                // in - the tree's `object` calls `quoted_or_plain` on every key - while a value
                // keeps the style it was written in.
                let as_name = matches!(
                    stack.last(),
                    Some(Frame::Map { pending, .. }) if !matches!(pending, Some(Pending::Key(_)))
                );
                let mut node = if as_name {
                    quoted_or_plain(Cow::Borrowed(value.as_ref()))
                } else {
                    match style {
                        ScalarStyle::SingleQuoted => {
                            single_quoted_node(Cow::Borrowed(value.as_ref()))
                        }
                        ScalarStyle::DoubleQuoted => quoted_or_plain(Cow::Borrowed(value.as_ref())),
                        // The tree stores the word `null` as a null node, and every other plain
                        // scalar as the hub's own spelling of the token.
                        ScalarStyle::Plain => {
                            if *value == "null" {
                                CustomNode::plain_null()
                            } else {
                                hub_scalar(value)
                            }
                        }
                        other => {
                            return Err(refuse(&format!(
                                "JSON never writes a scalar in {other:?}"
                            )));
                        }
                    }
                };
                // The notes that came before this node ride it.
                for (text, leading) in core::mem::take(&mut following) {
                    apply(&mut node, text, leading);
                }
                match stack.last_mut() {
                    Some(Frame::Map { pairs, pending, .. }) => {
                        match pending.take() {
                            // This scalar is the first key of the mapping.
                            None => {
                                *pending = Some(Pending::Key(node));
                            }
                            // This scalar is the value of the key that came before it.
                            Some(Pending::Key(key)) => {
                                *pending = Some(Pending::Member(key, node));
                            }
                            // The member before this one is complete, so it is filed now, and
                            // this scalar is the next key.
                            Some(Pending::Member(key, value)) => {
                                file_member(pairs, (key, value));
                                *pending = Some(Pending::Key(node));
                            }
                        }
                    }
                    Some(Frame::Seq { items, item, .. }) => {
                        if let Some(done) = item.take() {
                            items.push(done);
                        }
                        *item = Some(node);
                    }
                    None => {
                        if root.is_some() {
                            return Err(refuse("a second root value in one stream"));
                        }
                        root = Some(node);
                    }
                }
            }
            StreamEventType::Alias { name } => {
                return Err(refuse(&format!("JSON has no alias, and {name} was one")));
            }
            StreamEventType::Comment { text, owner, .. } => {
                mark_noted(&mut stack);
                match owner {
                    NoteOwner::FollowingLeading | NoteOwner::FollowingInline => {
                        // The note belongs to the node the consumer builds next.
                        following.push((text.clone(), owner.is_leading()));
                    }
                    NoteOwner::PrecedingLeading | NoteOwner::PrecedingInline => {
                        // The note belongs to the node the consumer built last.
                        let target = match stack.last_mut() {
                            Some(Frame::Map { pending, .. }) => match pending {
                                Some(Pending::Member(_, value)) => Some(value),
                                Some(Pending::Key(_)) | None => None,
                            },
                            Some(Frame::Seq { item, .. }) => item.as_mut(),
                            None => root.as_mut(),
                        };
                        let Some(node) = target else {
                            return Err(refuse("a note arrived before the node it rides"));
                        };
                        apply(node, text.clone(), owner.is_leading());
                    }
                }
            }
            StreamEventType::StreamStart
            | StreamEventType::StreamEnd
            | StreamEventType::DocumentStart
            | StreamEventType::DocumentEnd => {}
        }
    }
    if !stack.is_empty() {
        return Err(refuse("the stream ended with a container still open"));
    }
    root.ok_or_else(|| refuse("a stream with no document in it"))
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

    /// The dispatch of `object`, with every note group reported where the tree files it.
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
        // The notes after `{` stay pending, as they do in the tree: the first key claims them, and
        // an empty mapping leaves them to the member that holds it.
        self.ws();
        if self.peek() == Some(b'}') {
            let close = self.pos;
            self.pos += 1;
            out.at(close, StreamEventType::MappingEnd);
            return Ok(());
        }
        loop {
            self.ws();
            // The tree takes the notes here as the key's own group, so the stream does too - and
            // reports them before the key's own event, because the key is the node they ride.
            let key_group = core::mem::take(&mut self.pending_comment);
            let token = self.pos;
            let (key, style) = if self.peek() == Some(b'"') {
                (self.string()?, ScalarStyle::DoubleQuoted)
            } else if self.allow_single_quoted && self.peek() == Some(b'\'') {
                (self.single_quoted_string()?, ScalarStyle::SingleQuoted)
            } else if self.allow_unquoted_keys && self.at_identifier_start() {
                (self.identifier_key()?, ScalarStyle::Plain)
            } else {
                return Err(self.err("expected a quoted object key"));
            };
            out.group(key_group, NoteSide::Following, GroupSlot::ByRule);
            out.decoded(token, key, style);
            self.ws();
            self.expect(":", "expected `:` after the object key")?;
            self.ws();
            self.stream_value(out)?;
            // The notes around the value are one group in the tree, and the tree files them on the
            // value - so the stream reports them after the value's own events. A note between the
            // key and the colon lands here too, because the value's own container takes it when
            // the value is a container; the tree files it on the value's first member then, and
            // the stream reports it inside that container.
            self.ws();
            let value_group = core::mem::take(&mut self.pending_comment);
            out.group(value_group, NoteSide::Preceding, GroupSlot::ByRule);
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                    self.ws();
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
        // The notes after `[` stay pending, as they do in the tree: the first element claims them.
        self.ws();
        if self.peek() == Some(b']') {
            let close = self.pos;
            self.pos += 1;
            out.at(close, StreamEventType::SequenceEnd);
            return Ok(());
        }
        loop {
            self.ws();
            // The notes above the element are the element's own leading group in the tree, so the
            // stream reports them before the element's own event.
            let element_group = core::mem::take(&mut self.pending_comment);
            out.group(element_group, NoteSide::Following, GroupSlot::ByRule);
            self.stream_value(out)?;
            // The notes after the element are a second group, and the tree files them on the same
            // node - so the stream reports them after the element's own events.
            self.ws();
            let after_group = core::mem::take(&mut self.pending_comment);
            out.group(after_group, NoteSide::Preceding, GroupSlot::ByRule);
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                    self.ws();
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
    fn a_duplicate_name_collapses_even_when_a_key_note_made_the_nodes_unequal() {
        // Found by `fuzz/fuzz_targets/json_events_replay.rs` and `json_roundtrip.rs`: a same-line
        // note on one key made the repeated name's nodes unequal, so both pairs survived - and the
        // strict writer then emitted the name twice, which a re-read folds into one. The output was
        // not a fixed point and the replay tree disagreed with the walk's.
        for text in [
            // the note introduces the second `a` (the artifact's shape)
            "{\"a\": 0, // on the key\n  \"a\": 1,\n  \"b\": 2}",
            // the note introduces the first `a`, and the repeat is clean
            "{/* on the key */ \"a\": 0, \"a\": 1, \"b\": 2}",
        ] {
            let tree = from_jsonc(text).expect("valid JSONC");
            let CustomNode::Mapping { pairs, .. } = &tree else {
                unreachable!()
            };
            assert_eq!(pairs.len(), 2, "one `a` and one `b` for {text:?}");
            let (key, value) = pairs.get_index(0).expect("first pair");
            assert_eq!(scalar_text(key), "a");
            assert_eq!(scalar_text(value), "1", "last value wins for {text:?}");
            let written = to_jsonc_text(&tree).expect("writable");
            assert!(written.contains("on the key"), "{written}");
            let strict = crate::to_json_text(&tree).expect("writable");
            assert_eq!(strict, r#"{"a":1,"b":2}"#, "{text:?}");
            assert_eq!(
                crate::to_json_text(&from_json(&strict).expect("its own output re-parses"))
                    .expect("writable"),
                strict
            );
        }
    }

    #[test]
    fn a_note_written_before_a_nested_map_still_collapses_its_repeat() {
        // The note sits between the colon and the `{`, so the outer `ws()` consumes it and this
        // map's first key inherits it through the pending slot instead of its own `ws()`. The
        // collapse trigger has to fire for that route too.
        let text = "{\"outer\": /* on the key */ {\"a\": 0, \"a\": 1, \"b\": 2}}";
        let tree = from_jsonc(text).expect("valid JSONC");
        let CustomNode::Mapping { pairs, .. } = &tree else {
            unreachable!()
        };
        let CustomNode::Mapping { pairs: inner, .. } = pairs.get_index(0).expect("outer pair").1
        else {
            unreachable!()
        };
        assert_eq!(inner.len(), 2, "one `a` and one `b`");
        assert_eq!(scalar_text(inner.get_index(0).expect("first pair").1), "1");
        assert!(
            to_jsonc_text(&tree)
                .expect("writable")
                .contains("on the key")
        );
        assert_eq!(
            crate::to_json_text(&tree).expect("writable"),
            r#"{"outer":{"a":1,"b":2}}"#
        );
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

    // The two spellings of a JSON5 string disagreed about a raw control byte: the single-quoted
    // scanner copied through everything except CR and LF, while the double-quoted one stopped on
    // every byte below 0x20 and refused it. JSON5's rule is the single-quoted one, and `pyjson5`
    // answers the same way for either spelling, so this table asks all 32 C0 bytes in both quotes
    // and names the two that stay refused. Strict JSON and JSONC keep RFC 8259 §7 for the same
    // byte in the same place.
    #[test]
    fn a_json5_quoted_string_keeps_every_c0_byte_that_is_not_a_line_break() {
        for cp in 0x00..0x20_u8 {
            let raw = cp as char;
            let is_break = cp == b'\n' || cp == b'\r';
            for quote in ['"', '\''] {
                let text = format!("{quote}a{raw}b{quote}");
                match from_json5(&text) {
                    Ok(CustomNode::Scalar { value, .. }) => {
                        assert!(!is_break, "{text:?} is not one line");
                        assert_eq!(&*value, format!("a{raw}b"), "{text:?}");
                    }
                    Ok(_) => unreachable!(),
                    Err(_) => assert!(is_break, "{text:?} is content in JSON5"),
                }
            }
            let strict = format!("\"a{raw}b\"");
            assert!(from_json(&strict).is_err(), "{strict:?} must stay refused");
            assert!(from_jsonc(&strict).is_err(), "{strict:?} must stay refused");
        }

        // What the JSON5 reader takes in it can write back: the JSON5 writer escapes the control
        // byte, so the document that comes out is one line and holds the same text.
        let node = from_json5("\"a\tb\"").expect("a raw tab is content in JSON5");
        let written = crate::to_json5_text(&node).expect("writable");
        let again = from_json5(&written).expect("its own output re-parses");
        assert_eq!(scalar_text(&again), "a\tb");
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
    fn json5_line_continuation_covers_every_line_terminator_in_both_quote_styles() {
        // The spec's LineTerminator set is LF, CR, CRLF, LS and PS, and a continuation removes the
        // backslash plus the whole terminator. Reading only the terminator's first byte left the
        // single-quoted path rejecting `\<CRLF>` that the double-quoted path accepted - the two
        // scanners of one dialect disagreeing is the defect this closes.
        const TERMINATORS: [(&str, &str); 5] = [
            ("LF", "\n"),
            ("CR", "\r"),
            ("CRLF", "\r\n"),
            ("LS", "\u{2028}"),
            ("PS", "\u{2029}"),
        ];
        for (name, terminator) in TERMINATORS {
            let single = format!("'x\\{terminator}y'");
            let double = format!("\"x\\{terminator}y\"");
            assert_eq!(
                scalar_text(&from_json5(&single).unwrap()),
                "xy",
                "{name} continuation, single-quoted"
            );
            assert_eq!(
                scalar_text(&from_json5(&double).unwrap()),
                "xy",
                "{name} continuation, double-quoted"
            );
        }
        // Strict JSON has no line continuation: the backslash-terminator pair stays a rejection, and
        // the guard that removed the byte-level arm must not have widened it.
        assert!(from_json("\"x\\\ny\"").is_err());
        assert!(from_jsonc("\"x\\\r\ny\"").is_err());
        // A terminator still ends the document's string when it is not escaped.
        assert!(from_json5("'x\ny'").is_err());
    }

    /// A JSON string spelled like an integer past the resolvers' parse keeps its quotes.
    ///
    /// Core answers `Str` for a digit run too long to fit an `i64`, and JSON puts no bound on an
    /// integer literal, so a projection that wrote it plain turned a `str` into a number on the way
    /// back out (libFuzzer `json_roundtrip` crash-09341b75).
    #[test]
    fn a_string_too_long_for_the_resolver_still_leaves_as_a_string() {
        let big = "\"9999999999999999999\"";
        let node = from_json(big).expect("a quoted digit run is a JSON string");
        let CustomNode::Scalar { style, .. } = &node else {
            unreachable!()
        };
        assert_eq!(*style, ScalarStyle::DoubleQuoted, "quoted in, quoted out");
        assert_eq!(
            crate::to_json_text(&node).expect("a string root writes"),
            big,
            "and it leaves as the string it was"
        );
        // The predicate claims JSON's number grammar, so it is asked of that grammar's edges.
        for (text, is_number) in [
            ("0", true),
            ("00", false),
            ("-1", true),
            ("+1", false),
            ("1e5", true),
            ("1.5e-3", true),
            ("1.", false),
            ("9999999999999999999", true),
            ("1_0", false),
        ] {
            assert_eq!(json_number_text(text), is_number, "{text}");
        }
    }

    #[test]
    fn json5_does_not_hand_the_reader_a_string_it_would_trim_away() {
        // The JSON5 projection resolves a plain scalar through `resolve_json_type`, which trims
        // with Unicode rules, so a string whose text starts or ends with a non-ASCII space went
        // out plain and came back as something else - `load_json5` answered `{None: 1}` for a
        // quoted U+2028 key. The character is built from its code point, and the escape from two
        // pieces, so no tool rewrites the sequence under test.
        let space = char::from_u32(0x2028).unwrap();
        let esc = concat!("\\", "u");
        let doc = alloc::format!("{{ a: \"{esc}2028{esc}0041\" }}");
        let CustomNode::Mapping { pairs, .. } = &from_json5(&doc).unwrap() else {
            unreachable!()
        };
        let (_, value) = pairs.iter().next().unwrap();
        let CustomNode::Scalar {
            value: text, style, ..
        } = value
        else {
            unreachable!()
        };
        assert_eq!(*style, ScalarStyle::DoubleQuoted, "quoted in, quoted out");
        let chars: alloc::vec::Vec<char> = text.chars().collect();
        assert_eq!(chars, alloc::vec![space, 'A']);

        // The value that is nothing but such a character is the shape that became null.
        let only = alloc::format!("{{ a: \"{esc}2028\" }}");
        let CustomNode::Mapping { pairs, .. } = &from_json5(&only).unwrap() else {
            unreachable!()
        };
        let (_, value) = pairs.iter().next().unwrap();
        let CustomNode::Scalar {
            value: text, style, ..
        } = value
        else {
            unreachable!()
        };
        assert!(
            !matches!(style, ScalarStyle::Plain),
            "{text:?} went out plain"
        );
        assert_eq!(text.chars().count(), 1);
        assert_eq!(text.chars().next().unwrap(), space);
    }

    #[test]
    fn json5_accepts_escaped_characters_in_an_unquoted_key() {
        // JSON5's Identifier is ES5 `IdentifierName`, which allows a UnicodeEscapeSequence in any
        // identifier position and does not re-validate what it decodes to. The reference agrees:
        // the escaped forms of a digit, a dollar sign and a dot are all legal keys, even though the
        // literal characters could not start an identifier.
        //
        // The escape is spelled in two pieces because `ESC` plus four digits is the very sequence
        // under test, and a source file that writes it directly is hard to tell apart from the
        // document the reader must accept.
        const ESC: &str = concat!("\\", "u");
        let keys = |node: &CustomNode| -> Vec<String> {
            match node {
                CustomNode::Mapping { pairs, .. } => pairs.keys().map(scalar_text).collect(),
                other => panic!("expected a mapping, got {other:?}"),
            }
        };
        let escaped_key = |digits: &str| format!("{{{ESC}{digits}: 1}}");

        assert_eq!(keys(&from_json5(&escaped_key("0061")).unwrap()), vec!["a"]);
        assert_eq!(keys(&from_json5(&escaped_key("0031")).unwrap()), vec!["1"]);
        assert_eq!(keys(&from_json5(&escaped_key("0024")).unwrap()), vec!["$"]);
        assert_eq!(keys(&from_json5(&escaped_key("002E")).unwrap()), vec!["."]);
        // An escape mid-name, and a name that mixes literal and escaped characters.
        assert_eq!(
            keys(&from_json5(&format!("{{a{ESC}0062: 1}}")).unwrap()),
            vec!["ab"]
        );
        assert_eq!(
            keys(&from_json5(&format!("{{{ESC}0041b: 1}}")).unwrap()),
            vec!["Ab"]
        );
        // Not a Unicode scalar, fewer than four digits, and escapes that are not identifier escapes.
        assert!(from_json5(&escaped_key("D800")).is_err());
        assert!(from_json5(&escaped_key("00")).is_err());
        assert!(from_json5(&format!("{{{ESC}{{41}}: 1}}")).is_err());
        assert!(from_json5(r"{\x41: 1}").is_err());
        // The narrower dialects have no unquoted keys, so an escaped key is not a way in.
        assert!(from_jsonc(&escaped_key("0061")).is_err());
        assert!(from_json(&escaped_key("0061")).is_err());
        // Whatever the writer makes of a decoded key, the reader reads it back as the same map, and
        // the second emission does not move - the fixed point the round-trip tier insists on.
        let once = crate::to_json5_text(&from_json5(&escaped_key("002E")).unwrap()).unwrap();
        assert_eq!(keys(&from_json5(&once).unwrap()), vec!["."]);
        assert_eq!(
            crate::to_json5_text(&from_json5(&once).unwrap()).unwrap(),
            once
        );
    }

    #[test]
    fn json5_supports_the_two_digit_hex_escape() {
        // The spec's EscapeSequence list includes `\x` followed by exactly two hex digits, so a
        // decoded value never leaves U+0000..=U+00FF. Found by reading the spec against the
        // reference: pyjson5 gives `{'a': 'A'}` for `{a: '\x41'}` while this reader reported
        // "invalid escape sequence" - although `\v`, `\0` and the line continuation were here.
        assert_eq!(scalar_text(&from_json5(r#""A\x7eZ""#).unwrap()), "A~Z");
        assert_eq!(scalar_text(&from_json5(r#"'caf\xE9'"#).unwrap()), "café");
        // Exactly two digits: a third hex-looking character is ordinary text, not part of it.
        assert_eq!(scalar_text(&from_json5(r#""\x410""#).unwrap()), "A0");
        // Control characters arrive as characters, not as the line break they spell.
        assert_eq!(scalar_text(&from_json5(r#""a\x0Ab""#).unwrap()), "a\nb");
        assert_eq!(scalar_text(&from_json5(r#""a\x00b""#).unwrap()), "a\0b");
        // The digits are accumulated one byte at a time because `from_str_radix` accepts a leading
        // sign, which would make `\x+1` a valid escape; and the tail cases are typed rejections.
        assert!(from_json5(r#""\x+1""#).is_err());
        assert!(from_json5(r#""\x4""#).is_err());
        assert!(from_json5(r#""\xZZ""#).is_err());
        assert!(from_json5("\"\\x").is_err());
        // A malformed escape straddling a multibyte character must not reach a char-boundary
        // slice - the sibling of the #153 crashes that `hex4` documents.
        assert!(from_json5("\"\\xé").is_err());
        // Neither strict JSON nor JSONC unlocks it.
        assert!(from_json(r#""\x41""#).is_err());
        assert!(from_jsonc(r#""\x41""#).is_err());
        // The decoded value writes back as JSON5 that this reader accepts, and it settles.
        let once = crate::to_json5_text(&from_json5(r#""A\x7eZ""#).unwrap()).unwrap();
        assert_eq!(scalar_text(&from_json5(&once).unwrap()), "A~Z");
        assert_eq!(
            crate::to_json5_text(&from_json5(&once).unwrap()).unwrap(),
            once
        );
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
            inline_comments: 0,
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
            let guarded = quoted_or_plain(Cow::Borrowed(text));
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
            StreamEventType::Comment {
                text,
                owns_line,
                owner,
            } => {
                format!("comment[owns_line={owns_line},owner={owner:?}]={text}")
            }
        }
    }

    /// Replay through the public reader. A stream this crate produced is well-formed by
    /// construction, so a refusal here is the defect rather than a case to tolerate; the reader's
    /// own refusals are tested by `a_malformed_stream_is_refused_rather_than_guessed`.
    fn fold(events: &[StreamEvent<'_>]) -> CustomNode {
        from_events(events).expect("a stream this crate wrote replays")
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
    ///
    /// The tree's flag is the placement: `true` means the note rides the node's leading list, and
    /// the node's inline slot is the one note that does not.
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

    /// Every note a stream reports, as text, the source fact and the owner.
    fn stream_notes(events: &[StreamEvent<'_>]) -> Vec<(String, bool, NoteOwner)> {
        events
            .iter()
            .filter_map(|event| match &event.event_type {
                StreamEventType::Comment {
                    text,
                    owns_line,
                    owner,
                } => Some((text.to_string(), *owns_line, *owner)),
                _ => None,
            })
            .collect()
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
    fn a_number_and_an_unescaped_string_borrow_while_a_decoded_string_owns_its_text() {
        let text = r#"{"n": 12345, "s": "a\u0062", "t": true, "b": "plain"}"#;
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
        let StreamEventType::Scalar {
            value,
            style: ScalarStyle::DoubleQuoted,
            ..
        } = value_of("b")
        else {
            panic!("\"plain\" must arrive as a double-quoted scalar");
        };
        let Cow::Borrowed(plain) = value else {
            panic!("a string with no escape is the source's own bytes");
        };
        assert_eq!(plain, "plain");
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
        assert!(
            shapes.contains(&"comment[owns_line=false,owner=FollowingInline]=note".to_string())
        );
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
        // keeps the reading the bytes gave. The owner says the other half - which node the note
        // rides and in which slot - so the two facts travel together instead of over each other.
        let text = "{\n  // own line\n  \"a\": 1, /* same line */ \"b\": 2\n}";
        let stream = parse_stream_with_options(text, JsonParseOptions::JSONC).expect("valid JSONC");
        let notes = stream_notes(&stream);
        assert_eq!(
            notes,
            [
                ("own line".to_string(), true, NoteOwner::FollowingLeading),
                ("same line".to_string(), false, NoteOwner::FollowingInline)
            ]
        );
    }

    #[test]
    fn a_same_line_note_around_a_comma_names_the_side_it_sat_on() {
        // The comma that ends a pair is not an event, so a note's own position cannot say which
        // node it belongs to: one note before the comma is the value's, the same note after it is
        // the next key's. The producer files both by its own rule and now says so - the value's
        // note is `Preceding`, the key's is `Following` - so the two documents no longer look
        // alike, and a consumer can put each note back where the tree had it.
        let before_comma = r#"{"a": 1 /* x */, "b": 2}"#;
        let after_comma = r#"{"a": 1, /* x */ "b": 2}"#;
        let notes_of = |text: &str| -> Vec<(String, bool, NoteOwner)> {
            stream_notes(
                &parse_stream_with_options(text, JsonParseOptions::JSONC).expect("valid JSONC"),
            )
        };
        assert_eq!(
            notes_of(before_comma),
            vec![("x".to_string(), false, NoteOwner::PrecedingInline)]
        );
        assert_eq!(
            notes_of(after_comma),
            vec![("x".to_string(), false, NoteOwner::FollowingInline)]
        );

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

        // The owner is the tree's own answer, so the fold puts the note back where the tree had
        // it - the whole document, decorations included, not only its structure.
        for text in [before_comma, after_comma] {
            let tree = from_json_with_options(text, JsonParseOptions::JSONC).expect("valid JSONC");
            let events =
                parse_stream_with_options(text, JsonParseOptions::JSONC).expect("valid JSONC");
            assert_eq!(fold(&events), tree, "the fold lost the note in {text}");
        }
    }

    #[test]
    fn the_two_walks_answer_the_same_two_questions() {
        // Two same-line notes introduce one value. The single inline slot cannot hold both, so
        // #361 moves them to the node's leading list - and a leading-list note carries
        // `standalone: true`, so in the tree the flag means "this note rides the leading list".
        // The stream reported the same boolean as "the source gave this note a line of its own",
        // which the source did not: one boolean, two questions.
        //
        // The split answers them separately. `owner` is the tree's own placement - which node and
        // which slot - and `owns_line` is the source fact, so the two walks now answer the same
        // two questions and a consumer can rebuild the decorations.
        let text = r#"{"a" /* on the key */: /* on the value */ 1}"#;
        let tree = from_json_with_options(text, JsonParseOptions::JSONC).expect("valid JSONC");
        let mut kept = Vec::new();
        tree_notes(&tree, &mut kept);
        kept.sort();
        assert_eq!(
            kept,
            vec![
                ("on the key".to_string(), true),
                ("on the value".to_string(), true)
            ]
        );
        let mut reported = stream_notes(
            &parse_stream_with_options(text, JsonParseOptions::JSONC).expect("valid JSONC"),
        );
        reported.sort();
        assert_eq!(
            reported,
            vec![
                ("on the key".to_string(), false, NoteOwner::PrecedingLeading),
                (
                    "on the value".to_string(),
                    false,
                    NoteOwner::PrecedingLeading
                )
            ]
        );
    }

    #[test]
    fn every_note_the_tree_files_is_reported_with_the_same_owner() {
        // The per-dialect invariant of decision 6: for every note the tree files on a node, the
        // stream reports the same `owns_line` and the same `owner`. The tree's slot is its
        // `standalone` flag - the leading list against the one inline slot - and the stream's is
        // the owner's, so the two must agree note for note.
        let cases: &[(JsonParseOptions, &str)] = &[
            (JsonParseOptions::JSONC, "// above\n{}"),
            (JsonParseOptions::JSONC, "{} // after"),
            (JsonParseOptions::JSONC, "{\"a\": /*b*/ 1}"),
            (JsonParseOptions::JSONC, "[1 /* trailing */, 2]"),
            (
                JsonParseOptions::JSONC,
                "{\"a\": [\n  // line\n  1,\n  /* block */ 2\n] /* tail */}",
            ),
            (JsonParseOptions::JSONC, "[/* one */ /* two */ 1]"),
            (JsonParseOptions::JSONC, "{\"a\": 1 /* x */, \"b\": 2}"),
            (JsonParseOptions::JSONC, "{\"a\": 1, /* x */ \"b\": 2}"),
            (JsonParseOptions::JSONC, "{\"a\": /* c */ {\"k\": 1}}"),
            (JsonParseOptions::JSONC, "{\"a\": /* c */ [1]}"),
            (JsonParseOptions::JSONC, "{\"a\": /* c */ {}}"),
            (
                JsonParseOptions::JSON5,
                "{/*k*/ a: /*v*/ 'b' /*t*/, c: 1 // end\n}",
            ),
            (JsonParseOptions::JSON5, "[1, /* between */ 2]"),
        ];
        for (opts, text) in cases {
            let tree = from_json_with_options(text, *opts)
                .unwrap_or_else(|e| panic!("{text} must parse: {e}"));
            let events = parse_stream_with_options(text, *opts)
                .unwrap_or_else(|e| panic!("{text} must stream: {e}"));
            let mut from_tree = Vec::new();
            tree_notes(&tree, &mut from_tree);
            let from_stream = stream_notes(&events);
            assert_eq!(
                from_tree.len(),
                from_stream.len(),
                "{text}: the tree keeps {} notes and the stream reports {}",
                from_tree.len(),
                from_stream.len()
            );
            // The slot has to agree note for note; the texts are matched as a multiset because
            // the two walks meet the notes in different orders.
            let mut tree_slots: Vec<(String, bool)> = from_tree.clone();
            let mut stream_slots: Vec<(String, bool)> = from_stream
                .iter()
                .map(|(text, _, owner)| (text.clone(), owner.is_leading()))
                .collect();
            tree_slots.sort();
            stream_slots.sort();
            assert_eq!(
                tree_slots, stream_slots,
                "{text}: the tree's slots and the stream's owners disagree"
            );
            // The source fact has to agree with the placement: a note the source gave a line of
            // its own always leads, and a note in the inline slot never had a line of its own.
            for (note, owns_line, owner) in &from_stream {
                assert!(
                    !*owns_line || owner.is_leading(),
                    "{text}: {note} had a line of its own but does not lead"
                );
                assert!(
                    !owner.is_inline() || !*owns_line,
                    "{text}: {note} takes the inline slot but had a line of its own"
                );
            }
            // The owner is the tree's own answer, so the fold puts every note back where the
            // tree had it - the whole document, decorations included.
            assert_eq!(fold(&events), tree, "{text}: the fold lost a note");
        }
    }

    #[test]
    fn a_malformed_stream_is_refused_rather_than_guessed() {
        // `from_events` is a public reader of a caller-supplied list, so the shapes no producer of
        // this crate can emit are refusals and not panics or silent repairs.
        use pyrs_event::{StreamEvent, StreamEventType};

        fn event(event_type: StreamEventType<'static>) -> StreamEvent<'static> {
            StreamEvent {
                event_type,
                line: 0,
                column: 0,
            }
        }
        let scalar = |text: &str, style| {
            event(StreamEventType::Scalar {
                value: alloc::borrow::Cow::Owned(text.into()),
                style,
                anchor: None,
                tag: None,
            })
        };
        let mapping = |anchor: bool| {
            event(StreamEventType::MappingStart {
                anchor: anchor.then(|| "a".to_string()),
                tag: None,
            })
        };
        let plain = |text: &str| scalar(text, ScalarStyle::Plain);
        let cases: Vec<(Vec<StreamEvent<'static>>, &str)> = vec![
            (vec![], "no document"),
            (vec![plain("1"), plain("2")], "a second root value"),
            (
                vec![event(StreamEventType::MappingEnd)],
                "closed with nothing open",
            ),
            (
                vec![
                    mapping(false),
                    event(StreamEventType::SequenceEnd),
                    event(StreamEventType::MappingEnd),
                ],
                "closed against the wrong kind",
            ),
            (
                vec![mapping(false), plain("k"), plain("v")],
                "left open at the end",
            ),
            (
                vec![scalar("x", ScalarStyle::Literal)],
                "a block style JSON never writes",
            ),
            (
                vec![event(StreamEventType::Alias { name: "a".into() })],
                "an alias",
            ),
            (
                vec![mapping(true), event(StreamEventType::MappingEnd)],
                "an anchored mapping",
            ),
        ];
        for (events, why) in cases {
            let outcome = from_events(&events);
            assert!(
                matches!(outcome, Err(ParseError::Syntax { .. })),
                "{why} was accepted as {outcome:?}"
            );
        }
        // A well-formed stream of scalars, mappings and sequences round-trips through the reader,
        // and a note in the middle of one is placed where its owner says it rides.
        let ok = vec![
            event(StreamEventType::StreamStart),
            event(StreamEventType::DocumentStart),
            mapping(false),
            plain("k"),
            plain("1"),
            event(StreamEventType::Comment {
                text: alloc::sync::Arc::from("note"),
                owns_line: true,
                owner: NoteOwner::PrecedingLeading,
            }),
            event(StreamEventType::MappingEnd),
            event(StreamEventType::DocumentEnd),
            event(StreamEventType::StreamEnd),
        ];
        let mut value = CustomNode::plain_scalar("1");
        value.push_leading_comment(pyrs_ast::ast::Comment {
            text: alloc::sync::Arc::from("note"),
            standalone: true,
        });
        assert_eq!(
            fold(&ok),
            CustomNode::plain_mapping(
                vec![(quoted_or_plain("k".into()), value,)]
                    .into_iter()
                    .collect()
            ),
        );
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
                            fold(&events),
                            node,
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
