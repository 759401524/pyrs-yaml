//! # pyrs-event
//!
//! The event vocabulary the document stream is described in: `StreamStart` / `StreamEnd`,
//! `DocumentStart` / `DocumentEnd`, scalar, mapping and sequence open/close, alias, and comment.
//! It sits beside `pyrs-ast` - the node model - and,
//! like it, is `no_std` + `alloc` and free of any format engine's own types.
//!
//! ## What this crate is, at this step
//!
//! The shape here is YAML's, moved verbatim out of `crates/pyrs-yaml-core/src/parser/stream.rs`.
//! That is deliberate: a neutral vocabulary has to be designed against more than one producer, and
//! the producers do not exist yet. The sequence this step belongs to is recorded in
//! `docs/dev/event-stream.md`:
//!
//! 1. **this crate**, holding the type, with the YAML adapter left where it is and re-exporting -
//!    no behaviour change, so the instruction-count gate proves the move cost nothing.
//! 2. a JSON producer, which is the first thing that says which fields are shared and which are a
//!    format's payload.
//! 3. a TOML producer, same question again.
//! 4. a replay property test - events fed back through one reader must rebuild the AST the direct
//!    parsers build - because that is the only test that can show the vocabulary is neutral rather
//!    than merely renamed.
//!
//! So a field like `anchor`, `tag` or `Alias` is not a claim that JSON has anchors; it is YAML's
//! grammar still visible, and step 2 is where it either generalises or moves into a payload.

#![no_std]

extern crate alloc;

use alloc::borrow::Cow;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use pyrs_ast::ast::{ScalarStyle, Tag};

/// Line and column information for a YAML stream event.
///
/// The scalar text borrows from the parsed source (`Cow<'a, str>`) and owns only what a decoder
/// had to produce — a double-quoted value with escapes arrives already decoded and owned, a plain
/// or block value is a slice of the input. `into_owned` is the escape hatch for an owner that
/// cannot carry the lifetime, which is what the Python boundary needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamEvent<'a> {
    /// The type of event that occurred.
    pub event_type: StreamEventType<'a>,
    /// The line number (0-indexed) where the event starts.
    pub line: usize,
    /// The column number (0-indexed) where the event starts.
    pub column: usize,
}

impl StreamEvent<'_> {
    /// Copy out of the source this event borrows, yielding an event that owns its text.
    pub fn into_owned(self) -> StreamEvent<'static> {
        StreamEvent {
            event_type: self.event_type.into_owned(),
            line: self.line,
            column: self.column,
        }
    }
}

/// The type of a streaming YAML event, representing a single occurrence
/// in the YAML document stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamEventType<'a> {
    /// The beginning of the YAML stream.
    StreamStart,
    /// The end of the YAML stream.
    StreamEnd,
    /// The beginning of a YAML document.
    DocumentStart,
    /// The end of a YAML document.
    DocumentEnd,
    /// A scalar value.
    Scalar {
        /// The scalar text, borrowed from the source unless a decode produced it.
        value: Cow<'a, str>,
        /// The scalar style (plain, quoted, literal, folded).
        style: ScalarStyle,
        /// The anchor name, if present.
        anchor: Option<String>,
        /// The YAML tag, if present.
        tag: Option<Tag>,
    },
    /// The beginning of a mapping node.
    MappingStart {
        /// The anchor name, if present.
        anchor: Option<String>,
        /// The YAML tag, if present.
        tag: Option<Tag>,
    },
    /// The end of a mapping node.
    MappingEnd,
    /// The beginning of a sequence node.
    SequenceStart {
        /// The anchor name, if present.
        anchor: Option<String>,
        /// The YAML tag, if present.
        tag: Option<Tag>,
    },
    /// The end of a sequence node.
    SequenceEnd,
    /// An alias reference.
    Alias {
        /// The alias name.
        name: String,
    },
    /// A comment line or inline comment.
    Comment {
        /// The comment text (without the `#` prefix and leading space).
        text: Arc<str>,
        /// Whether this is a standalone line comment (`true`) or an inline
        /// comment at the end of a line (`false`).
        standalone: bool,
    },
}

