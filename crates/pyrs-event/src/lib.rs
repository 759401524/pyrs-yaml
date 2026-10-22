//! # pyrs-event
//!
//! The event vocabulary the document stream is described in: `StreamStart` / `StreamEnd`,
//! `DocumentStart` / `DocumentEnd`, scalar, mapping and sequence open/close, alias, and comment.
//! It sits beside `pyrs-ast` - the node model - and,
//! like it, is `no_std` + `alloc` and free of any format engine's own types.
//!
//! ## What this crate is now
//!
//! The shape is YAML's, moved verbatim out of `crates/pyrs-yaml-core/src/parser/stream.rs`, and the
//! sequence that had to prove it was neutral rather than merely renamed is finished: `pyrs-json`
//! walks its own scanner into these events, `pyrs-toml` projects them from the intermediate
//! representation its AST route already builds, a fuzz target plus a deterministic seed corpus
//! replay both walks against the tree on every `cargo nextest`, and one Python-facing verb reads all
//! three formats. So the fields below are answered by two producers rather than asserted.
//!
//! What that measured: `anchor` and `Alias` stayed YAML's own — the other two never emit them, and
//! `tag` turned out to generalise, because TOML arrives with `!!timestamp` / `!time` and `!!int`,
//! exactly what its tree stores. `style` reports whatever a given engine remembers, which
//! is why a TOML literal and a basic string read alike. Positions are the producer's to carry: a
//! close event's offset exists in the stream and nowhere else, and a header's key precedes the
//! `MappingStart` it opens.
//!
//! The limit this crate was measured against is now closed: a note carries *two* facts instead of
//! one boolean read two ways. `Comment::owns_line` is the source fact — whether the source gave
//! the note a line of its own — and `Comment::owner` is the placement the tree files it under,
//! expressed as the node the note rides relative to the consumer's own build order. A consumer
//! that rebuilds a tree from a stream can therefore put every note back in the slot it came from
//! — `docs/dev/event-stream.md` carries the decision and the gate that holds it.

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

/// Which node a note is filed under, and in which of its two slots.
///
/// The tree files every note in one of two slots: the node's *leading* list, for a note that
/// precedes the node, and the node's *inline* slot, for a note that rides the node's own line.
/// The stream reports the same placement, but the consumer that rebuilds a tree sees the nodes
/// in the order the producer emitted them, so "precedes" and "rides" have to be said relative to
/// that order rather than relative to the source text.
///
/// Hence two axes: the direction and the slot.
///
/// - `Following*` — the note belongs to the node the consumer builds **next**.
/// - `Preceding*` — the note belongs to the node the consumer built **last**.
///
/// The direction is what removes the ambiguity the source text cannot: a note between a key and
/// its value is written *before* the value, but the tree files it under the value, and a note
/// after a value but before the next key is written *after* the value yet still belongs to the
/// next key. Only the producer knows which node it filed the note under, so only the producer
/// can answer, and it answers in the consumer's build order.
///
/// The slot is the tree's own two-slot model, so a consumer that rebuilds a tree puts the note
/// back where the tree had it — `pyrs_ast::ast::CustomNode` keeps that model unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NoteOwner {
    /// The leading list of the node that follows — the consumer builds it next.
    FollowingLeading,
    /// The inline slot of the node that follows — the consumer builds it next.
    FollowingInline,
    /// The leading list of the node that precedes — the consumer built it last.
    PrecedingLeading,
    /// The inline slot of the node that precedes — the consumer built it last.
    PrecedingInline,
}

impl NoteOwner {
    /// Whether this owner points at the node the consumer has not built yet.
    pub fn is_following(self) -> bool {
        matches!(
            self,
            NoteOwner::FollowingLeading | NoteOwner::FollowingInline
        )
    }

    /// Whether this owner points at the node the consumer built last.
    pub fn is_preceding(self) -> bool {
        !self.is_following()
    }