impl StreamEventType<'_> {
    /// Own every text this variant carries, dropping the borrow on the source.
    ///
    /// Only the scalar has anything to do: an anchor name is recovered from the source by the
    /// producer (granit hands out a numeric id, never the name) and a comment text is already
    /// shared through `Arc<str>`, so both are owned where they are built.
    pub fn into_owned(self) -> StreamEventType<'static> {
        match self {
            StreamEventType::Scalar {
                value,
                style,
                anchor,
                tag,
            } => StreamEventType::Scalar {
                value: Cow::Owned(value.into_owned()),
                style,
                anchor,
                tag,
            },
            StreamEventType::MappingStart { anchor, tag } => {
                StreamEventType::MappingStart { anchor, tag }
            }
            StreamEventType::SequenceStart { anchor, tag } => {
                StreamEventType::SequenceStart { anchor, tag }
            }
            StreamEventType::Alias { name } => StreamEventType::Alias { name },
            StreamEventType::Comment { text, standalone } => {
                StreamEventType::Comment { text, standalone }
            }
            StreamEventType::StreamStart => StreamEventType::StreamStart,
            StreamEventType::StreamEnd => StreamEventType::StreamEnd,
            StreamEventType::DocumentStart => StreamEventType::DocumentStart,
            StreamEventType::DocumentEnd => StreamEventType::DocumentEnd,
            StreamEventType::MappingEnd => StreamEventType::MappingEnd,
            StreamEventType::SequenceEnd => StreamEventType::SequenceEnd,
        }
    }
}

/// The line/column machinery a producer needs in order to report positions.
///
/// A tokenizer moves through byte offsets; the vocabulary reports the granit convention the YAML
/// engine already hands out - a 0-indexed line and a column counted in characters from the start of
/// that line. So each producer converts, and the conversion is the same arithmetic. One pass over
/// the source builds the table, which makes an event's position a binary search plus one char count
/// instead of a rescan from byte zero per event.
///
/// Lines break at `\n`, because that is what both engines' own diagnostics count: `err` in
/// `pyrs-json` and `err_at` in `pyrs-toml`, so an event and the error for the same byte never
/// disagree about where they sit.
pub struct LineTable<'a> {
    text: &'a str,
    starts: Vec<usize>,
}

impl<'a> LineTable<'a> {
    /// Index the line starts of `text`.
    pub fn new(text: &'a str) -> Self {
        let mut starts = Vec::with_capacity(8);
        starts.push(0usize);
        for (index, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                starts.push(index + 1);
            }
        }
        Self { text, starts }
    }

    /// The 0-indexed `(line, column)` pair of a byte offset, the column counted in characters.
    ///
    /// An offset at or past the end of the source is clamped, and one that falls inside a
    /// multi-byte character is snapped down to that character's start: a producer that stopped
    /// mid-code point reports a position rather than panicking.
    pub fn line_col(&self, pos: usize) -> (usize, usize) {
        let line = self
            .starts
            .partition_point(|&start| start <= pos)
            .saturating_sub(1);
        let start = self.starts[line];
        let cut = floor_char_boundary(self.text, pos.max(start));
        (line, self.text[start..cut].chars().count())
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

    #[test]
    fn a_line_table_reports_the_granit_convention_for_both_engine_dialects() {
        // Two lines, one of them multi-byte, and a trailing line with no break of its own.
        let text = "a = 1\nkéy = 2";
        let table = LineTable::new(text);
        assert_eq!(table.line_col(0), (0, 0));
        assert_eq!(table.line_col(2), (0, 2));
        // The second line starts just after the `\n`; `é` is one character, so a column counted
        // in bytes would say 6 here and every consumer would shift by one.
        let key = text.find('k').unwrap();
        assert_eq!(table.line_col(key), (1, 0));
        let equals = text.rfind('=').unwrap();
        assert_eq!(table.line_col(equals), (1, 4));
    }

    #[test]
    fn an_offset_at_or_past_the_end_is_a_position_not_a_panic() {
        let text = "a\né";
        let table = LineTable::new(text);
        let len = text.len();
        assert_eq!(table.line_col(len), table.line_col(len + 40));
        // Mid-code-point: `é` occupies two bytes, and the producer may stop at either.
        let accent = text.find('é').unwrap();
        assert_eq!(table.line_col(accent), (1, 0));
        assert_eq!(table.line_col(accent + 1), (1, 0));
    }
}