    /// Whether this owner names the node's leading list rather than its inline slot.
    pub fn is_leading(self) -> bool {
        matches!(
            self,
            NoteOwner::FollowingLeading | NoteOwner::PrecedingLeading
        )
    }

    /// Whether this owner names the node's inline slot rather than its leading list.
    pub fn is_inline(self) -> bool {
        !self.is_leading()
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
        /// Whether the source gave this note a line of its own.
        ///
        /// This is the *source fact*, and it is what the old single `standalone` flag was read as
        /// on the stream side: a note the source wrote on a line by itself (`true`) against one it
        /// wrote at the end of a line that carries something else (`false`).
        owns_line: bool,
        /// Which node the note is filed under, and in which of its two slots.
        ///
        /// This is the *placement*, and it is what the old single `standalone` flag was read as on
        /// the tree side: the tree files a note in the leading list of the node that follows it or
        /// in the inline slot of the node it rides. The two questions are not the same question —
        /// two same-line notes cannot share one inline slot, so the tree files them in the leading
        /// list instead — which is why one boolean could not carry both.
        owner: NoteOwner,
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
            StreamEventType::Comment {
                text,
                owns_line,
                owner,
            } => StreamEventType::Comment {
                text,
                owns_line,
                owner,
            },
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
    use alloc::vec;
    use proptest::prelude::*;

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

    /// The vocabulary has no constructor of its own, so a test builds the record directly.
    fn event<'a>(line: usize, column: usize, event_type: StreamEventType<'a>) -> StreamEvent<'a> {
        StreamEvent {
            event_type,
            line,
            column,
        }
    }

    /// One event of every variant, built once so the two tests below ask the same question of all
    /// eleven arms of `into_owned` rather than of whichever variants a producer happens to emit.
    fn one_of_every_variant<'a>() -> Vec<StreamEvent<'a>> {
        let tag = Tag::local("str");
        vec![
            event(0, 0, StreamEventType::StreamStart),
            event(0, 9, StreamEventType::StreamEnd),
            event(0, 0, StreamEventType::DocumentStart),
            event(0, 9, StreamEventType::DocumentEnd),
            event(
                0,
                3,
                StreamEventType::Scalar {
                    value: Cow::Borrowed("plain"),
                    style: ScalarStyle::Plain,
                    anchor: Some(String::from("a")),
                    tag: Some(tag.clone()),
                },
            ),
            event(
                1,
                0,
                StreamEventType::Scalar {
                    value: Cow::Owned(String::from("decoded")),
                    style: ScalarStyle::DoubleQuoted,
                    anchor: None,
                    tag: None,
                },
            ),
            event(
                1,
                2,
                StreamEventType::MappingStart {
                    anchor: Some(String::from("m")),
                    tag: Some(tag.clone()),
                },
            ),
            event(1, 8, StreamEventType::MappingEnd),
            event(
                2,
                2,
                StreamEventType::SequenceStart {
                    anchor: None,
                    tag: Some(tag),
                },
            ),
            event(2, 8, StreamEventType::SequenceEnd),
            event(
                3,
                0,
                StreamEventType::Alias {
                    name: String::from("m"),
                },
            ),
            event(
                4,
                0,
                StreamEventType::Comment {
                    text: Arc::from("note"),
                    owns_line: true,
                    owner: NoteOwner::FollowingLeading,
                },
            ),
        ]
    }

    #[test]
    fn owning_a_stream_keeps_every_field_of_every_variant() {
        // `into_owned` is an eleven-arm match, so the way it breaks is silently: an arm that maps to
        // a neighbouring variant, or one that rebuilds a variant without a field, still compiles.
        // Neither producer nor reader can see that; equality on the whole event can.
        let events = one_of_every_variant();
        for event in &events {
            assert_eq!(
                event,
                &event.clone(),
                "{:?} is not a faithful clone",
                event.event_type
            );
        }
        let owned: Vec<StreamEvent<'static>> = events
            .clone()
            .into_iter()
            .map(StreamEvent::into_owned)
            .collect();
        assert_eq!(events.len(), owned.len());
        for (before, after) in events.iter().zip(owned.iter()) {
            assert_eq!(before, after, "owning changed {:?}", before.event_type);
            assert_eq!(before.line, after.line);
            assert_eq!(before.column, after.column);
        }
    }

    #[test]
    fn a_borrowed_scalar_and_its_copy_out_of_the_source_are_the_same_event() {
        // The replay gates compare an event that borrowed the source against one that owns its text;
        // that comparison is only meaningful if `Cow` answers equal for the two. Pinned here rather
        // than assumed, because a vocabulary whose borrowed and owned forms disagreed would make a
        // passing replay test say nothing.
        let borrowed = event(
            0,
            3,
            StreamEventType::Scalar {
                value: Cow::Borrowed("plain"),
                style: ScalarStyle::Plain,
                anchor: None,
                tag: None,
            },
        );
        let owned = event(
            0,
            3,
            StreamEventType::Scalar {
                value: Cow::Owned(String::from("plain")),
                style: ScalarStyle::Plain,
                anchor: None,
                tag: None,
            },
        );
        assert_eq!(borrowed, owned);
        assert_eq!(borrowed.clone().into_owned(), owned);
    }

    #[test]
    fn owning_a_comment_moves_the_shared_text_rather_than_copying_it() {
        // A note's text is `Arc<str>` precisely so that detaching at the Python boundary costs a
        // reference-count bump; the claim is this pointer identity, and a change that copies here
        // would still pass every round-trip test above.
        let text: Arc<str> = Arc::from("note");
        let event = event(
            0,
            0,
            StreamEventType::Comment {
                text: Arc::clone(&text),
                owns_line: false,
                owner: NoteOwner::PrecedingInline,
            },
        );
        let owned = event.into_owned();
        let StreamEventType::Comment {
            text: owned_text,
            owns_line,
            owner,
        } = owned.event_type
        else {
            panic!("owning a comment stopped being a comment");
        };
        assert!(Arc::ptr_eq(&text, &owned_text));
        assert!(!owns_line);
        assert_eq!(owner, NoteOwner::PrecedingInline);
    }

    /// The four owners are four different claims, and a helper that answers one of them for the
    /// wrong owner is a helper that lies. Each axis is checked on its own: the direction and the
    /// slot are independent, so a mix-up in either has to be visible.
    #[test]
    fn an_owner_names_a_direction_and_a_slot_independently() {
        let owners = [
            NoteOwner::FollowingLeading,
            NoteOwner::FollowingInline,
            NoteOwner::PrecedingLeading,
            NoteOwner::PrecedingInline,
        ];
        for owner in owners {
            assert_eq!(owner.is_following(), owner.is_preceding() ^ true);
            assert_eq!(owner.is_leading(), owner.is_inline() ^ true);
        }
        assert!(NoteOwner::FollowingLeading.is_following());
        assert!(NoteOwner::FollowingLeading.is_leading());
        assert!(NoteOwner::FollowingInline.is_following());
        assert!(!NoteOwner::FollowingInline.is_leading());
        assert!(NoteOwner::PrecedingLeading.is_preceding());
        assert!(NoteOwner::PrecedingLeading.is_leading());
        assert!(NoteOwner::PrecedingInline.is_preceding());
        assert!(!NoteOwner::PrecedingInline.is_leading());
    }

    proptest! {
        /// Every character offset of an arbitrary document maps to that character's own line and
        /// column, a producer that stops inside a code point gets the character's start rather than
        /// a position before it, and the answer never runs backwards as the offset grows. The two
        /// hand-picked tables above pin the convention; this is the statement that holds for every
        /// line count and every mix of one-, two-, three- and four-byte characters.
        #[test]
        fn every_character_offset_is_the_position_of_its_own_character(
            lines in prop::collection::vec(
                prop::collection::vec(
                    prop::char::any().prop_filter("a line break belongs to the separator", |c| {
                        *c != '\u{a}'
                    }),
                    0..6,
                ),
                0..5,
            ),
        ) {
            let lines: Vec<String> = lines.into_iter().map(|l| l.into_iter().collect()).collect();
            let text = lines.join("\u{a}");
            let table = LineTable::new(&text);

            let mut offset = 0usize;
            let mut previous: Option<(usize, usize)> = None;
            for (line, content) in lines.iter().enumerate() {
                for (column, ch) in content.chars().enumerate() {
                    let position = table.line_col(offset);
                    prop_assert_eq!(position, (line, column));
                    for inner in 1..ch.len_utf8() {
                        prop_assert_eq!(table.line_col(offset + inner), (line, column));
                    }
                    if let Some(earlier) = previous {
                        prop_assert!(earlier <= position, "{earlier:?} then {position:?} is not monotone");
                    }
                    previous = Some(position);
                    offset += ch.len_utf8();
                }
                if line + 1 < lines.len() {
                    let end = table.line_col(offset);
                    prop_assert_eq!(end.0, line);
                    offset += 1;
                }
            }

            // The end of the source, and anything past it, clamp to one answer rather than
            // panicking: a producer that ran off the end still has to report somewhere.
            prop_assert_eq!(table.line_col(text.len()), table.line_col(text.len() + 64));
        }

        /// Owning an event is a no-op on everything but where its text lives, and it is idempotent
        /// — the boundary hands Python an owned stream, so the second `into_owned` is not a
        /// hypothetical. Generalises the eleven hand-built variants to arbitrary field contents.
        #[test]
        fn owning_an_event_changes_ownership_and_nothing_else(
            body in prop::collection::vec(prop::char::any(), 0..8),
            borrowed in any::<bool>(),
            positions in (any::<usize>(), any::<usize>()),
            quoted in any::<bool>(),
            anchored in any::<bool>(),
            owns_line in any::<bool>(),
            following in any::<bool>(),
            leading in any::<bool>(),
        ) {
            let text: String = body.into_iter().collect();
            let value = if borrowed {
                alloc::borrow::Cow::Borrowed(text.as_str())
            } else {
                alloc::borrow::Cow::Owned(text.clone())
            };
            let style = if quoted {
                ScalarStyle::DoubleQuoted
            } else {
                ScalarStyle::Plain
            };
            let anchor = anchored.then(|| text.clone());
            let owner = match (following, leading) {
                (true, true) => NoteOwner::FollowingLeading,
                (true, false) => NoteOwner::FollowingInline,
                (false, true) => NoteOwner::PrecedingLeading,
                (false, false) => NoteOwner::PrecedingInline,
            };

            let variants = alloc::vec![
                StreamEventType::Scalar {
                    value: value.clone(),
                    style,
                    anchor: anchor.clone(),
                    tag: None,
                },
                StreamEventType::MappingStart {
                    anchor,
                    tag: Some(Tag::local("str")),
                },
                StreamEventType::SequenceStart {
                    anchor: None,
                    tag: None,
                },
                StreamEventType::Alias { name: text.clone() },
                StreamEventType::Comment {
                    text: Arc::from(text.as_str()),
                    owns_line,
                    owner,
                },
                StreamEventType::StreamStart,
                StreamEventType::MappingEnd,
            ];

            for event_type in variants {
                let event = StreamEvent {
                    event_type,
                    line: positions.0 % 64,
                    column: positions.1 % 64,
                };
                prop_assert_eq!(&event, &event.clone());
                let owned = event.clone().into_owned();
                prop_assert_eq!(&event, &owned, "owning changed {:?}", event.event_type);
                prop_assert_eq!(&owned, &owned.clone().into_owned(), "owning twice differs");
                prop_assert_eq!(event.line, owned.line);
                prop_assert_eq!(event.column, owned.column);
            }
        }
    }
}
